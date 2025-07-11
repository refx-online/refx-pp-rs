use crate::{
    catch::{CatchGradualDifficulty, CatchPerformanceAttributes, CatchScoreState},
    model::mode::ConvertError,
    Beatmap, Difficulty,
};

/// Gradually calculate the performance attributes of an osu!catch map.
///
/// After each hit object you can call [`next`] and it will return the resulting
/// current [`CatchPerformanceAttributes`]. To process multiple objects at once,
/// use [`nth`] instead.
///
/// Both methods require a [`CatchScoreState`] that contains the current
/// hitresults as well as the maximum combo so far.
///
/// Note that neither hits nor misses of tiny droplets require to be processed.
/// Only fruits and droplets do.
///
/// If you only want to calculate difficulty attributes use
/// [`CatchGradualDifficulty`] instead.
///
/// # Example
///
/// ```
/// use refx_pp::{Beatmap, Difficulty};
/// use refx_pp::catch::{Catch, CatchGradualPerformance, CatchScoreState};
///
/// let map = Beatmap::from_path("./resources/2118524.osu").unwrap();
///
/// let difficulty = Difficulty::new().mods(64); // DT
/// let mut gradual = CatchGradualPerformance::new(difficulty, &map).unwrap();
/// let mut state = CatchScoreState::new(); // empty state, everything is on 0.
///
/// // The first 10 hitresults are only fruits
/// for _ in 0..10 {
///     state.fruits += 1;
///     state.max_combo += 1;
///
///     let attrs = gradual.next(state.clone()).unwrap();
///     println!("PP: {}", attrs.pp);
/// }
///
/// // Then comes a miss.
/// // Note that state's max combo won't be incremented for
/// // the next few objects because the combo is reset.
/// state.misses += 1;
/// let attrs = gradual.next(state.clone()).unwrap();
/// println!("PP: {}", attrs.pp);
///
/// // The next 10 objects will be a mixture of fruits and droplets.
/// // Notice how tiny droplets from sliders do not count as hit objects
/// // that require processing. Only fruits and droplets do.
/// // Also notice how all 10 objects will be processed in one go.
/// state.fruits += 4;
/// state.droplets += 6;
/// state.tiny_droplets += 12;
/// // The `nth` method takes a zero-based value.
/// let attrs = gradual.nth(state.clone(), 9).unwrap();
/// println!("PP: {}", attrs.pp);
///
/// // Now comes another fruit. Note that the max combo gets incremented again.
/// state.fruits += 1;
/// state.max_combo += 1;
/// let attrs = gradual.next(state.clone()).unwrap();
/// println!("PP: {}", attrs.pp);
///
/// // Skip to the end
/// # /*
/// state.max_combo = ...
/// state.fruits = ...
/// state.droplets = ...
/// state.tiny_droplets = ...
/// state.tiny_droplet_misses = ...
/// state.misses = ...
/// # */
/// let attrs = gradual.last(state.clone()).unwrap();
/// println!("PP: {}", attrs.pp);
///
/// // Once the final performance has been calculated, attempting to process
/// // further objects will return `None`.
/// assert!(gradual.next(state).is_none());
/// ```
///
/// [`next`]: CatchGradualPerformance::next
/// [`nth`]: CatchGradualPerformance::nth
pub struct CatchGradualPerformance {
    difficulty: CatchGradualDifficulty,
}

impl CatchGradualPerformance {
    /// Create a new gradual performance calculator for osu!catch maps.
    pub fn new(difficulty: Difficulty, map: &Beatmap) -> Result<Self, ConvertError> {
        let difficulty = CatchGradualDifficulty::new(difficulty, map)?;

        Ok(Self { difficulty })
    }

    /// Process the next hit object and calculate the performance attributes
    /// for the resulting score state.
    ///
    /// Note that neither hits nor misses of tiny droplets require to be
    /// processed. Only fruits and droplets do.
    pub fn next(&mut self, state: CatchScoreState) -> Option<CatchPerformanceAttributes> {
        self.nth(state, 0)
    }

    /// Process all remaining hit objects and calculate the final performance
    /// attributes.
    pub fn last(&mut self, state: CatchScoreState) -> Option<CatchPerformanceAttributes> {
        self.nth(state, usize::MAX)
    }

    /// Process everything up the the next `n`th hit object and calculate the
    /// performance attributes for the resulting score state.
    ///
    /// Note that the count is zero-indexed, so `n=0` will process 1 object,
    /// `n=1` will process 2, and so on.
    #[allow(clippy::missing_panics_doc)]
    pub fn nth(&mut self, state: CatchScoreState, n: usize) -> Option<CatchPerformanceAttributes> {
        let performance = self
            .difficulty
            .nth(n)?
            .performance()
            .state(state)
            .difficulty(self.difficulty.difficulty.clone())
            .passed_objects(self.difficulty.idx as u32)
            .calculate()
            .expect("no conversion required");

        Some(performance)
    }

    /// Returns the amount of remaining objects.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.difficulty.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::{catch::CatchPerformance, Beatmap};

    use super::*;

    #[test]
    fn next_and_nth() {
        let map = Beatmap::from_path("./resources/2118524.osu").unwrap();

        let difficulty = Difficulty::new().mods(88); // HDHRDT

        let mut gradual = CatchGradualPerformance::new(difficulty.clone(), &map).unwrap();
        let mut gradual_2nd = CatchGradualPerformance::new(difficulty.clone(), &map).unwrap();
        let mut gradual_3rd = CatchGradualPerformance::new(difficulty.clone(), &map).unwrap();

        let mut state = CatchScoreState::default();

        for i in 1.. {
            state.misses += 1;

            let Some(next_gradual) = gradual.next(state.clone()) else {
                assert_eq!(i, 731);
                assert!(gradual_2nd.last(state.clone()).is_none()); // 730 % 2 == 0
                assert!(gradual_3rd.last(state.clone()).is_some()); // 730 % 3 == 1
                break;
            };

            if i % 2 == 0 {
                let next_gradual_2nd = gradual_2nd.nth(state.clone(), 1).unwrap();
                assert_eq!(next_gradual, next_gradual_2nd);
            }

            if i % 3 == 0 {
                let next_gradual_3rd = gradual_3rd.nth(state.clone(), 2).unwrap();
                assert_eq!(next_gradual, next_gradual_3rd);
            }

            let regular_calc = CatchPerformance::new(&map)
                .difficulty(difficulty.clone())
                .passed_objects(i as u32)
                .state(state.clone());

            let expected = regular_calc.calculate().unwrap();

            assert_eq!(next_gradual, expected);
        }
    }
}
