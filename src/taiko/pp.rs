use std::borrow::Cow;

use super::{TaikoDifficultyAttributes, TaikoPerformanceAttributes, TaikoScoreState, TaikoStars};
use crate::{
    Beatmap, DifficultyAttributes, GameMode, HitResultPriority, Mods, OsuPP, PerformanceAttributes,
};

/// Performance calculator on osu!taiko maps.
///
/// # Example
///
/// ```
/// use rosu_pp::{TaikoPP, Beatmap};
///
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
///
/// let pp_result = TaikoPP::new(&map)
///     .mods(8 + 64) // HDDT
///     .combo(1234)
///     .accuracy(98.5)
///     .n_misses(1)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", pp_result.pp(), pp_result.stars());
///
/// let next_result = TaikoPP::new(&map)
///     .attributes(pp_result) // reusing previous results for performance
///     .mods(8 + 64) // has to be the same to reuse attributes
///     .accuracy(99.5)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", next_result.pp(), next_result.stars());
/// ```
#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct TaikoPP<'map> {
    pub(crate) map: Cow<'map, Beatmap>,
    attributes: Option<TaikoDifficultyAttributes>,
    mods: u32,
    combo: Option<usize>,
    acc: Option<f64>,
    passed_objects: Option<usize>,
    clock_rate: Option<f64>,
    hitresult_priority: Option<HitResultPriority>,

    pub(crate) n300: Option<usize>,
    pub(crate) n100: Option<usize>,
    pub(crate) n_misses: Option<usize>,
}

impl<'map> TaikoPP<'map> {
    /// Create a new performance calculator for osu!taiko maps.
    #[inline]
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map: map.convert_mode(GameMode::Taiko),
            attributes: None,
            mods: 0,
            combo: None,
            acc: None,
            n_misses: None,
            passed_objects: None,
            clock_rate: None,
            n300: None,
            n100: None,
            hitresult_priority: None,
        }
    }

    /// Provide the result of a previous difficulty or performance calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attrs: impl TaikoAttributeProvider) -> Self {
        if let Some(attrs) = attrs.attributes() {
            self.attributes = Some(attrs);
        }

        self
    }

    /// Specify mods through their bit values.
    ///
    /// See [https://github.com/ppy/osu-api/wiki#mods](https://github.com/ppy/osu-api/wiki#mods)
    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }

    /// Specify the max combo of the play.
    #[inline]
    pub fn combo(mut self, combo: usize) -> Self {
        self.combo = Some(combo);

        self
    }

    /// Specify how hitresults should be generated.
    ///
    /// Defauls to [`HitResultPriority::BestCase`].
    #[inline]
    pub fn hitresult_priority(mut self, priority: HitResultPriority) -> Self {
        self.hitresult_priority = Some(priority);

        self
    }

    /// Specify the amount of 300s of a play.
    #[inline]
    pub fn n300(mut self, n300: usize) -> Self {
        self.n300 = Some(n300);

        self
    }

    /// Specify the amount of 100s of a play.
    #[inline]
    pub fn n100(mut self, n100: usize) -> Self {
        self.n100 = Some(n100);

        self
    }

    /// Specify the amount of misses of the play.
    #[inline]
    pub fn n_misses(mut self, n_misses: usize) -> Self {
        self.n_misses = Some(n_misses.min(self.map.n_circles as usize));

        self
    }

    /// Specify the accuracy of a play between `0.0` and `100.0`.
    /// This will be used to generate matching hitresults.
    #[inline]
    pub fn accuracy(mut self, acc: f64) -> Self {
        self.acc = Some(acc / 100.0);

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// If you want to calculate the performance after every few objects, instead of
    /// using [`TaikoPP`] multiple times with different `passed_objects`, you should use
    /// [`TaikoGradualPerformanceAttributes`](crate::taiko::TaikoGradualPerformanceAttributes).
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects = Some(passed_objects);

        self
    }

    /// Adjust the clock rate used in the calculation.
    /// If none is specified, it will take the clock rate based on the mods
    /// i.e. 1.5 for DT, 0.75 for HT and 1.0 otherwise.
    #[inline]
    pub fn clock_rate(mut self, clock_rate: f64) -> Self {
        self.clock_rate = Some(clock_rate);

        self
    }

    /// Provide parameters through a [`TaikoScoreState`].
    #[inline]
    pub fn state(mut self, state: TaikoScoreState) -> Self {
        let TaikoScoreState {
            max_combo,
            n300,
            n100,
            n_misses,
        } = state;

        self.combo = Some(max_combo);
        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.n_misses = Some(n_misses);

        self
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> TaikoPerformanceAttributes {
        let attrs = self.attributes.take().unwrap_or_else(|| {
            let mut calculator = TaikoStars::new(self.map.as_ref())
                .mods(self.mods)
                .is_convert(matches!(self.map, Cow::Owned(_)));

            if let Some(passed_objects) = self.passed_objects {
                calculator = calculator.passed_objects(passed_objects);
            }

            if let Some(clock_rate) = self.clock_rate {
                calculator = calculator.clock_rate(clock_rate);
            }

            calculator.calculate()
        });

        let inner = TaikoPpInner {
            mods: self.mods,
            state: self.generate_hitresults(attrs.max_combo),
            attrs,
        };

        inner.calculate()
    }

    fn generate_hitresults(&self, max_combo: usize) -> TaikoScoreState {
        let total_result_count = if let Some(passed_objects) = self.passed_objects {
            max_combo.min(passed_objects)
        } else {
            max_combo
        };

        let priority = self.hitresult_priority.unwrap_or_default();

        let mut n300 = self.n300.unwrap_or(0);
        let mut n100 = self.n100.unwrap_or(0);
        let n_misses = self.n_misses.unwrap_or(0);

        if let Some(acc) = self.acc {
            match (self.n300, self.n100) {
                (Some(_), Some(_)) => {
                    let remaining = total_result_count.saturating_sub(n300 + n100 + n_misses);

                    match priority {
                        HitResultPriority::BestCase => n300 += remaining,
                        HitResultPriority::WorstCase => n100 += remaining,
                    }
                }
                (Some(_), None) => n100 += total_result_count.saturating_sub(n300 + n_misses),
                (None, Some(_)) => n300 += total_result_count.saturating_sub(n100 + n_misses),
                (None, None) => {
                    let target_total = (acc * (total_result_count * 2) as f64).round() as usize;
                    n300 = target_total - (total_result_count.saturating_sub(n_misses));
                    n100 = total_result_count.saturating_sub(n300 + n_misses);
                }
            }
        } else {
            let remaining = total_result_count.saturating_sub(n300 + n100 + n_misses);

            match priority {
                HitResultPriority::BestCase => match (self.n300, self.n100) {
                    (Some(_), None) => n100 = remaining,
                    (Some(_), Some(_)) => n300 += remaining,
                    (None, _) => n300 = remaining,
                },
                HitResultPriority::WorstCase => match (self.n300, self.n100) {
                    (None, Some(_)) => n300 = remaining,
                    (Some(_), Some(_)) => n100 += remaining,
                    (_, None) => n100 = remaining,
                },
            }
        }

        let max_combo = self.combo.map_or(max_combo, |combo| combo.min(max_combo));

        TaikoScoreState {
            max_combo,
            n300,
            n100,
            n_misses,
        }
    }
}

struct TaikoPpInner {
    attrs: TaikoDifficultyAttributes,
    mods: u32,
    state: TaikoScoreState,
}

impl TaikoPpInner {
    fn calculate(self) -> TaikoPerformanceAttributes {
        // * The effectiveMissCount is calculated by gaining a ratio for totalSuccessfulHits
        // * and increasing the miss penalty for shorter object counts lower than 1000.
        let total_successful_hits = self.total_successful_hits();

        let effective_miss_count = if total_successful_hits > 0 {
            (1000.0 / (total_successful_hits as f64)).max(1.0) * self.state.n_misses as f64
        } else {
            0.0
        };

        let mut multiplier = 1.13;

        if self.mods.hd() {
            multiplier *= 1.075;
        }

        if self.mods.ez() {
            multiplier *= 0.975;
        }

        let diff_value = self.compute_difficulty_value(effective_miss_count);
        let acc_value = self.compute_accuracy_value();

        let pp = (diff_value.powf(1.1) + acc_value.powf(1.1)).powf(1.0 / 1.1) * multiplier;

        TaikoPerformanceAttributes {
            difficulty: self.attrs,
            pp,
            pp_acc: acc_value,
            pp_difficulty: diff_value,
            effective_miss_count,
        }
    }

    fn compute_difficulty_value(&self, effective_miss_count: f64) -> f64 {
        let attrs = &self.attrs;
        let exp_base = 5.0 * (attrs.stars / 0.115).max(1.0) - 4.0;
        let mut diff_value = exp_base.powf(2.25) / 1150.0;

        let len_bonus = 1.0 + 0.1 * (attrs.max_combo as f64 / 1500.0).min(1.0);
        diff_value *= len_bonus;

        diff_value *= 0.986_f64.powf(effective_miss_count);

        if self.mods.ez() {
            diff_value *= 0.985;
        }

        if self.mods.hd() {
            diff_value *= 1.025;
        }

        if self.mods.hr() {
            diff_value *= 1.05;
        }

        if self.mods.fl() {
            diff_value *= 1.05 * len_bonus;
        }

        let acc = self.custom_accuracy();

        diff_value * acc * acc
    }

    #[inline]
    fn compute_accuracy_value(&self) -> f64 {
        if self.attrs.hit_window <= 0.0 {
            return 0.0;
        }

        let mut acc_value = (60.0 / self.attrs.hit_window).powf(1.1)
            * self.custom_accuracy().powi(8)
            * self.attrs.stars.powf(0.4)
            * 27.0;

        let len_bonus = (self.total_hits() / 1500.0).powf(0.3).min(1.15);
        acc_value *= len_bonus;

        // * Slight HDFL Bonus for accuracy. A clamp is used to prevent against negative values
        if self.mods.hd() && self.mods.fl() {
            acc_value *= (1.075 * len_bonus).max(1.05);
        }

        acc_value
    }

    fn total_hits(&self) -> f64 {
        self.state.total_hits() as f64
    }

    fn total_successful_hits(&self) -> usize {
        self.state.n300 + self.state.n100
    }

    fn custom_accuracy(&self) -> f64 {
        let total_hits = self.state.total_hits();

        if total_hits == 0 {
            return 0.0;
        }

        let numerator = self.state.n300 * 300 + self.state.n100 * 150;
        let denominator = total_hits * 300;

        numerator as f64 / denominator as f64
    }
}

impl<'map> From<OsuPP<'map>> for TaikoPP<'map> {
    #[inline]
    fn from(osu: OsuPP<'map>) -> Self {
        let OsuPP {
            map,
            attributes: _,
            mods,
            acc,
            combo,
            n300,
            n100,
            n50: _,
            n_misses,
            passed_objects,
            clock_rate,
            hitresult_priority,
            ..
        } = osu;

        Self {
            map: map.convert_mode(GameMode::Taiko),
            attributes: None,
            mods,
            combo,
            acc,
            passed_objects,
            clock_rate,
            hitresult_priority,
            n300,
            n100,
            n_misses,
        }
    }
}

/// Abstract type to provide flexibility when passing difficulty attributes to a performance calculation.
pub trait TaikoAttributeProvider {
    /// Provide the actual difficulty attributes.
    fn attributes(self) -> Option<TaikoDifficultyAttributes>;
}

impl TaikoAttributeProvider for TaikoDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<TaikoDifficultyAttributes> {
        Some(self)
    }
}

impl TaikoAttributeProvider for TaikoPerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<TaikoDifficultyAttributes> {
        Some(self.difficulty)
    }
}

impl TaikoAttributeProvider for DifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<TaikoDifficultyAttributes> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Taiko(attributes) = self {
            Some(attributes)
        } else {
            None
        }
    }
}

impl TaikoAttributeProvider for PerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<TaikoDifficultyAttributes> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Taiko(attributes) = self {
            Some(attributes.difficulty)
        } else {
            None
        }
    }
}

#[cfg(not(any(feature = "async_tokio", feature = "async_std")))]
#[cfg(test)]
mod test {
    use super::*;
    use crate::Beatmap;

    fn test_data() -> (Beatmap, TaikoDifficultyAttributes) {
        let path = "./maps/1028484.osu";
        let map = Beatmap::from_path(path).unwrap();

        let attrs = TaikoDifficultyAttributes {
            stamina: 1.4528845068865617,
            rhythm: 0.20130047251681948,
            colour: 1.0487315549761433,
            peak: 1.8881824429738323,
            hit_window: 35.0,
            stars: 2.9778030386845606,
            max_combo: 289,
        };

        (map, attrs)
    }

    #[test]
    fn hitresults_n300_n_misses_best() {
        let (map, attrs) = test_data();
        let max_combo = attrs.max_combo();

        let state = TaikoPP::new(&map)
            .attributes(attrs)
            .combo(100)
            .n300(150)
            .n_misses(2)
            .hitresult_priority(HitResultPriority::BestCase)
            .generate_hitresults(max_combo);

        let expected = TaikoScoreState {
            max_combo: 100,
            n300: 150,
            n100: 137,
            n_misses: 2,
        };

        assert_eq!(state, expected);
    }

    #[test]
    fn hitresults_n_misses_best() {
        let (map, attrs) = test_data();
        let max_combo = attrs.max_combo();

        let state = TaikoPP::new(&map)
            .attributes(attrs)
            .combo(100)
            .n_misses(2)
            .hitresult_priority(HitResultPriority::BestCase)
            .generate_hitresults(max_combo);

        let expected = TaikoScoreState {
            max_combo: 100,
            n300: 287,
            n100: 0,
            n_misses: 2,
        };

        assert_eq!(state, expected);
    }

    #[test]
    fn hitresults_acc_n_misses_worst() {
        let (map, attrs) = test_data();
        let max_combo = attrs.max_combo();

        let state = TaikoPP::new(&map)
            .attributes(attrs)
            .combo(100)
            .accuracy(97.2)
            .n_misses(2)
            .hitresult_priority(HitResultPriority::WorstCase)
            .generate_hitresults(max_combo);

        let expected = TaikoScoreState {
            max_combo: 100,
            n300: 275,
            n100: 12,
            n_misses: 2,
        };

        assert_eq!(
            state,
            expected,
            "{}% vs {}%",
            state.accuracy(),
            expected.accuracy()
        );
    }
}
