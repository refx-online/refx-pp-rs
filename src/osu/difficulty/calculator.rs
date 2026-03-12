use crate::{
    model::mods::GameMods, 
    util::{
        difficulty::reverse_lerp,
    },
};

const DIFFICULTY_MULTIPLIER: f64 = 0.0675;

pub struct OsuRatingCalculator<'mods> {
    mods: &'mods GameMods,
    total_hits: u32,
    overall_difficulty: f64,
}

impl<'mods> OsuRatingCalculator<'mods> {
    pub const fn new(
        mods: &'mods GameMods,
        total_hits: u32,
        overall_difficulty: f64,
    ) -> Self {
        Self {
            mods,
            total_hits,
            overall_difficulty,
        }
    }
}

impl OsuRatingCalculator<'_> {
    pub fn compute_aim_rating(&self, aim_difficulty_value: f64) -> f64 {
        if self.mods.ap() {
            return 0.0;
        }

        let mut aim_rating = aim_difficulty_value.powf(0.62) * 0.0248;

        if self.mods.td() {
            aim_rating = aim_rating.powf(0.8);
        }

        if self.mods.mg() {
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.5);
            aim_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, self.overall_difficulty).powf(2.0) / 2500.0;

        aim_rating * rating_multiplier.cbrt()
    }

    pub fn compute_speed_rating(&self, speed_difficulty_value: f64) -> f64 {
        let mut speed_rating = Self::calculate_difficulty_rating(speed_difficulty_value);

        if self.mods.ap() {
            speed_rating *= 0.5;
        }

        if self.mods.mg() {
            // * Reduce speed rating because of the speed distance scaling, with maximum reduction being 0.7x
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.5);
            speed_rating *= 1.0 - magnetised_strength * 0.3;
        }

        speed_rating
    }

    pub fn compute_reading_rating(&self, reading_difficulty_value: f64) -> f64 {
        let mut reading_rating = Self::calculate_difficulty_rating(reading_difficulty_value);

        if self.mods.td() {
            reading_rating = reading_rating.powf(0.8);
        }

        if self.mods.rx() {
            // We have our nerf.
            // reading_rating *= 0.6;
        } else if self.mods.ap() {
            reading_rating *= 0.3;
        }

        if self.mods.mg() {
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.5);
            reading_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        rating_multiplier *= 0.75 + f64::max(0.0, self.overall_difficulty).powf(2.2) / 800.0;

        reading_rating * rating_multiplier.cbrt()
    }

    pub fn compute_flashlight_rating(&self, flashlight_difficulty_value: f64) -> f64 {
        if !self.mods.fl() {
            return 0.0;
        }

        let mut flashlight_rating = Self::calculate_difficulty_rating(flashlight_difficulty_value);

        if self.mods.td() {
            flashlight_rating = flashlight_rating.powf(0.8);
        }

        if self.mods.ap() {
            flashlight_rating *= 0.4;
        }

        if self.mods.mg() {
            let magnetised_strength = self.mods.attraction_strength().unwrap_or(0.5);
            flashlight_rating *= 1.0 - magnetised_strength;
        }

        if self.mods.df() {
            let deflate_initial_scale = self.mods.start_scale().unwrap_or(2.0);
            flashlight_rating *= reverse_lerp(deflate_initial_scale, 11.0, 1.0).clamp(0.1, 1.0);
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

    pub fn calculate_difficulty_rating(difficulty_value: f64) -> f64 {
        difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER
    }
}
