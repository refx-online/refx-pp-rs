use std::f64::consts::E;

pub const fn bpm_to_milliseconds(bpm: f64, delimiter: Option<i32>) -> f64 {
    60_000.0 / i32_unwrap_or(delimiter, 4) as f64 / bpm
}

pub const fn milliseconds_to_bpm(ms: f64, delimiter: Option<i32>) -> f64 {
    60_000.0 / (ms * i32_unwrap_or(delimiter, 4) as f64)
}

// `Option::unwrap_or` is not const
const fn i32_unwrap_or(option: Option<i32>, default: i32) -> i32 {
    match option {
        Some(value) => value,
        None => default,
    }
}

// `f64::exp` is not const
pub fn logistic(x: f64, midpoint_offset: f64, multiplier: f64, max_value: Option<f64>) -> f64 {
    max_value.unwrap_or(1.0) / (1.0 + f64::exp(multiplier * (midpoint_offset - x)))
}

// `f64::exp` is not const
pub fn logistic_exp(exp: f64, max_value: Option<f64>) -> f64 {
    max_value.unwrap_or(1.0) / (1.0 + f64::exp(exp))
}

pub fn norm<const N: usize>(p: f64, values: [f64; N]) -> f64 {
    values
        .into_iter()
        .map(|x| f64::powf(x, p))
        .sum::<f64>()
        .powf(p.recip())
}

pub fn bell_curve(x: f64, mean: f64, width: f64, multiplier: Option<f64>) -> f64 {
    multiplier.unwrap_or(1.0) * f64::exp(E * -(f64::powf(x - mean, 2.0) / f64::powf(width, 2.0)))
}

pub const fn smoothstep(x: f64, start: f64, end: f64) -> f64 {
    let x = reverse_lerp(x, start, end);

    x * x * (3.0 - 2.0 * x)
}

pub const fn smootherstep(x: f64, start: f64, end: f64) -> f64 {
    let x = reverse_lerp(x, start, end);

    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

pub fn smoothstep_bell_curve(x: f64, mean: f64, width: f64) -> f64 {
    let x = x - mean;
    let x = if x > 0.0 { width - x } else { width + x };

    smoothstep(x, 0.0, width)
}

pub const fn reverse_lerp(x: f64, start: f64, end: f64) -> f64 {
    f64::clamp((x - start) / (end - start), 0.0, 1.0)
}

pub fn count_top_weighted_sliders(slider_strains: &[f64], difficulty_value: f64) -> f64 {
    if slider_strains.is_empty() {
        return 0.0;
    }

    // * What would the top strain be if all strain values were identical
    let consistent_top_strain = difficulty_value / 10.0;
    if consistent_top_strain == 0.0 {
        return 0.0;
    }

    // * Use a weighted sum of all strains. Constants are arbitrary and give nice values
    slider_strains
        .iter()
        .map(|&s| logistic(s / consistent_top_strain, 0.88, 10.0, Some(1.1)))
        .sum()
}