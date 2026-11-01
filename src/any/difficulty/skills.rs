use crate::util::{float_ext::FloatExt, hint::unlikely, strains_vec::StrainsVec};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrainPeak {
    pub value: f64,
    pub section_length: f64,
}

pub trait StrainSkill: Sized {
    type DifficultyObject<'a>;
    type DifficultyObjects<'a>: ?Sized;

    const DECAY_WEIGHT: f64 = 0.9;
    const SECTION_LENGTH: i32 = 400;

    fn process<'a>(
        &mut self,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    );

    fn count_top_weighted_strains(&self, difficulty_value: f64) -> f64;

    fn save_current_peak(&mut self);

    fn start_new_section_from<'a>(
        &mut self,
        time: f64,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    );

    fn into_current_strain_peaks(self) -> StrainsVec;

    fn get_current_strain_peaks(
        mut strain_peaks: StrainsVec,
        current_section_peak: f64,
    ) -> StrainsVec {
        strain_peaks.push(current_section_peak);

        strain_peaks
    }

    fn difficulty_value(current_strain_peaks: StrainsVec) -> f64;

    fn into_difficulty_value(self) -> f64;

    fn cloned_difficulty_value(&self) -> f64;
}

pub trait StrainDecaySkill: StrainSkill {
    fn calculate_initial_strain<'a>(
        &self,
        time: f64,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    ) -> f64;

    fn strain_value_at<'a>(
        &mut self,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    ) -> f64;

    fn strain_decay(ms: f64) -> f64;
}

pub trait HarmonicSkill: Sized {
    type DifficultyObject<'a>;
    type DifficultyObjects<'a>: ?Sized;

    fn process<'a>(
        &mut self,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    );

    fn cloned_difficulty_value(&self) -> f64;

    fn count_top_weighted_difficulties(&self, difficulty_value: f64) -> f64;
}

pub trait VariableLengthStrainSkill: Sized {
    type DifficultyObject<'a>;
    type DifficultyObjects<'a>: ?Sized;

    const DECAY_WEIGHT: f64 = 0.9;
    const MAX_SECTION_LENGTH: i32 = 400;

    fn process<'a>(
        &mut self,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    );

    fn save_current_peak(&mut self, section_length: f64);

    fn start_new_section_from<'a>(
        &mut self,
        time: f64,
        curr: &Self::DifficultyObject<'a>,
        objects: &Self::DifficultyObjects<'a>,
    );

    fn get_current_strain_peaks(&self) -> impl Iterator<Item = StrainPeak>;

    fn cloned_difficulty_value(&self) -> f64;

    fn count_top_weighted_strains(&self, difficulty_value: f64) -> f64;

    fn into_current_strain_peaks(self) -> Vec<f64>;
}

pub fn count_top_weighted_strains(
    object_strains: &[f64],
    difficulty_value: f64,
    decay_weight: f64,
) -> f64 {
    if unlikely(object_strains.is_empty()) {
        return 0.0;
    }

    // * What would the top strain be if all strain values were identical
    let consistent_top_strain = difficulty_value * (1.0 - decay_weight);

    if unlikely(FloatExt::eq(consistent_top_strain, 0.0)) {
        return object_strains.len() as f64;
    }

    // * Use a weighted sum of all strains. Constants are arbitrary and give nice
    //   values
    object_strains
        .iter()
        .map(|s| 1.1 / (1.0 + f64::exp(-10.0 * (s / consistent_top_strain - 0.88))))
        .sum()
}

pub fn difficulty_value(current_strain_peaks: StrainsVec, decay_weight: f64) -> f64 {
    let mut difficulty = 0.0;
    let mut weight = 1.0;

    // * Sections with 0 strain are excluded to avoid worst-case time complexity of
    //   the following sort (e.g. /b/2351871).
    // * These sections will not contribute to the difficulty.
    let mut peaks = current_strain_peaks;
    peaks.retain_non_zero_and_sort();

    // SAFETY: we just removed all zeros
    let peaks = unsafe { peaks.transmute_into_vec() };

    // * Difficulty is the weighted sum of the highest strains from every section.
    // * We're sorting from highest to lowest strain.
    for strain in peaks {
        difficulty += strain * weight;
        weight *= decay_weight;
    }

    difficulty
}

pub fn strain_decay(ms: f64, strain_decay_base: f64) -> f64 {
    f64::powf(strain_decay_base, ms / 1000.0)
}

pub fn get_reduced_strain_peaks(
    peaks: Vec<StrainPeak>,
    reduced_section_time: f64,
    reduced_strain_baseline: f64,
) -> Vec<StrainPeak> {
    // * Sections with 0 strain are excluded to avoid worst-case time complexity of
    //   the following sort (e.g. /b/2351871).
    // * These sections will not contribute to the difficulty.
    let peaks = peaks.into_iter().filter(|p| p.value > 0.0);

    let mut strains: Vec<StrainPeak> = peaks.collect::<Vec<_>>().into_iter().collect();

    strains.sort_unstable_by(|a, b| b.value.total_cmp(&a.value));

    #[expect(clippy::items_after_statements, reason = "staying in-sync with lazer")]
    const CHUNK_SIZE: f64 = 20.0;

    let mut time = 0.0;

    // * All strains are removed at the end for optimization purposes
    let mut strains_to_remove = 0;

    // * We are reducing the highest strains first to account for extreme difficulty spikes
    // * Strains are split into 20ms chunks to try to mitigate inconsistencies caused by reducing strains
    while strains_to_remove < strains.len() && time < reduced_section_time {
        let strain = &strains[strains_to_remove];
        let strain_value = strain.value;
        let strain_section_length = strain.section_length;

        let mut added_time = 0.0;
        while added_time < strain_section_length {
            let t = (time + added_time) / reduced_section_time;
            let scale = f64::log10(f64::lerp(1.0, 10.0, t.clamp(0.0, 1.0)));

            strains.push(StrainPeak {
                value: strain_value * f64::lerp(reduced_strain_baseline, 1.0, scale),
                section_length: f64::min(CHUNK_SIZE, strain_section_length - added_time),
            });

            added_time += CHUNK_SIZE;
        }

        time += strain_section_length;
        strains_to_remove += 1;
    }

    strains.drain(0..strains_to_remove);
    strains.sort_unstable_by(|a, b| b.value.total_cmp(&a.value));

    strains
}
