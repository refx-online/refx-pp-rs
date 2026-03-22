use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::difficulty::object::OsuDifficultyObject,
    util::difficulty::{milliseconds_to_bpm, reverse_lerp, smootherstep, smoothstep},
};

pub struct SnapAimEvaluator;

impl SnapAimEvaluator {
    const ACUTE_ANGLE_MULTIPLIER: f64 = 2.41;
    const MAXIMUM_REPETITION_NERF: f64 = 0.15;
    const MAXIMUM_VECTOR_INFLUENCE: f64 = 0.5;
    const SLIDER_MULTIPLIER: f64 = 1.5;
    const VELOCITY_CHANGE_MULTIPLIER: f64 = 0.9;
    // * WARNING: Increasing this multiplier beyond 1.02 reduces difficulty as
    //   distance increases. Refer to the desmos link above the wiggle bonus
    //   calculation
    const WIDE_ANGLE_MULTIPLIER: f64 = 1.05;
    const WIGGLE_MULTIPLIER: f64 = 1.02;

    #[allow(clippy::too_many_lines)]
    pub fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
        with_slider_travel_distance: bool,
    ) -> f64 {
        if curr.base.is_spinner() || curr.idx <= 1 {
            return 0.0;
        }

        let osu_curr_obj = curr;

        let Some(osu_last_obj) = osu_curr_obj.previous(0, diff_objects) else {
            return 0.0;
        };

        if osu_last_obj.base.is_spinner() {
            return 0.0;
        }

        let osu_last_2_obj = osu_curr_obj.previous(2, diff_objects);

        #[expect(clippy::items_after_statements, reason = "staying in-sync with lazer")]
        const RADIUS: i32 = OsuDifficultyObject::NORMALIZED_RADIUS;
        #[expect(clippy::items_after_statements, reason = "staying in-sync with lazer")]
        const DIAMETER: i32 = OsuDifficultyObject::NORMALIZED_DIAMETER;

        // * Calculate the velocity to the current hitobject, which starts with a base
        //   distance / time assuming the last object is a hitcircle.
        let curr_distance = if with_slider_travel_distance {
            osu_curr_obj.lazy_jump_dist
        } else {
            osu_curr_obj.jump_dist
        };
        let mut curr_velocity = curr_distance / osu_curr_obj.adjusted_delta_time;

        // * But if the last object is a slider, then we extend the travel velocity
        //   through the slider into the current object.
        if osu_last_obj.base.is_slider() && with_slider_travel_distance {
            let slider_distance = osu_last_obj.lazy_travel_dist + osu_curr_obj.lazy_jump_dist;
            curr_velocity = curr_velocity.max(slider_distance / osu_curr_obj.adjusted_delta_time);
        }

        let prev_distance = if with_slider_travel_distance {
            osu_last_obj.lazy_jump_dist
        } else {
            osu_last_obj.jump_dist
        };
        let prev_velocity = prev_distance / osu_last_obj.adjusted_delta_time;

        let mut wide_angle_bonus = 0.0;
        let mut acute_angle_bonus = 0.0;
        let mut slider_bonus = 0.0;
        let mut velocity_change_bonus = 0.0;
        let mut wiggle_bonus = 0.0;

        let mut aim_strain = curr_velocity;

        if let (Some(curr_angle), Some(last_angle)) = (osu_curr_obj.angle, osu_last_obj.angle) {
            // * Rewarding angles, take the smaller velocity as base.
            let angle_bonus = curr_velocity.min(prev_velocity);

            // * If rhythms are the same.
            if osu_curr_obj
                .adjusted_delta_time
                .max(osu_last_obj.adjusted_delta_time)
                < 1.25
                    * osu_curr_obj
                        .adjusted_delta_time
                        .min(osu_last_obj.adjusted_delta_time)
            {
                acute_angle_bonus = Self::calc_acute_angle_bonus(curr_angle);

                // * Penalize angle repetition.
                acute_angle_bonus *= 0.08
                    + 0.92
                        * (1.0
                            - acute_angle_bonus
                                .min(Self::calc_acute_angle_bonus(last_angle).powi(3)));

                // * Apply acute angle bonus for BPM above 300 1/2 and distance more than one
                //   diameter
                acute_angle_bonus *= angle_bonus
                    * smootherstep(
                        milliseconds_to_bpm(osu_curr_obj.adjusted_delta_time, Some(2)),
                        300.0,
                        400.0,
                    )
                    * smootherstep(curr_distance, 0.0, f64::from(DIAMETER) * 2.0);
            }

            wide_angle_bonus = Self::calc_wide_angle_bonus(curr_angle);

            // * Penalize angle repetition.
            wide_angle_bonus *= 0.25
                + 0.75
                    * (1.0 - wide_angle_bonus.min(Self::calc_wide_angle_bonus(last_angle).powi(3)));

            wide_angle_bonus *= angle_bonus;

            // * Apply wiggle bonus for jumps that are [radius, 3*diameter] in distance,
            //   with < 110 angle
            // * https://www.desmos.com/calculator/dp0v0nvowc
            wiggle_bonus = angle_bonus
                * smootherstep(curr_distance, f64::from(RADIUS), f64::from(DIAMETER))
                * reverse_lerp(
                    curr_distance,
                    f64::from(DIAMETER) * 3.0,
                    f64::from(DIAMETER),
                )
                .powf(1.8)
                * smootherstep(curr_angle, 110_f64.to_radians(), 60_f64.to_radians())
                * smootherstep(prev_distance, f64::from(RADIUS), f64::from(DIAMETER))
                * reverse_lerp(
                    prev_distance,
                    f64::from(DIAMETER) * 3.0,
                    f64::from(DIAMETER),
                )
                .powf(1.8)
                * smootherstep(last_angle, 110_f64.to_radians(), 60_f64.to_radians());

            if let Some(osu_last_2_obj) = osu_last_2_obj {
                // * If objects just go back and forth through a middle point - don't give as
                //   much wide bonus
                // * Use Previous(2) and Previous(0) because angles calculation is done
                //   prevprev-prev-curr,
                // * so any object's angle's center point is always the previous object
                let distance =
                    (osu_last_2_obj.base.stacked_pos() - osu_last_obj.base.stacked_pos()).length();

                if distance < 1.0 {
                    wide_angle_bonus *= 1.0 - 0.55 * (1.0 - f64::from(distance));
                }
            }
        }

        if prev_velocity.max(curr_velocity) != 0.0 {
            if with_slider_travel_distance {
                // * We want to use just the object jump without slider velocity when awarding differences
                curr_velocity = curr_distance / osu_curr_obj.adjusted_delta_time;
            }

            // * Scale with ratio of difference compared to 0.5 * max dist.
            let dist_ratio = smoothstep(
                (prev_velocity - curr_velocity).abs() / prev_velocity.max(curr_velocity),
                0.0,
                1.0,
            );

            // * Reward for % distance up to 125 / strainTime for overlaps where velocity is
            //   still changing.
            let overlap_velocity_buff = (f64::from(DIAMETER) * 1.25
                / osu_curr_obj
                    .adjusted_delta_time
                    .min(osu_last_obj.adjusted_delta_time))
            .min((prev_velocity - curr_velocity).abs());

            velocity_change_bonus = overlap_velocity_buff * dist_ratio;

            // * Penalize for rhythm changes.
            velocity_change_bonus *= (osu_curr_obj
                .adjusted_delta_time
                .min(osu_last_obj.adjusted_delta_time)
                / osu_curr_obj
                    .adjusted_delta_time
                    .max(osu_last_obj.adjusted_delta_time))
            .powi(2);
        }

        if osu_curr_obj.base.is_slider() {
            // * Reward sliders based on velocity.
            slider_bonus = osu_curr_obj.travel_dist / osu_curr_obj.travel_time;
        }

        // * Penalize angle repetition.
        aim_strain *= Self::vector_angle_repetition(osu_curr_obj, osu_last_obj, diff_objects);

        aim_strain += wiggle_bonus * Self::WIGGLE_MULTIPLIER;
        aim_strain += velocity_change_bonus * Self::VELOCITY_CHANGE_MULTIPLIER;

        // * Add in acute angle bonus or wide angle bonus, whichever is larger.
        aim_strain += (acute_angle_bonus * Self::ACUTE_ANGLE_MULTIPLIER)
            .max(wide_angle_bonus * Self::WIDE_ANGLE_MULTIPLIER);

        // * Add in additional slider velocity bonus.
        if with_slider_travel_distance {
            aim_strain += if slider_bonus < 1.0 {
                slider_bonus
            } else {
                slider_bonus.powf(0.75)
            } * Self::SLIDER_MULTIPLIER;
        }

        // * Apply high circle size bonus
        aim_strain *= osu_curr_obj.small_circle_bonus;

        aim_strain *= Self::high_bpm_bonus(
            osu_curr_obj.adjusted_delta_time,
            osu_curr_obj.lazy_jump_dist,
        );

        aim_strain
    }

    // * We decrease strain for distances <radius to fix cases where doubles with no
    //   aim requirement
    // * have their strain buffed incredibly high due to the delta time.
    // * These objects do not require any movement, so it does not make sense to
    //   award them.
    fn high_bpm_bonus(ms: f64, distance: f64) -> f64 {
        1.0 / (1.0 - 0.03_f64.powf((ms / 1000.0).powf(0.65)))
            * smootherstep(
                distance,
                0.0,
                f64::from(OsuDifficultyObject::NORMALIZED_RADIUS),
            )
    }

    fn vector_angle_repetition<'a>(
        current: &'a OsuDifficultyObject<'a>,
        previous: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
    ) -> f64 {
        if current.angle.is_none() || previous.angle.is_none() {
            return 1.0;
        }

        #[expect(clippy::items_after_statements, reason = "staying in-sync with lazer")]
        const NOTE_LIMIT: usize = 6;

        let mut constant_angle_count: f64 = 0.0;

        for index in 0..NOTE_LIMIT {
            let Some(loop_obj) = current.previous(index, diff_objects) else {
                break;
            };

            // * Only consider vectors in the same jump section, stopping to change rhythm
            //   ruins momentum
            if current
                .adjusted_delta_time
                .max(loop_obj.adjusted_delta_time)
                > 1.1
                    * current
                        .adjusted_delta_time
                        .min(loop_obj.adjusted_delta_time)
            {
                break;
            }

            if let (Some(loop_normalized_angle), Some(curr_normalized_angle)) =
                (loop_obj.normalized_vec_angle, current.normalized_vec_angle)
            {
                let angle_difference = (curr_normalized_angle - loop_normalized_angle).abs();
                // * Refer to this desmos for tuning, constants need to be precise so that
                //   values stay within the range of 0 and 1.
                // * https://www.desmos.com/calculator/a8jesv5sv2
                constant_angle_count += (8.0 * angle_difference.min(11.25_f64.to_radians())).cos();
            }
        }

        let vector_repetition = (0.5 / constant_angle_count).min(1.0).powi(2);

        let stack_factor = smootherstep(
            current.lazy_jump_dist,
            0.0,
            f64::from(OsuDifficultyObject::NORMALIZED_DIAMETER),
        );

        let curr_angle = current.angle.unwrap();
        let last_angle = previous.angle.unwrap();

        let angle_difference_adjusted =
            (2.0 * ((curr_angle - last_angle).abs() * stack_factor).min(45_f64.to_radians())).cos();

        let base_nerf = 1.0
            - Self::MAXIMUM_REPETITION_NERF
                * Self::calc_acute_angle_bonus(last_angle)
                * angle_difference_adjusted;

        (base_nerf
            + (1.0 - base_nerf) * vector_repetition * Self::MAXIMUM_VECTOR_INFLUENCE * stack_factor)
            .powi(2)
    }

    const fn calc_wide_angle_bonus(angle: f64) -> f64 {
        smoothstep(angle, 40_f64.to_radians(), 140_f64.to_radians())
    }

    pub const fn calc_acute_angle_bonus(angle: f64) -> f64 {
        smoothstep(angle, 140_f64.to_radians(), 40_f64.to_radians())
    }
}
