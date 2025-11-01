use crate::model::mods::GameMods;

const DIFFICULTY_MULTIPLIER: f64 = 0.0675;

pub struct OsuRatingCalculator<'mods> {
    mods: &'mods GameMods,
    total_hits: u32,
    approach_rate: f64,
    overall_difficulty: f64,
}

impl<'mods> OsuRatingCalculator<'mods> {
    pub const fn new(
        mods: &'mods GameMods,
        total_hits: u32,
        approach_rate: f64,
        overall_difficulty: f64,
    ) -> Self {
        Self {
            mods,
            total_hits,
            approach_rate,
            overall_difficulty,
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

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, self.total_hits as f64 / 2000.0)
            + if self.total_hits > 2000 {
                f64::log10(self.total_hits as f64 / 2000.0) * 0.5
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

        rating_multiplier *= 1.0 + ar_factor * approach_rate_length_bonus;

        if self.mods.hd() {
            // * We want to give more reward for lower AR when it comes to aim and HD. This nerfs high AR and buffs lower AR.
            rating_multiplier *= 1.0 + 0.04 * (12.0 - self.approach_rate);
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

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, self.total_hits as f64 / 2000.0)
            + if self.total_hits > 2000 {
                f64::log10(self.total_hits as f64 / 2000.0) * 0.5
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
            // * We want to give more reward for lower AR when it comes to aim and HD. This nerfs high AR and buffs lower AR.
            rating_multiplier *= 1.0 + 0.04 * (12.0 - self.approach_rate);
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
        rating_multiplier *= 0.7 + 0.1 * f64::min(1.0, self.total_hits as f64 / 200.0)
            + if self.total_hits > 200 {
                0.2 * f64::min(1.0, (self.total_hits - 200) as f64 / 200.0)
            } else {
                0.0
            };

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, self.overall_difficulty).powf(2.0) / 2500.0;

        flashlight_rating * rating_multiplier.sqrt()
    }
}
