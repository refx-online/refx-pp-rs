use std::{cmp::Ordering, mem};

use crate::osu::{difficulty_object::OsuDifficultyObject, SECTION_LEN};

pub(crate) trait Skill {
    fn process(&mut self, curr: &OsuDifficultyObject<'_>, diff_objects: &[OsuDifficultyObject<'_>]);
    #[allow(dead_code)]
    fn difficulty_value(&mut self) -> f64;
}

pub(crate) trait StrainSkill: Skill + Sized {
    const DECAY_WEIGHT: f64 = 0.9;

    fn strain_peaks_mut(&mut self) -> &mut Vec<f64>;
    fn curr_section_peak(&mut self) -> &mut f64;
    fn curr_section_end(&mut self) -> &mut f64;

    fn strain_value_at(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
    ) -> f64;

    fn calculate_initial_strain(
        &self,
        time: f64,
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
    ) -> f64;

    fn process(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
    ) {
        // * The first object doesn't generate a strain, so we begin with an incremented section end
        if curr.idx == 0 {
            let section_len = SECTION_LEN as f64;
            *self.curr_section_end() = (curr.start_time / section_len).ceil() * section_len;
        }

        while curr.start_time > *self.curr_section_end() {
            self.save_curr_peak();

            {
                let section_end = *self.curr_section_end();
                self.start_new_section_from(section_end, curr, diff_objects);
            }

            *self.curr_section_end() += SECTION_LEN as f64;
        }

        *self.curr_section_peak() = self
            .strain_value_at(curr, diff_objects)
            .max(*self.curr_section_peak());
    }

    #[inline]
    fn save_curr_peak(&mut self) {
        let peak = *self.curr_section_peak();
        self.strain_peaks_mut().push(peak);
    }

    #[inline]
    fn start_new_section_from(
        &mut self,
        time: f64,
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
    ) {
        // * The maximum strain of the new section is not zero by default
        // * This means we need to capture the strain level at the beginning of the new section,
        // * and use that as the initial peak level.
        *self.curr_section_peak() = self.calculate_initial_strain(time, curr, diff_objects);
    }

    #[allow(dead_code)]
    fn difficulty_value(&mut self) -> f64;

    #[inline]
    fn get_curr_strain_peaks(&mut self) -> Vec<f64> {
        let curr_peak = *self.curr_section_peak();
        let mut strain_peaks = mem::take(self.strain_peaks_mut());
        strain_peaks.push(curr_peak);

        strain_peaks
    }
}

pub(crate) trait OsuStrainSkill: StrainSkill + Sized {
    const REDUCED_SECTION_COUNT: usize = 10;
    const REDUCED_STRAIN_BASELINE: f64 = 0.75;
    const DIFFICULTY_MULTIPLER: f64 = 1.06;

    fn difficulty_value(&mut self) -> f64 {
        let mut difficulty = 0.0;
        let mut weight = 1.0;

        // * Sections with 0 strain are excluded to avoid worst-case time complexity of the following sort (e.g. /b/2351871).
        // * These sections will not contribute to the difficulty.
        let mut peaks = self.get_curr_strain_peaks();

        peaks.retain(|&peak| peak > 0.0);
        peaks.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

        let peak_iter = peaks.iter_mut().take(Self::REDUCED_SECTION_COUNT);

        fn lerp(start: f64, end: f64, amount: f64) -> f64 {
            start + (end - start) * amount
        }

        // * We are reducing the highest strains first to account for extreme difficulty spikes
        for (i, strain) in peak_iter.enumerate() {
            let clamped = (i as f32 / Self::REDUCED_SECTION_COUNT as f32).clamp(0.0, 1.0) as f64;
            let scale = (lerp(1.0, 10.0, clamped)).log10();
            *strain *= lerp(Self::REDUCED_STRAIN_BASELINE, 1.0, scale);
        }

        peaks.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

        // * Difficulty is the weighted sum of the highest strains from every section.
        // * We're sorting from highest to lowest strain.
        for strain in peaks {
            difficulty += strain * weight;
            weight *= Self::DECAY_WEIGHT;
        }

        self.set_raw_difficulty_value(difficulty);
        difficulty * Self::DIFFICULTY_MULTIPLER
    }

    fn strains(&self) -> &Vec<f64>;

    fn set_raw_difficulty_value(&mut self, value: f64);
    fn get_raw_difficulty_value(&self) -> f64;

    fn count_difficult_strains(&mut self) -> f64 {
        let difficulty_value = self.get_raw_difficulty_value();
        if difficulty_value == 0.0 {
            0.0
        } else {
            // * What would the top strain be if all strain values were identical
            let consistent_top_strain = difficulty_value / 10.0;

            let strains = self.strains();

            // Use a weighted sum of all strains. Constants are arbitrary and give nice values
            strains
                .iter()
                .map(|&s| 1.1 / (1.0 + (-10.0 * (s / consistent_top_strain - 0.88)).exp()))
                .sum()
        }
    }
}
