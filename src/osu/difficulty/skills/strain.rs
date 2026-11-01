use crate::util::difficulty::logistic;

pub trait OsuHarmonicSkill {
    const HARMONIC_SCALE: f64 = 1.0;
    const DECAY_EXPONENT: f64 = 0.9;

    fn difficulty_to_performance(difficulty: f64) -> f64 {
        harmonic_difficulty_to_performance(difficulty)
    }

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
            .map(|&d| logistic(d / consistent_top_note, 0.88, 10.0, Some(1.1)))
            .sum()
    }
}

pub fn harmonic_difficulty_value(
    mut object_difficulties: Vec<f64>,
    harmonic_scale: f64,
    decay_exponent: f64,
) -> (f64, f64) {
    object_difficulties.retain(|&d| d > 0.0);

    if object_difficulties.is_empty() {
        return (0.0, 0.0);
    }

    object_difficulties.sort_by(|a, b| b.total_cmp(a));

    let mut difficulty = 0.0;
    let mut note_weight_sum = 0.0;

    for (index, &note) in object_difficulties.iter().enumerate() {
        let i = index as f64;

        // * Use a harmonic sum that considers each note of the map according to a
        //   predefined weight.
        let weight = (1.0 + (harmonic_scale / (1.0 + i)))
            / (f64::powf(i, decay_exponent) + 1.0 + (harmonic_scale / (1.0 + i)));

        note_weight_sum += weight;
        difficulty += note * weight;
    }

    (difficulty, note_weight_sum)
}

pub fn harmonic_difficulty_to_performance(difficulty: f64) -> f64 {
    4.0 * f64::powf(difficulty, 3.0)
}
