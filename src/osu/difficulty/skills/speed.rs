use super::strain::OsuHarmonicSkill;
use crate::{
    any::difficulty::skills::strain_decay,
    osu::difficulty::{
        evaluators::{RhythmEvaluator, SpeedEvaluator},
        object::OsuDifficultyObject,
        skills::strain::harmonic_difficulty_value,
    },
};

define_skill! {
    pub struct Speed: HarmonicSkill => [OsuDifficultyObject<'a>][OsuDifficultyObject<'a>] {
        current_strain: f64 = 0.0,
        hit_window: f64,
        slider_strains: Vec<f64> = Vec::with_capacity(64),
    }

    pub fn new(hit_window: f64) -> Self {
        Self {
            current_strain: 0.0,
            hit_window: hit_window,
            slider_strains: Vec::with_capacity(64),
        }
    }
}

impl Speed {
    const SKILL_MULTIPLIER: f64 = 1.16;
    const STRAIN_DECAY_BASE: f64 = 0.3;

    fn strain_value_at(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let decay = strain_decay(curr.adjusted_delta_time, Self::STRAIN_DECAY_BASE);

        self.current_strain *= decay;
        self.current_strain += SpeedEvaluator::evaluate_diff_of(curr, objects, self.hit_window)
            * (1.0 - decay)
            * Self::SKILL_MULTIPLIER;

        let current_rhythm = RhythmEvaluator::evaluate_diff_of(curr, objects, self.hit_window);
        let total_difficulty = self.current_strain * current_rhythm;

        if curr.base.is_slider() {
            self.slider_strains.push(total_difficulty);
        }

        total_difficulty
    }

    fn calculate_current_values(&self) -> (f64, f64) {
        if self.harmonic_skill_object_difficulties.is_empty() {
            return (0.0, 0.0);
        }

        harmonic_difficulty_value(
            self.harmonic_skill_object_difficulties.clone(),
            Self::HARMONIC_SCALE,
            Self::DECAY_EXPONENT,
        )
    }

    pub fn relevant_note_count(&self) -> f64 {
        self.harmonic_skill_object_difficulties
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .filter(|&n| n > 0.0)
            .map_or(0.0, |max_strain| {
                self.harmonic_skill_object_difficulties
                    .iter()
                    .fold(0.0, |sum, strain| {
                        sum + (1.0 + f64::exp(-(strain / max_strain * 12.0 - 6.0))).recip()
                    })
            })
    }

    pub fn count_top_weighted_sliders(&self, difficulty_value: f64) -> f64 {
        if self.slider_strains.is_empty() {
            return 0.0;
        }

        let (_, weight_sum) = self.calculate_current_values();

        if weight_sum == 0.0 {
            return 0.0;
        }

        // * What would the top note be if all note values were identical
        let consistent_top_note = difficulty_value / weight_sum;
        if consistent_top_note == 0.0 {
            return 0.0;
        }

        // * Use a weighted sum of all notes. Constants are arbitrary and give nice
        //   values
        self.slider_strains
            .iter()
            .map(|&s| {
                crate::util::difficulty::logistic(s / consistent_top_note, 0.88, 10.0, Some(1.1))
            })
            .sum()
    }

    pub fn count_top_weighted_difficulties(&self, difficulty_value: f64) -> f64 {
        let (_, weight_sum) = self.calculate_current_values();

        Self::count_top_weighted_object_difficulties(
            &self.harmonic_skill_object_difficulties,
            difficulty_value,
            weight_sum,
        )
    }
}

impl OsuHarmonicSkill for Speed {
    const HARMONIC_SCALE: f64 = 20.0;
}
