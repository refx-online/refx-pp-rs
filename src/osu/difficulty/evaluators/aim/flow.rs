use super::snap::SnapAimEvaluator;
use crate::{
    any::difficulty::object::IDifficultyObject, osu::difficulty::object::OsuDifficultyObject,
    util::difficulty::smoothstep,
};

pub struct FlowAimEvaluator;

impl FlowAimEvaluator {
    const VELOCITY_CHANGE_MULTIPLIER: f64 = 2.0;

    pub fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
        with_slider_travel_distance: bool,
        objects_radius: f64,
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

        let Some(osu_last_last_obj) = osu_curr_obj.previous(1, diff_objects) else {
            return 0.0;
        };

        let curr_distance = if with_slider_travel_distance {
            osu_curr_obj.lazy_jump_dist
        } else {
            osu_curr_obj.jump_dist
        };
        let prev_distance = if with_slider_travel_distance {
            osu_last_obj.lazy_jump_dist
        } else {
            osu_last_obj.jump_dist
        };

        let mut curr_velocity = curr_distance / osu_curr_obj.adjusted_delta_time;

        // * If the last object is a slider, then we extend the travel velocity through
        //   the slider into the current object.
        if osu_last_obj.base.is_slider() && with_slider_travel_distance {
            let slider_distance = osu_last_obj.lazy_travel_dist + osu_curr_obj.lazy_jump_dist;
            curr_velocity = curr_velocity.max(slider_distance / osu_curr_obj.adjusted_delta_time);
        }

        let prev_velocity = prev_distance / osu_last_obj.adjusted_delta_time;

        let mut flow_difficulty = curr_velocity;

        // * Apply high circle size bonus to the base velocity.
        // * We use reduced CS bonus here because the bonus was made for an evaluator with a different d/t scaling
        flow_difficulty *= osu_curr_obj.small_circle_bonus.sqrt();

        // * Rhythm changes are harder to flow
        let delta_diff = (osu_curr_obj
            .adjusted_delta_time
            .max(osu_last_obj.adjusted_delta_time)
            - osu_curr_obj
                .adjusted_delta_time
                .min(osu_last_obj.adjusted_delta_time))
            / 50.0;
        flow_difficulty *= 1.0 + delta_diff.powf(4.0).min(0.25);

        if let (Some(angle), Some(last_angle)) = (osu_curr_obj.angle, osu_last_obj.angle) {
            let angle_diff = (angle - last_angle).abs();
            let angle_diff_adjusted = (angle_diff / 2.0).sin() * 180.0;
            let angular_velocity = angle_diff_adjusted / (osu_curr_obj.adjusted_delta_time * 0.1);
            // * Low angular velocity flow (angles are consistent) is easier to follow than
            //   erratic flow
            flow_difficulty *= 0.8 + (angular_velocity / 270.0).sqrt();
        }

        // * If all three notes are overlapping - don't reward bonuses as you don't have
        //   to do additional movement
        let mut overlapped_notes_weight = 1.0;

        if osu_curr_obj.idx > 2 {
            let o1 = Self::calculate_overlap_factor(osu_curr_obj, osu_last_obj, objects_radius);
            let o2 =
                Self::calculate_overlap_factor(osu_curr_obj, osu_last_last_obj, objects_radius);
            let o3 =
                Self::calculate_overlap_factor(osu_last_obj, osu_last_last_obj, objects_radius);

            overlapped_notes_weight = 1.0 - o1 * o2 * o3;
        }

        if let Some(curr_angle) = osu_curr_obj.angle {
            // * Acute angles are also hard to flow
            // * We square root velocity to make acute angle switches in streams aren't
            //   having difficulty higher than snap
            flow_difficulty += curr_velocity.sqrt()
                * SnapAimEvaluator::calc_angle_acuteness(curr_angle)
                * overlapped_notes_weight;
        }

        if prev_velocity.max(curr_velocity) != 0.0 {
            if with_slider_travel_distance {
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
            let overlap_velocity_buff = (f64::from(OsuDifficultyObject::NORMALIZED_DIAMETER)
                * 1.25
                / osu_curr_obj
                    .adjusted_delta_time
                    .min(osu_last_obj.adjusted_delta_time))
            .min((prev_velocity - curr_velocity).abs());

            flow_difficulty += overlap_velocity_buff
                * dist_ratio
                * overlapped_notes_weight
                * Self::VELOCITY_CHANGE_MULTIPLIER;
        }

        if osu_curr_obj.base.is_slider() && with_slider_travel_distance {
            // * Include slider velocity to make velocity more consistent with snap
            flow_difficulty += osu_curr_obj.travel_dist / osu_curr_obj.travel_time;
        }

        // * Final velocity is being raised to a power because flow difficulty scales
        //   harder with both high distance and time, and we want to account for that
        flow_difficulty.powf(1.45)
    }

    fn calculate_overlap_factor(
        first: &OsuDifficultyObject,
        second: &OsuDifficultyObject,
        radius: f64,
    ) -> f64 {
        let object_radius = radius;

        let distance = f64::from((first.base.stacked_pos() - second.base.stacked_pos()).length());

        (1.0 - ((distance - object_radius).max(0.0) / object_radius).powi(2)).clamp(0.0, 1.0)
    }
}
