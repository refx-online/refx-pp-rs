use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::difficulty::object::OsuDifficultyObject,
    util::{
        difficulty::{norm, reverse_lerp, smootherstep},
        float_ext::FloatExt,
    },
};

pub struct ReadingEvaluator;

impl ReadingEvaluator {
    const DENSITY_DIFFICULTY_BASE: f64 = 2.5;
    const DENSITY_MULTIPLIER: f64 = 2.4;
    // * 1.5 circles distance between centers
    const DISTANCE_INFLUENCE_THRESHOLD: f64 = OsuDifficultyObject::NORMALIZED_DIAMETER as f64 * 1.5;
    const HIDDEN_MULTIPLIER: f64 = 0.28;
    const MAXIMUM_ANGLE_RELEVANCY_TIME: f64 = 200.0;
    // * 2 seconds
    const MINIMUM_ANGLE_RELEVANCY_TIME: f64 = 2000.0;
    const PREEMPT_BALANCING_FACTOR: f64 = 140_000.0;
    // * AR 9.66 in milliseconds
    const PREEMPT_STARTING_POINT: f64 = 500.0;
    // * 3 seconds
    const READING_WINDOW_SIZE: f64 = 3000.0;

    pub fn evaluate_diff_of(
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
        hidden: bool,
    ) -> f64 {
        if curr.base.is_spinner() || curr.idx == 0 {
            return 0.0;
        }

        let curr_obj = curr;
        let next_obj = curr.next(0, objects);

        let velocity = 1.0_f64.max(curr_obj.lazy_jump_dist / curr_obj.adjusted_delta_time); // * Only allow velocity to buff

        let current_visible_object_density =
            Self::retrieve_current_visible_object_density(curr_obj, objects);
        let past_object_difficulty_influence =
            Self::get_past_object_difficulty_influence(curr_obj, objects);

        let constant_angle_nerf_factor = Self::get_constant_angle_nerf_factor(curr_obj, objects);

        let note_density_difficulty = Self::calculate_density_difficulty(
            next_obj,
            velocity,
            constant_angle_nerf_factor,
            past_object_difficulty_influence,
            current_visible_object_density,
        );

        let hidden_difficulty = if hidden {
            Self::calculate_hidden_difficulty(
                curr_obj,
                objects,
                past_object_difficulty_influence,
                current_visible_object_density,
                velocity,
                constant_angle_nerf_factor,
            )
        } else {
            0.0
        };

        let preempt_difficulty = Self::calculate_preempt_difficulty(
            velocity,
            constant_angle_nerf_factor,
            curr_obj.clock_rate_adjusted_preempt,
        );

        norm(
            1.5,
            [
                preempt_difficulty,
                hidden_difficulty,
                note_density_difficulty,
            ],
        )
    }

    fn calculate_density_difficulty(
        next_obj: Option<&OsuDifficultyObject<'_>>,
        velocity: f64,
        constant_angle_nerf_factor: f64,
        past_object_difficulty_influence: f64,
        current_visible_object_density: f64,
    ) -> f64 {
        let mut future_object_difficulty_influence = current_visible_object_density.sqrt();

        if let Some(next) = next_obj {
            future_object_difficulty_influence *= smootherstep(
                next.lazy_jump_dist,
                15.0,
                Self::DISTANCE_INFLUENCE_THRESHOLD,
            );
        }

        let mut note_density_difficulty =
            (past_object_difficulty_influence + future_object_difficulty_influence).powf(1.7)
                * 0.4
                * constant_angle_nerf_factor
                * velocity;

        note_density_difficulty =
            0.0_f64.max(note_density_difficulty - Self::DENSITY_DIFFICULTY_BASE);

        note_density_difficulty.powf(0.45) * Self::DENSITY_MULTIPLIER
    }

    fn calculate_preempt_difficulty(
        velocity: f64,
        constant_angle_nerf_factor: f64,
        preempt: f64,
    ) -> f64 {
        // * Arbitrary curve for the base value preempt difficulty should have as
        //   approach rate increases.
        // * https://www.desmos.com/calculator/c175335a71
        let mut preempt_difficulty = ((Self::PREEMPT_STARTING_POINT - preempt
            + (preempt - Self::PREEMPT_STARTING_POINT).abs())
            / 2.0)
            .powf(2.5)
            / Self::PREEMPT_BALANCING_FACTOR;

        preempt_difficulty *= constant_angle_nerf_factor * velocity;

        preempt_difficulty
    }

    fn calculate_hidden_difficulty(
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
        past_object_difficulty_influence: f64,
        current_visible_object_density: f64,
        velocity: f64,
        constant_angle_nerf_factor: f64,
    ) -> f64 {
        // * Higher preempt means that time spent invisible is higher too, we want to
        //   reward that
        let preempt_factor = curr.preempt.powf(2.2) * 0.01;

        // * Account for both past and current densities
        let density_factor =
            (current_visible_object_density + past_object_difficulty_influence).powf(3.3) * 3.0;

        let mut hidden_difficulty =
            (preempt_factor + density_factor) * constant_angle_nerf_factor * velocity * 0.01;

        // * Apply a soft cap to general HD reading to account for partial memorization
        hidden_difficulty = hidden_difficulty.powf(0.4) * Self::HIDDEN_MULTIPLIER;

        // * Buff perfect stacks only if current note is completely invisible at the
        //   time you click the previous note.
        if let Some(previous_obj) = curr.previous(0, objects) {
            if FloatExt::eq(curr.lazy_jump_dist, 0.0)
                && FloatExt::eq(curr.opacity_at(previous_obj.start_time, true), 0.0)
                && previous_obj.start_time > curr.start_time - curr.preempt
            {
                // * Perfect stacks are harder the less time between notes
                hidden_difficulty +=
                    Self::HIDDEN_MULTIPLIER * 2500.0 / curr.adjusted_delta_time.powf(1.5);
            }
        }

        hidden_difficulty
    }

    fn get_past_object_difficulty_influence(
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let mut past_object_difficulty_influence = 0.0;

        for i in 0..curr.idx {
            let Some(loop_obj) = curr.previous(i, objects) else {
                break;
            };

            let time_diff = curr.start_time - loop_obj.start_time;

            if time_diff > Self::READING_WINDOW_SIZE
                || loop_obj.start_time < curr.start_time - curr.preempt
            {
                // * Current object not visible at the time object needs to be clicked
                break;
            }

            let mut loop_difficulty = curr.opacity_at(loop_obj.base.start_time, false);

            // * When aiming an object small distances mean previous objects may be cheesed,
            //   so it doesn't matter whether they were arranged confusingly.
            loop_difficulty *= smootherstep(
                loop_obj.lazy_jump_dist,
                15.0,
                Self::DISTANCE_INFLUENCE_THRESHOLD,
            );
            loop_difficulty *= Self::get_time_nerf_factor(time_diff);

            past_object_difficulty_influence += loop_difficulty;
        }

        past_object_difficulty_influence
    }

    fn retrieve_current_visible_object_density(
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let mut visible_object_count = 0.0;
        let mut i = 0;

        while let Some(hit_object) = curr.next(i, objects) {
            let time_diff = hit_object.start_time - curr.start_time;

            if time_diff > Self::READING_WINDOW_SIZE
                || curr.start_time < hit_object.start_time - hit_object.preempt
            {
                // * Current object not visible at the time object needs to be clicked
                break;
            }

            let time_nerf_factor = Self::get_time_nerf_factor(time_diff)
                * hit_object.opacity_at(curr.base.start_time, false);
            visible_object_count += time_nerf_factor;

            i += 1;
        }

        visible_object_count
    }

    fn get_constant_angle_nerf_factor(
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let mut constant_angle_count = 0.0;
        let mut index = 0;
        let mut current_time_gap = 0.0;

        while current_time_gap < Self::MINIMUM_ANGLE_RELEVANCY_TIME {
            let Some(loop_obj) = curr.previous(index, objects) else {
                break;
            };

            // * Account less for objects that are close to the time limit.
            let long_interval_factor = 1.0
                - reverse_lerp(
                    loop_obj.adjusted_delta_time,
                    Self::MAXIMUM_ANGLE_RELEVANCY_TIME,
                    Self::MINIMUM_ANGLE_RELEVANCY_TIME,
                );

            if let (Some(loop_angle), Some(curr_angle)) = (loop_obj.angle, curr.angle) {
                let angle_difference = (curr_angle - loop_angle).abs();

                let stack_factor = smootherstep(
                    loop_obj.lazy_jump_dist,
                    0.0,
                    f64::from(OsuDifficultyObject::NORMALIZED_RADIUS),
                );

                let radians_30 = 30.0_f64.to_radians();
                constant_angle_count += (3.0 * radians_30.min(angle_difference * stack_factor))
                    .cos()
                    * long_interval_factor;
            }

            current_time_gap = curr.start_time - loop_obj.start_time;
            index += 1;
        }

        (2.0 / constant_angle_count).clamp(0.2, 1.0)
    }

    fn get_time_nerf_factor(delta_time: f64) -> f64 {
        (2.0 - delta_time / (Self::READING_WINDOW_SIZE / 2.0)).clamp(0.0, 1.0)
    }
}
