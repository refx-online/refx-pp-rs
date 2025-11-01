use crate::{
    Beatmap,
    model::mods::GameMods,
    osu::{
        object::{OsuObject, OsuObjectKind, NestedSliderObjectKind},
        convert::convert_objects,
        difficulty::scaling_factor::ScalingFactor,
    },
};

use super::utils::calculate_difficulty_peppy_stars;

const MAXIMUM_ROTATIONS_PER_SECOND: f64 = 477.0 / 60.0;
const MINIMUM_ROTATIONS_PER_SECOND: f64 = 3.0;

/// Attributes calculated by the legacy score simulator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LegacyScoreAttributes {
    /// Score from circles, sliders, and spinners (300 points each).
    pub accuracy_score: i32,
    /// Score from combo multiplier.
    pub combo_score: i32,
    /// Bonus score from spinner ticks and slider ticks.
    pub bonus_score: i32,
    /// Ratio of standardised bonus score to legacy bonus score.
    pub bonus_score_ratio: f64,
    /// Maximum combo achievable.
    pub max_combo: i32,
}

/// Simulates a perfect play through a beatmap to calculate legacy score components.
/// This is used for converting legacy scores (Score V1) to the standardised scoring system.
pub struct OsuLegacyScoreSimulator {
    legacy_bonus_score: i32,
    standardised_bonus_score: i32,
    combo: i32,
    score_multiplier: f64,
}

impl OsuLegacyScoreSimulator {
    pub fn new() -> Self {
        Self {
            legacy_bonus_score: 0,
            standardised_bonus_score: 0,
            combo: 0,
            score_multiplier: 0.0,
        }
    }

    pub fn simulate(&mut self, beatmap: &Beatmap, mods: &GameMods) -> LegacyScoreAttributes {
        self.legacy_bonus_score = 0;
        self.standardised_bonus_score = 0;
        self.combo = 0;

        self.score_multiplier = calculate_difficulty_peppy_stars(beatmap) as f64;

        let map_attrs = beatmap.attributes().mods(mods.clone()).build();
        let scaling_factor = ScalingFactor::new(map_attrs.cs);
        let time_preempt = map_attrs.hit_windows.ar * map_attrs.clock_rate;
        
        let mut attrs = crate::osu::OsuDifficultyAttributes::default();
        let osu_objects = convert_objects(
            beatmap,
            &scaling_factor,
            mods.reflection(),
            time_preempt,
            beatmap.hit_objects.len(),
            &mut attrs,
        );

        let mut attributes = LegacyScoreAttributes::default();

        for obj in osu_objects.iter() {
            self.simulate_hit(obj, &mut attributes);
        }

        attributes.bonus_score_ratio = if self.legacy_bonus_score == 0 {
            0.0
        } else {
            self.standardised_bonus_score as f64 / self.legacy_bonus_score as f64
        };
        attributes.bonus_score = self.legacy_bonus_score;
        attributes.max_combo = self.combo;

        attributes
    }

    fn simulate_hit(&mut self, hit_object: &OsuObject, attributes: &mut LegacyScoreAttributes) {
        match &hit_object.kind {
            OsuObjectKind::Circle => {
                self.simulate_circle(attributes);
            }
            OsuObjectKind::Slider(slider) => {
                self.simulate_slider(slider, attributes);
            }
            OsuObjectKind::Spinner(spinner) => {
                self.simulate_spinner(spinner, attributes);
            }
        }
    }

    fn simulate_circle(&mut self, attributes: &mut LegacyScoreAttributes) {
        let score_increase = 300;
        self.add_combo_score(score_increase, attributes);
        attributes.accuracy_score += score_increase;
        self.combo += 1;
    }

    fn simulate_slider(
        &mut self,
        slider: &crate::osu::object::OsuSlider,
        attributes: &mut LegacyScoreAttributes,
    ) {
        for nested in &slider.nested_objects {
            match nested.kind {
                NestedSliderObjectKind::Tick => {
                    attributes.accuracy_score += 10;
                    self.combo += 1;
                }
                NestedSliderObjectKind::Repeat => {
                    attributes.accuracy_score += 30;
                    self.combo += 1;
                }
                NestedSliderObjectKind::Tail => {
                    attributes.accuracy_score += 30;
                    self.combo += 1;
                }
            }
        }

        attributes.accuracy_score += 30;
        self.combo += 1;

        let score_increase = 300;
        self.add_combo_score(score_increase, attributes);
        attributes.accuracy_score += score_increase;
    }

    fn simulate_spinner(
        &mut self,
        spinner: &crate::model::hit_object::Spinner,
        attributes: &mut LegacyScoreAttributes,
    ) {
        let seconds_duration = spinner.duration / 1000.0;

        // * The total amount of half spins possible for the entire spinner.
        let total_half_spins_possible = (seconds_duration * MAXIMUM_ROTATIONS_PER_SECOND * 2.0) as i32;
        
        // * The amount of half spins that are required to successfully complete the spinner (i.e. get a 300).
        let half_spins_required_for_completion = (seconds_duration * MINIMUM_ROTATIONS_PER_SECOND) as i32;
        
        // * To be able to receive bonus points, the spinner must be rotated another 1.5 times.
        let half_spins_required_before_bonus = half_spins_required_for_completion + 3;

        for i in 0..=total_half_spins_possible {
            if i > half_spins_required_before_bonus && (i - half_spins_required_before_bonus) % 2 == 0 {
                self.legacy_bonus_score += 1100;
                self.standardised_bonus_score += 50;
            } else if i > 1 && i % 2 == 0 {
                self.legacy_bonus_score += 100;
                self.standardised_bonus_score += 10;
            }
        }

        let score_increase = 300;
        self.add_combo_score(score_increase, attributes);
        attributes.accuracy_score += score_increase;
        self.combo += 1;
    }

    fn add_combo_score(&self, score_increase: i32, attributes: &mut LegacyScoreAttributes) {
        // * Integer division is intentional to match stable's behavior
        attributes.combo_score += (((self.combo - 1).max(0) * (score_increase / 25)) as f64 * self.score_multiplier as f64) as i32;
    }
}

impl Default for OsuLegacyScoreSimulator {
    fn default() -> Self {
        Self::new()
    }
}
