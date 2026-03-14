use super::strain::OsuHarmonicSkill;
use crate::{
    osu::difficulty::{
        evaluators::ReadingEvaluator, object::OsuDifficultyObject,
        skills::strain::harmonic_difficulty_value,
    },
    util::{difficulty::logistic, float_ext::FloatExt},
};

define_skill! {
    pub struct Reading: HarmonicSkill => [OsuDifficultyObject<'a>][OsuDifficultyObject<'a>] {
        current_strain: f64 = 0.0,
        has_hidden_mod: bool,
        evaluator: ReadingEvaluator,
        object_start_times: Vec<f64> = Vec::with_capacity(256),
    }

    pub fn new(has_hidden_mod: bool, time_preempt: f64, time_fade_in: f64) -> Self {
        Self {
            current_strain: 0.0,
            has_hidden_mod: has_hidden_mod,
            evaluator: ReadingEvaluator::new(time_preempt, time_fade_in),
            object_start_times: Vec::with_capacity(256),
        }
    }
}

impl Reading {
    // * Assume the first seconds are completely memorised
    const REDUCED_DIFFICULTY_BASE_LINE: f64 = 0.0;
    const REDUCED_DIFFICULTY_DURATION: f64 = 60_000.0;
    const SKILL_MULTIPLIER: f64 = 2.5;
    const STRAIN_DECAY_BASE: f64 = 0.8;

    fn strain_value_at(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        self.object_start_times.push(curr.start_time);

        self.current_strain *= Self::strain_decay(curr.delta_time);

        self.current_strain += self
            .evaluator
            .evaluate_diff_of(curr, objects, self.has_hidden_mod)
            * Self::SKILL_MULTIPLIER;

        self.current_strain
    }

    fn calculate_current_values(&self) -> (f64, f64) {
        if self.harmonic_skill_object_difficulties.is_empty() {
            return (0.0, 0.0);
        }

        let mut difficulties = self.harmonic_skill_object_difficulties.clone();

        self.apply_difficulty_transformation(&mut difficulties);

        harmonic_difficulty_value(difficulties, Self::HARMONIC_SCALE, Self::DECAY_EXPONENT)
    }

    fn strain_decay(ms: f64) -> f64 {
        Self::STRAIN_DECAY_BASE.powf(ms / 1000.0)
    }

    fn apply_difficulty_transformation(&self, difficulties: &mut [f64]) {
        let reduced_note_count = self.calculate_reduced_note_count();
        let limit = difficulties.len().min(reduced_note_count);

        if reduced_note_count == 0 {
            return;
        }

        for (i, diff) in difficulties.iter_mut().take(limit).enumerate() {
            let clamped = (i as f64 / reduced_note_count as f64).clamp(0.0, 1.0);
            let scale = (1.0 + 9.0 * clamped).log10();
            let lerp = f64::lerp(Self::REDUCED_DIFFICULTY_BASE_LINE, 1.0, scale);
            *diff *= lerp;
        }
    }

    fn calculate_reduced_note_count(&self) -> usize {
        let Some(&first_start_time) = self.object_start_times.first() else {
            return 0;
        };

        let reduced_duration = first_start_time + Self::REDUCED_DIFFICULTY_DURATION;

        self.object_start_times
            .iter()
            .take_while(|&&start_time| start_time <= reduced_duration)
            .count()
    }
}

impl OsuHarmonicSkill for Reading {
    fn count_top_weighted_object_difficulties(
        object_difficulties: &[f64],
        difficulty_value: f64,
        note_weight_sum: f64,
    ) -> f64 {
        if object_difficulties.is_empty() || note_weight_sum == 0.0 {
            return 0.0;
        }

        // * What would the top difficulty be if all object difficulties were identical
        let consistent_top_note = difficulty_value / note_weight_sum;
        if consistent_top_note == 0.0 {
            return 0.0;
        }

        object_difficulties
            .iter()
            .map(|&d| logistic(d / consistent_top_note, 1.15, 5.0, Some(1.1)))
            .sum()
    }
}
