use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::difficulty::object::OsuDifficultyObject,
    util::difficulty::smootherstep,
};

pub struct AgilityEvaluator;

impl AgilityEvaluator {
    const DISTANCE_CAP: f64 = OsuDifficultyObject::NORMALIZED_DIAMETER as f64 * 1.25;

    pub fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
    ) -> f64 {
        if curr.base.is_spinner() {
            return 0.0;
        }

        let travel_distance = curr
            .previous(0, diff_objects)
            .map_or(0.0, |prev| prev.lazy_travel_dist);

        #[expect(clippy::items_after_statements, reason = "staying in-sync with lazer")]
        const RADIUS: i32 = OsuDifficultyObject::NORMALIZED_RADIUS;

        let distance = travel_distance + curr.lazy_jump_dist;
        
        let distance_scaled = distance.min(Self::DISTANCE_CAP) / Self::DISTANCE_CAP;
        let mut strain = distance_scaled * 1000.0 / curr.adjusted_delta_time;
        
        strain *= Self::high_bpm_bonus(curr.adjusted_delta_time);
        
        strain * smootherstep(distance, 0.0, RADIUS as f64)
    }
    
    fn high_bpm_bonus(ms: f64) -> f64 {
        1.0 / (1.0 - 0.3_f64.powf((ms / 1000.0).powf(0.9)))
    }
}
