use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::difficulty::object::OsuDifficultyObject,
    util::difficulty::{bpm_to_milliseconds, milliseconds_to_bpm},
};

pub struct SpeedEvaluator;

impl SpeedEvaluator {
    // 200 BPM 1/4th
    const MIN_SPEED_BONUS: f64 = 200.0;
    const SPEED_BALANCING_FACTOR: f64 = 40.0;

    pub fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
        hit_window: f64,
    ) -> f64 {
        if curr.base.is_spinner() {
            return 0.0;
        }

        // * derive strainTime for calculation
        let osu_curr_obj = curr;
        let osu_next_obj = curr.next(0, diff_objects);

        let mut strain_time = osu_curr_obj.adjusted_delta_time;
        // Note: Technically `osu_next_obj` is never `None` but instead the
        // default value. This could maybe invalidate the `get_doubletapness`
        // result.
        let doubletapness = 1.0 - osu_curr_obj.get_doubletapness(osu_next_obj, hit_window);

        // * Cap deltatime to the OD 300 hitwindow.
        // * 0.93 is derived from making sure 260bpm OD8 streams aren't nerfed harshly,
        //   whilst 0.92 limits the effect of the cap.
        strain_time /= ((strain_time / hit_window) / 0.93).clamp(0.92, 1.0);

        // * speedBonus will be 0.0 for BPM < 200
        let speed_bonus = if milliseconds_to_bpm(strain_time, None) > Self::MIN_SPEED_BONUS {
            // * Add additional scaling bonus for streams/bursts higher than 200bpm
            let base = (bpm_to_milliseconds(Self::MIN_SPEED_BONUS, None) - strain_time)
                / Self::SPEED_BALANCING_FACTOR;

            0.75 * base.powf(2.0)
        } else {
            0.0
        };

        // * Base difficulty with all bonuses
        let mut difficulty = (1.0 + speed_bonus) * 1000.0 / strain_time;

        difficulty *= Self::high_bpm_bonus(osu_curr_obj.adjusted_delta_time);

        // * Apply penalty if there's doubletappable doubles
        difficulty * doubletapness
    }

    fn high_bpm_bonus(ms: f64) -> f64 {
        1.0 / (1.0 - 0.3_f64.powf(ms / 1000.0))
    }
}
