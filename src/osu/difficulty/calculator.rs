use crate::{
    model::mods::GameMods, 
    util::{
        difficulty::reverse_lerp,
        float_ext::FloatExt,
    },
};

const DIFFICULTY_MULTIPLIER: f64 = 0.0675;

pub struct OsuRatingCalculator<'mods> {
    mods: &'mods GameMods,
    total_hits: u32,
    approach_rate: f64,
    overall_difficulty: f64,
    mechanical_difficulty_rating: f64,
    slider_factor: f64,
}

impl<'mods> OsuRatingCalculator<'mods> {
    pub const fn new(
        mods: &'mods GameMods,
        total_hits: u32,
        approach_rate: f64,
        overall_difficulty: f64,
        mechanical_difficulty_rating: f64,
        slider_factor: f64,
    ) -> Self {
        Self {
            mods,
            total_hits,
            approach_rate,
            overall_difficulty,
            mechanical_difficulty_rating,
            slider_factor,
        }
    }
}

impl OsuRatingCalculator<'_> {
    pub fn compute_aim_rating(&self, aim_difficulty_value: f64) -> f64 {
        if self.mods.ap() {
            return 0.0;
        }

        let mut aim_rating = f64::sqrt(aim_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if self.mods.td() {
            aim_rating = aim_rating.powf(0.8);
        }

        if self.mods.mg() {
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.0);
            aim_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, f64::from(self.total_hits) / 2000.0)
            + if self.total_hits > 2000 {
                f64::log10(f64::from(self.total_hits) / 2000.0) * 0.5
            } else {
                0.0
            };

        let ar_factor = if self.approach_rate > 10.33 {
            0.3 * (self.approach_rate - 10.33)
        } else if self.approach_rate < 8.0 {
            0.05 * (8.0 - self.approach_rate)
        } else {
            0.0
        };

        rating_multiplier *= 1.0 + ar_factor * approach_rate_length_bonus; // * Buff for longer maps with high AR.

        if self.mods.hd() {
            let visibility_factor = self.calculate_aim_visibility_factor(self.approach_rate);
            rating_multiplier += Self::calculate_visibility_bonus(
                self.mods.clone(),
                ar_factor, 
                Some(visibility_factor),
                Some(self.slider_factor),
            );
        }

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, self.overall_difficulty).powf(2.0) / 2500.0;

        aim_rating * rating_multiplier.cbrt()
    }

    pub fn compute_speed_rating(&self, speed_difficulty_value: f64) -> f64 {
        let mut speed_rating = f64::sqrt(speed_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if self.mods.ap() {
            speed_rating *= 0.5;
        }

        if self.mods.mg() {
            // * Reduce speed rating because of the speed distance scaling, with maximum reduction being 0.7x
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.0);
            speed_rating *= 1.0 - magnetised_strength * 0.3;
        }

        let mut rating_multiplier = 1.0;

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, f64::from(self.total_hits) / 2000.0)
            + if self.total_hits > 2000 {
                f64::log10(f64::from(self.total_hits) / 2000.0) * 0.5
            } else {
                0.0
            };

        let ar_factor = if self.mods.ap() {
            0.0
        } else if self.approach_rate > 10.33 {
            0.3 * (self.approach_rate - 10.33)
        } else {
            0.0
        };

        rating_multiplier *= 1.0 + ar_factor * approach_rate_length_bonus;

        if self.mods.hd() {
            let visibility_factor = self.calculate_speed_visibility_factor(self.approach_rate);
            rating_multiplier += Self::calculate_visibility_bonus(
                self.mods.clone(),
                ar_factor, 
                Some(visibility_factor),
                Some(self.slider_factor),
            );
        }

        rating_multiplier *= 0.95 + f64::max(0.0, self.overall_difficulty).powf(2.0) / 750.0;

        speed_rating * rating_multiplier.cbrt()
    }

    pub fn compute_flashlight_rating(&self, flashlight_difficulty_value: f64) -> f64 {
        if !self.mods.fl() {
            return 0.0;
        }

        let mut flashlight_rating = f64::sqrt(flashlight_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if self.mods.td() {
            flashlight_rating = flashlight_rating.powf(0.8);
        }

        if self.mods.ap() {
            flashlight_rating *= 0.4;
        }

        if self.mods.mg() {
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.0);
            flashlight_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        // * Account for shorter maps having a higher ratio of 0 combo/100 combo flashlight radius.
        rating_multiplier *= 0.7 + 0.1 * f64::min(1.0, f64::from(self.total_hits) / 200.0)
            + if self.total_hits > 200 {
                0.2 * f64::min(1.0, f64::from(self.total_hits - 200) / 200.0)
            } else {
                0.0
            };

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, self.overall_difficulty).powf(2.0) / 2500.0;

        flashlight_rating * rating_multiplier.sqrt()
    }

    fn calculate_aim_visibility_factor(&self, approach_rate: f64) -> f64 {
        const AR_FACTOR_END_POINT: f64 = 11.5;

        let mechanical_difficulty_factor =
            reverse_lerp(self.mechanical_difficulty_rating, 5.0, 10.0);
        let ar_factor_starting_point = f64::lerp(9.0, 10.33, mechanical_difficulty_factor);

        reverse_lerp(approach_rate, AR_FACTOR_END_POINT, ar_factor_starting_point)
    }

    fn calculate_speed_visibility_factor(&self, approach_rate: f64) -> f64 {
        const AR_FACTOR_END_POINT: f64 = 11.5;

        let mechanical_difficulty_factor =
            reverse_lerp(self.mechanical_difficulty_rating, 5.0, 10.0);
        let ar_factor_starting_point = f64::lerp(10.0, 10.33, mechanical_difficulty_factor);

        reverse_lerp(approach_rate, AR_FACTOR_END_POINT, ar_factor_starting_point)
    }
    
    /// Calculates a visibility bonus that is applicable to Hidden and Traceable.
    pub fn calculate_visibility_bonus(
        mods: GameMods,
        approach_rate: f64, 
        visibility_factor: Option<f64>, 
        slider_factor: Option<f64>,
    ) -> f64 {
        let visibility_factor = visibility_factor.unwrap_or(1.0);
        let slider_factor = slider_factor.unwrap_or(1.0);

        // * NOTE: TC's effect is only noticeable in performance calculations until lazer mods are accounted for server-side.
        let is_always_partially_visible = mods.hd() && mods.only_fade_approach_circles().is_some()
            || mods.tc();

        // * Start from normal curve, rewarding lower AR up to AR7
        // * TC forcefully requires a lower reading bonus for now as it's post-applied in PP which makes it multiplicative with the regular AR bonuses
        // * This means it has an advantage over HD, so we decrease the multiplier to compensate
        // * This should be removed once we're able to apply TC bonuses in SR (depends on real-time difficulty calculations being possible)
        let mut reading_bonus = if is_always_partially_visible { 0.025 } else { 0.04 } * (12.0 - approach_rate.max(7.0));

        reading_bonus *= visibility_factor;

        // * We want to reward slideraim on low AR less
        let slider_visibility_factor = slider_factor.powf(3.0);

        // * For AR up to 0 - reduce reward for very low ARs when object is visible
        if approach_rate < 7.0 {
            reading_bonus += if is_always_partially_visible { 0.02 } else { 0.045 }
                * (7.0 - approach_rate.max(0.0))
                * slider_visibility_factor;
        }

        // * Starting from AR0 - cap values so they won't grow to infinity
        if approach_rate < 0.0 {
            reading_bonus += if is_always_partially_visible { 0.01 } else { 0.1 }
                * (1.0 - 1.5f64.powf(approach_rate))
                * slider_visibility_factor;
        }

        reading_bonus
    }

    pub fn calculate_difficulty_rating(difficulty_value: f64) -> f64 {
        difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER
    }

}
