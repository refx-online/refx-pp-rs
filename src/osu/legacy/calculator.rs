use crate::{
    model::mods::GameMods,
    osu::{OsuDifficultyAttributes, OsuScoreState},
};

pub struct OsuLegacyScoreMissCalculator<'a> {
    state: &'a OsuScoreState,
    attrs: &'a OsuDifficultyAttributes,
    mods: &'a GameMods,
    legacy_total_score: Option<i64>,
    accuracy: f64,
}

impl<'a> OsuLegacyScoreMissCalculator<'a> {
    pub const fn new(
        state: &'a OsuScoreState,
        attrs: &'a OsuDifficultyAttributes,
        mods: &'a GameMods,
        legacy_total_score: Option<i64>,
        accuracy: f64,
    ) -> Self {
        Self {
            state,
            attrs,
            mods,
            legacy_total_score,
            accuracy,
        }
    }

    pub fn calculate(&self) -> f64 {
        if self.attrs.max_combo == 0 || self.legacy_total_score.is_none() {
            return 0.0;
        }

        let legacy_total_score = self.legacy_total_score.unwrap() as f64;

        let score_v1_multiplier = self.attrs.legacy_score_base_multiplier 
            * self.get_legacy_score_multiplier();
        
        let relevant_combo_per_object = self.calculate_relevant_score_combo_per_object();

        let maximum_miss_count = self.calculate_maximum_combo_based_miss_count();

        let score_obtained_during_max_combo = self.calculate_score_at_combo(
            f64::from(self.state.max_combo),
            relevant_combo_per_object,
            score_v1_multiplier,
        );

        let remaining_score = legacy_total_score - score_obtained_during_max_combo;

        if remaining_score <= 0.0 {
            return maximum_miss_count;
        }

        let remaining_combo = f64::from(self.attrs.max_combo) - f64::from(self.state.max_combo);
        let expected_remaining_score = self.calculate_score_at_combo(
            remaining_combo,
            relevant_combo_per_object,
            score_v1_multiplier,
        );

        let mut score_based_miss_count = expected_remaining_score / remaining_score;

        // * If there's less than one miss detected - let combo-based miss count decide if this is FC or not
        score_based_miss_count = score_based_miss_count.max(1.0);

        // * Cap result by very harsh version of combo-based miss count
        score_based_miss_count.min(maximum_miss_count)
    }

    /// Calculates the amount of score that would be achieved at a given combo.
    fn calculate_score_at_combo(
        &self,
        combo: f64,
        relevant_combo_per_object: f64,
        score_v1_multiplier: f64,
    ) -> f64 {
        let total_hits = f64::from(self.state.total_hits());
        let count_miss = f64::from(self.state.misses);

        let estimated_objects = combo / relevant_combo_per_object - 1.0;

        // * The combo portion of ScoreV1 follows arithmetic progression
        // * Therefore, we calculate the combo portion of score using the combo per object and our current combo.
        let combo_score = if relevant_combo_per_object > 0.0 {
            (2.0 * (relevant_combo_per_object - 1.0) + (estimated_objects - 1.0) * relevant_combo_per_object) 
                * estimated_objects / 2.0
        } else {
            0.0
        };

        // * We then apply the accuracy and ScoreV1 multipliers to the resulting score.
        let combo_score = combo_score * self.accuracy * 300.0 / 25.0 * score_v1_multiplier;

        let objects_hit = (total_hits - count_miss) * combo / f64::from(self.attrs.max_combo);

        // * Score also has a non-combo portion we need to create the final score value.
        let non_combo_score = (300.0 + self.attrs.nested_score_per_object) 
            * self.accuracy
            * objects_hit;

        combo_score + non_combo_score
    }

    /// Calculates the relevant combo per object for legacy score.
    /// This assumes a uniform distribution for circles and sliders.
    /// This handles cases where objects (such as buzz sliders) do not fit a normal arithmetic progression model.
    fn calculate_relevant_score_combo_per_object(&self) -> f64 {
        let mut combo_score = self.attrs.maximum_legacy_combo_score;

        // * We then reverse apply the ScoreV1 multipliers to get the raw value.
        combo_score /= 300.0 / 25.0 * self.attrs.legacy_score_base_multiplier;

        // * Reverse the arithmetic progression to work out the amount of combo per object based on the score.
        let result = (self.attrs.max_combo - 2) * self.attrs.max_combo;
        

        f64::from(result) / f64::max(
            f64::from(self.attrs.max_combo) + 2.0 * (combo_score - 1.0),
            1.0,
        )
    }

    /// This function is a harsher version of current combo-based miss count, 
    /// used to provide reasonable value for cases where score-based miss count can't do this.
    fn calculate_maximum_combo_based_miss_count(&self) -> f64 {
        let count_miss = f64::from(self.state.misses);

        if self.attrs.n_sliders == 0 {
            return count_miss;
        }

        let count_ok = f64::from(self.state.n100);
        let count_meh = f64::from(self.state.n50);

        let total_imperfect_hits = count_ok + count_meh + count_miss;

        let mut miss_count = 0.0;

        // * Consider that full combo is maximum combo minus dropped slider tails since they don't contribute to combo but also don't break it
        // In classic scores we can't know the amount of dropped sliders so we estimate to 10% of all sliders on the map
        let full_combo_threshold = f64::from(self.attrs.max_combo) - 0.1 * f64::from(self.attrs.n_sliders);

        if f64::from(self.state.max_combo) < full_combo_threshold {
            miss_count = f64::powf(
                full_combo_threshold / f64::max(1.0, f64::from(self.state.max_combo)),
                2.5,
            );
        }

        // * In classic scores there can't be more misses than a sum of all non-perfect judgements
        miss_count = miss_count.min(total_imperfect_hits);

        // * Every slider has *at least* 2 combo attributed in classic mechanics.
        // * If they broke on a slider with a tick, then this still works since they would have lost at least 2 combo (the tick and the end)
        // * Using this as a max means a score that loses 1 combo on a map can't possibly have been a slider break.
        // * It must have been a slider end.
        let max_possible_slider_breaks = (self.attrs.n_sliders as i32)
            .min((self.attrs.max_combo as i32 - self.state.max_combo as i32) / 2);

        let score_miss_count = f64::from(self.state.misses);

        let slider_breaks = miss_count - score_miss_count;

        if slider_breaks > f64::from(max_possible_slider_breaks) {
            miss_count = score_miss_count + f64::from(max_possible_slider_breaks);
        }

        // * In classic scores there can't be more misses than a sum of all non-perfect judgements
        miss_count = miss_count.min(total_imperfect_hits);

        miss_count
    }

    fn get_legacy_score_multiplier(&self) -> f64 {
        let mut multiplier = 1.0;

        if self.mods.nf() {
            multiplier *= if self.mods.score_v2() { 1.0 } else { 0.5 };
        }
        if self.mods.ez() {
            multiplier *= 0.5;
        }
        if self.mods.ht() {
            multiplier *= 0.3;
        }
        if self.mods.hd() {
            multiplier *= 1.06;
        }
        if self.mods.hr() {
            multiplier *= if self.mods.score_v2() { 1.10 } else { 1.06 };
        }
        if self.mods.dt() {
            multiplier *= if self.mods.score_v2() { 1.20 } else { 1.12 };
        }
        if self.mods.fl() {
            multiplier *= 1.12;
        }
        if self.mods.so() {
            multiplier *= 0.9;
        }
        if self.mods.rx() || self.mods.ap() {
            return 0.0;
        }

        multiplier
    }
}
