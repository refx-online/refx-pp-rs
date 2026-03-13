use super::strain::OsuStrainSkill;
use crate::{
    any::difficulty::{
        object::{HasStartTime, IDifficultyObject},
        skills::{StrainSkill, strain_decay},
    },
    osu::difficulty::{
        evaluators::{AgilityEvaluator, FlowAimEvaluator, SnapAimEvaluator},
        object::OsuDifficultyObject,
    },
    util::{
        difficulty::{logistic, logistic_exp, norm},
        float_ext::FloatExt,
        strains_vec::StrainsVec,
    },
};

define_skill! {
    #[derive(Clone)]
    pub struct Aim: StrainSkill => [OsuDifficultyObject<'a>][OsuDifficultyObject<'a>] {
        include_sliders: bool,
        with_td: bool,
        radius: f64,
        current_strain: f64 = 0.0,
        slider_strains: Vec<f64> = Vec::with_capacity(64), // TODO: use `StrainsVec`?
    }
}

impl Aim {
    const MEAN_EXPONENT: f64 = 1.2;
    const SKILL_MULTIPLIER_AGILITY: f64 = 2.0;
    const SKILL_MULTIPLIER_FLOW: f64 = 244.0;
    const SKILL_MULTIPLIER_SNAP: f64 = 71.0;
    const SKILL_MULTIPLIER_TOTAL: f64 = 1.1;
    const STRAIN_DECAY_BASE: f64 = 0.15;

    fn calculate_initial_strain(
        &mut self,
        time: f64,
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let prev_start_time = curr
            .previous(0, objects)
            .map_or(0.0, HasStartTime::start_time);

        self.current_strain * strain_decay(time - prev_start_time, Self::STRAIN_DECAY_BASE)
    }

    fn strain_value_at(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        objects: &[OsuDifficultyObject<'_>],
    ) -> f64 {
        let decay = strain_decay(curr.adjusted_delta_time, Self::STRAIN_DECAY_BASE);

        let mut snap_difficulty =
            SnapAimEvaluator::evaluate_diff_of(curr, objects, self.include_sliders)
                * Self::SKILL_MULTIPLIER_SNAP;
        let agility_difficulty =
            AgilityEvaluator::evaluate_diff_of(curr, objects) * Self::SKILL_MULTIPLIER_AGILITY;
        let mut flow_difficulty =
            FlowAimEvaluator::evaluate_diff_of(curr, objects, self.include_sliders, self.radius)
                * Self::SKILL_MULTIPLIER_FLOW;

        if self.with_td {
            snap_difficulty = snap_difficulty.powf(0.89);
            // * we don't adjust agility here since agility represents TD difficulty in a
            //   decent enough way
            flow_difficulty = flow_difficulty.powf(1.1);
        }

        let total_difficulty = Self::calculate_total_value(
            Self::MEAN_EXPONENT,
            snap_difficulty,
            agility_difficulty,
            flow_difficulty,
            Self::SKILL_MULTIPLIER_TOTAL,
        );

        self.current_strain *= decay;
        self.current_strain += total_difficulty * (1.0 - decay);

        if curr.base.is_slider() {
            self.slider_strains.push(self.current_strain);
        }

        self.current_strain
    }

    fn calculate_total_value(
        mean_exponent: f64,
        snap_difficulty: f64,
        agility_difficulty: f64,
        flow_difficulty: f64,
        skill_multiplier_total: f64,
    ) -> f64 {
        // * We compare flow to combined snap and agility because snap by itself doesn't
        //   have enough difficulty to be above flow on streams
        // * Agility on the other hand is supposed to measure the rate of cursor
        //   velocity changes while snapping
        // * So snapping every circle on a stream requires an enormous amount of agility
        //   at which point it's easier to flow
        let combined_snap_difficulty = norm(mean_exponent, [snap_difficulty, agility_difficulty]);

        let p_snap =
            Self::calculate_snap_flow_probability(flow_difficulty / combined_snap_difficulty);
        let p_flow = 1.0 - p_snap;

        let total_difficulty = combined_snap_difficulty * p_snap + flow_difficulty * p_flow;

        total_difficulty * skill_multiplier_total
    }

    fn calculate_snap_flow_probability(ratio: f64) -> f64 {
        const K: f64 = 7.27; // why

        if FloatExt::eq(ratio, 0.0) {
            return 0.0;
        }

        if ratio.is_nan() || ratio.is_infinite() {
            return 1.0;
        }

        logistic_exp(-K * ratio.ln(), None)
    }

    pub fn get_difficult_sliders(&self) -> f64 {
        if self.slider_strains.is_empty() {
            return 0.0;
        }

        let max_slider_strain = self.slider_strains.iter().copied().fold(0.0, f64::max);

        if FloatExt::eq(max_slider_strain, 0.0) {
            return 0.0;
        }

        self.slider_strains
            .iter()
            .copied()
            .map(|strain| 1.0 / (1.0 + f64::exp(-(strain / max_slider_strain * 12.0 - 6.0))))
            .sum()
    }

    pub fn count_top_weighted_sliders(&self, difficulty_value: f64) -> f64 {
        if self.slider_strains.is_empty() {
            return 0.0;
        }

        // * What would the top strain be if all strain values were identical
        let consistent_top_strain = difficulty_value * (1.0 - Self::DECAY_WEIGHT);
        if consistent_top_strain == 0.0 {
            return 0.0;
        }

        // * Use a weighted sum of all strains. Constants are arbitrary and give nice
        //   values
        self.slider_strains
            .iter()
            .map(|&s| logistic(s / consistent_top_strain, 0.88, 10.0, Some(1.1)))
            .sum()
    }

    // From `OsuStrainSkill`; native rather than trait function so that it has
    // priority over `StrainSkill::difficulty_value`
    fn difficulty_value(current_strain_peaks: StrainsVec) -> f64 {
        super::strain::difficulty_value(
            current_strain_peaks,
            Self::REDUCED_SECTION_COUNT,
            Self::REDUCED_STRAIN_BASELINE,
            Self::DECAY_WEIGHT,
        )
    }
}

impl OsuStrainSkill for Aim {}
