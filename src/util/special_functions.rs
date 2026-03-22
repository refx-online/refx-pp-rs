#![allow(
    clippy::excessive_precision,
    clippy::too_many_lines,
    clippy::unreadable_literal,
    clippy::many_single_char_names
)]

pub fn erf(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }

    if x == f64::INFINITY {
        return 1.0;
    }

    if x == f64::NEG_INFINITY {
        return -1.0;
    }

    if x.is_nan() {
        return f64::NAN;
    }

    // * Constants for approximation (Abramowitz and Stegun formula 7.1.26)
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let tau = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));

    let erf_val = 1.0 - tau * f64::exp(-x * x);

    if x >= 0.0 {
        erf_val
    } else {
        -erf_val
    }
}

pub fn erf_inv(mut x: f64) -> f64 {
    if x <= -1.0 {
        return f64::NEG_INFINITY;
    }

    if x >= 1.0 {
        return f64::INFINITY;
    }

    if x == 0.0 {
        return 0.0;
    }

    let a = 0.147;
    let sgn = x.signum();
    x = x.abs();

    let ln = f64::ln(1.0 - x * x);
    let t1 = 2.0 / (std::f64::consts::PI * a) + ln / 2.0;
    let t2 = ln / a;
    let base_approx = f64::sqrt(t1 * t1 - t2) - t1;

    // * Correction reduces max error from -0.005 to -0.00045.
    let c = if x >= 0.85 {
        f64::powf((x - 0.85) / 0.293, 8.0)
    } else {
        0.0
    };

    sgn * (f64::sqrt(base_approx) + c)
}
