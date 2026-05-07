// Relax-specific aim evaluator for refx pp (ported from rosu-ccv3-pp).
//
// Uses the rosu aim formula base (smoothstep/smootherstep,
// adjusted_delta_time, wiggle_bonus, small_circle_bonus,
// DIAMETER-based gating).

use std::f64::consts::PI;

use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::difficulty::object::OsuDifficultyObject,
    util::{
        difficulty::{milliseconds_to_bpm, reverse_lerp, smootherstep, smoothstep},
        float_ext::FloatExt,
    },
};

pub struct AimRxEvaluator;

// ─── Windowed statistics helpers ────────────────────────────────────

const ANGLE_WINDOW: usize = 8;

fn windowed_angle_stats<'a>(
    curr: &'a OsuDifficultyObject<'a>,
    diff_objects: &'a [OsuDifficultyObject<'a>],
    window: usize,
) -> (f64, f64, usize) {
    let mut angles: Vec<f64> = Vec::with_capacity(window + 1);

    if let Some(a) = curr.angle {
        angles.push(a);
    }
    for back in 0..window {
        if let Some(prev) = curr.previous(back, diff_objects) {
            if let Some(a) = prev.angle {
                angles.push(a);
            }
        } else {
            break;
        }
    }

    let n = angles.len();
    if n < 3 {
        return (0.0, 0.0, n);
    }

    let mean: f64 = angles.iter().sum::<f64>() / n as f64;
    let variance: f64 = angles.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / n as f64;
    (mean, variance.sqrt(), n)
}

fn windowed_dist_stats<'a>(
    curr: &'a OsuDifficultyObject<'a>,
    diff_objects: &'a [OsuDifficultyObject<'a>],
    window: usize,
) -> (f64, f64, usize) {
    let mut dists: Vec<f64> = Vec::with_capacity(window + 1);
    dists.push(curr.lazy_jump_dist);

    for back in 0..window {
        if let Some(prev) = curr.previous(back, diff_objects) {
            dists.push(prev.lazy_jump_dist);
        } else {
            break;
        }
    }

    let n = dists.len();
    if n < 2 {
        return (0.0, 0.0, n);
    }
    let mean = dists.iter().sum::<f64>() / n as f64;
    let var = dists.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n as f64;
    (mean, var.sqrt(), n)
}

fn windowed_vel_stats<'a>(
    curr: &'a OsuDifficultyObject<'a>,
    diff_objects: &'a [OsuDifficultyObject<'a>],
    window: usize,
) -> (f64, f64, usize) {
    let mut vels: Vec<f64> = Vec::with_capacity(window + 1);
    if curr.adjusted_delta_time > 0.0 {
        vels.push(curr.lazy_jump_dist / curr.adjusted_delta_time);
    }

    for back in 0..window {
        if let Some(prev) = curr.previous(back, diff_objects) {
            if prev.adjusted_delta_time > 0.0 {
                vels.push(prev.lazy_jump_dist / prev.adjusted_delta_time);
            }
        } else {
            break;
        }
    }

    let n = vels.len();
    if n < 2 {
        return (0.0, 0.0, n);
    }
    let mean = vels.iter().sum::<f64>() / n as f64;
    let var = vels.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    (mean, var.sqrt(), n)
}

/// Detect N/X alternating patterns: look at consecutive angle pairs
/// and check if they alternate between two values (±tolerance).
fn detect_nx_pattern<'a>(
    curr: &'a OsuDifficultyObject<'a>,
    diff_objects: &'a [OsuDifficultyObject<'a>],
    window: usize,
) -> f64 {
    let mut angles: Vec<f64> = Vec::with_capacity(window + 1);
    if let Some(a) = curr.angle {
        angles.push(a);
    }
    for back in 0..window {
        if let Some(prev) = curr.previous(back, diff_objects) {
            if let Some(a) = prev.angle {
                angles.push(a);
            }
        } else {
            break;
        }
    }

    if angles.len() < 4 {
        return 0.0;
    }

    // Check alternation: angles should alternate between two clusters.
    // even[i] ≈ even[j], odd[i] ≈ odd[j], but even ≠ odd.
    let evens: Vec<f64> = angles.iter().step_by(2).copied().collect();
    let odds: Vec<f64> = angles.iter().skip(1).step_by(2).copied().collect();

    if evens.len() < 2 || odds.len() < 2 {
        return 0.0;
    }

    let even_mean = evens.iter().sum::<f64>() / evens.len() as f64;
    let odd_mean = odds.iter().sum::<f64>() / odds.len() as f64;

    let even_var = evens.iter().map(|a| (a - even_mean).powi(2)).sum::<f64>() / evens.len() as f64;
    let odd_var = odds.iter().map(|a| (a - odd_mean).powi(2)).sum::<f64>() / odds.len() as f64;

    let even_stddev = even_var.sqrt();
    let odd_stddev = odd_var.sqrt();

    // Both clusters must be tight (stddev < 0.25 rad each)
    // AND the two clusters must be different (|mean_diff| > 0.3 rad)
    let cluster_tight = even_stddev < 0.25 && odd_stddev < 0.25;
    let clusters_differ = (even_mean - odd_mean).abs() > 0.3;

    if cluster_tight && clusters_differ {
        // Strength: how tight × how different
        let tightness = 1.0 - ((even_stddev + odd_stddev) / 0.50).clamp(0.0, 1.0);
        let separation = ((even_mean - odd_mean).abs() / PI).clamp(0.0, 1.0);
        tightness * separation
    } else {
        0.0
    }
}

impl AimRxEvaluator {
    // Constants tuned against akat-based ccv3 RX evaluator, accounting for
    // rosu's wider angle bonus shapes (smoothstep vs sin²) and skill multiplier
    // difference (26.0 vs 25.18):
    //
    //   Wide:  1.56/1.45 × 1.5 × 0.968 = 1.56
    //   Acute: 2.05/1.90 × 2.55 × 0.968 = 2.66
    //   Slider: 1.20/1.35 × 1.35 × 0.968 = 1.16
    //   VelCh: 0.78/0.70 × 0.75 × 0.968 = 0.81
    const WIDE_ANGLE_MULTIPLIER: f64 = 1.56;
    const ACUTE_ANGLE_MULTIPLIER: f64 = 2.66;
    const SLIDER_MULTIPLIER: f64 = 1.16;
    const VELOCITY_CHANGE_MULTIPLIER: f64 = 0.81;
    const WIGGLE_MULTIPLIER: f64 = 1.02;

    // Final RX calibration scalar for refx vanilla RX scaling.
    const RX_CALIBRATION: f64 = 0.92;

    const SLOW_SLIDER_VEL_FLOOR: f64 = 0.55;

    const CONSTANT_DIST_RATIO: f64 = 0.18;
    const EDGE_TO_EDGE_THRESHOLD: f64 = 400.0;
    const CONSTANT_DIST_BPM_STRAIN_TIME: f64 = 85.7;

    // N/X pattern nerf: max cut when a strong alternating pattern is
    // detected at low BPM. Fades at high BPM (genuinely hard to hold).
    const NX_MAX_NERF: f64 = 0.30;

    // Aim slop: max nerf for constant-everything patterns.
    const SLOP_MAX_NERF: f64 = 0.35;
    const STACK_NERF_DECAY_PER_EXTRA: f64 = 0.12;
    const STACK_NERF_DECAY_MIN: f64 = 0.70;

    // Tech boost: max bonus for high-variance patterns.
    const TECH_MAX_BOOST: f64 = 0.08;

    fn nerf_stack_decay(active_nerf_count: usize) -> f64 {
        if active_nerf_count <= 1 {
            1.0
        } else {
            (1.0 - Self::STACK_NERF_DECAY_PER_EXTRA * (active_nerf_count - 1) as f64)
                .max(Self::STACK_NERF_DECAY_MIN)
        }
    }

    pub fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
        with_slider_travel_dist: bool,
    ) -> f64 {
        let osu_curr_obj = curr;

        let Some((osu_last_last_obj, osu_last_obj)) = curr
            .previous(1, diff_objects)
            .zip(curr.previous(0, diff_objects))
            .filter(|(_, last)| !(curr.base.is_spinner() || last.base.is_spinner()))
        else {
            return 0.0;
        };

        const RADIUS: i32 = OsuDifficultyObject::NORMALIZED_RADIUS;
        const DIAMETER: i32 = OsuDifficultyObject::NORMALIZED_DIAMETER;

        // ── Velocities ──────────────────────────────────────────────
        let mut curr_vel = osu_curr_obj.lazy_jump_dist / osu_curr_obj.adjusted_delta_time;

        if osu_last_obj.base.is_slider() && with_slider_travel_dist {
            let travel_vel = osu_last_obj.travel_dist / osu_last_obj.travel_time;
            let movement_vel = osu_curr_obj.min_jump_dist / osu_curr_obj.min_jump_time;
            curr_vel = curr_vel.max(movement_vel + travel_vel);
        }

        let mut prev_vel = osu_last_obj.lazy_jump_dist / osu_last_obj.adjusted_delta_time;

        if osu_last_last_obj.base.is_slider() && with_slider_travel_dist {
            let travel_vel = osu_last_last_obj.travel_dist / osu_last_last_obj.travel_time;
            let movement_vel = osu_last_obj.min_jump_dist / osu_last_obj.min_jump_time;
            prev_vel = prev_vel.max(movement_vel + travel_vel);
        }

        let mut wide_angle_bonus = 0.0;
        let mut acute_angle_bonus = 0.0;
        let mut slider_bonus = 0.0;
        let mut vel_change_bonus = 0.0;
        let mut wiggle_bonus = 0.0;

        let mut aim_strain = curr_vel;

        // ── Angle bonuses ───────────────────────────────────────────
        if let Some((curr_angle, last_angle)) = osu_curr_obj.angle.zip(osu_last_obj.angle) {
            let angle_bonus = curr_vel.min(prev_vel);

            if osu_curr_obj
                .adjusted_delta_time
                .max(osu_last_obj.adjusted_delta_time)
                < 1.25
                    * osu_curr_obj
                        .adjusted_delta_time
                        .min(osu_last_obj.adjusted_delta_time)
            {
                acute_angle_bonus = Self::calc_acute_angle_bonus(curr_angle);

                acute_angle_bonus *= 0.08
                    + 0.92
                        * (1.0
                            - f64::min(
                                acute_angle_bonus,
                                f64::powf(Self::calc_acute_angle_bonus(last_angle), 3.0),
                            ));

                acute_angle_bonus *= angle_bonus
                    * smootherstep(
                        milliseconds_to_bpm(osu_curr_obj.adjusted_delta_time, Some(2)),
                        300.0,
                        400.0,
                    )
                    * smootherstep(
                        osu_curr_obj.lazy_jump_dist,
                        f64::from(DIAMETER),
                        f64::from(DIAMETER * 2),
                    );
            }

            wide_angle_bonus = Self::calc_wide_angle_bonus(curr_angle);

            // ── Windowed variance repetition ────────────────────────
            let eff_bpm = 30_000.0 / osu_curr_obj.adjusted_delta_time;
            let high_bpm_t = ((eff_bpm - 410.0) / 90.0).clamp(0.0, 1.0);

            let (_win_mean, win_stddev, win_n) =
                windowed_angle_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);

            let variance_factor = if win_n >= 3 {
                (win_stddev / 1.2).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let rep_strength = 1.0 - variance_factor;

            let wide_penalty = rep_strength * (1.0 - high_bpm_t);
            let wide_rep_buff = high_bpm_t * 0.15;
            wide_angle_bonus *=
                angle_bonus * smootherstep(osu_curr_obj.lazy_jump_dist, 0.0, f64::from(DIAMETER))
                * (1.0 - wide_penalty + wide_rep_buff).max(0.0);

            wiggle_bonus = angle_bonus
                * smootherstep(
                    osu_curr_obj.lazy_jump_dist,
                    f64::from(RADIUS),
                    f64::from(DIAMETER),
                )
                * f64::powf(
                    reverse_lerp(
                        osu_curr_obj.lazy_jump_dist,
                        f64::from(DIAMETER * 3),
                        f64::from(DIAMETER),
                    ),
                    1.8,
                )
                * smootherstep(curr_angle, f64::to_radians(110.0), f64::to_radians(60.0))
                * smootherstep(
                    osu_last_obj.lazy_jump_dist,
                    f64::from(RADIUS),
                    f64::from(DIAMETER),
                )
                * f64::powf(
                    reverse_lerp(
                        osu_last_obj.lazy_jump_dist,
                        f64::from(DIAMETER * 3),
                        f64::from(DIAMETER),
                    ),
                    1.8,
                )
                * smootherstep(last_angle, f64::to_radians(110.0), f64::to_radians(60.0));

            if let Some(osu_last_2_obj) = curr.previous(2, diff_objects) {
                let distance =
                    (osu_last_2_obj.base.stacked_pos() - osu_last_obj.base.stacked_pos()).length();
                if distance < 1.0 {
                    wide_angle_bonus *= 1.0 - 0.35 * f64::from(1.0 - distance);
                }
            }
        }

        // ── Velocity change bonus ───────────────────────────────────
        if prev_vel.max(curr_vel).not_eq(0.0) {
            prev_vel = (osu_last_obj.lazy_jump_dist + osu_last_last_obj.travel_dist)
                / osu_last_obj.adjusted_delta_time;
            curr_vel = (osu_curr_obj.lazy_jump_dist + osu_last_obj.travel_dist)
                / osu_curr_obj.adjusted_delta_time;

            let dist_ratio = smoothstep(
                (prev_vel - curr_vel).abs() / prev_vel.max(curr_vel),
                0.0,
                1.0,
            );

            let overlap_vel_buff = (f64::from(DIAMETER) * 1.25
                / osu_curr_obj
                    .adjusted_delta_time
                    .min(osu_last_obj.adjusted_delta_time))
            .min((prev_vel - curr_vel).abs());

            vel_change_bonus = overlap_vel_buff * dist_ratio;

            let bonus_base = osu_curr_obj
                .adjusted_delta_time
                .min(osu_last_obj.adjusted_delta_time)
                / osu_curr_obj
                    .adjusted_delta_time
                    .max(osu_last_obj.adjusted_delta_time);
            vel_change_bonus *= bonus_base.powf(2.0);
        }

        // ── Slider bonus with slow-slider taper ─────────────────────
        if osu_last_obj.base.is_slider() {
            let travel_vel = osu_last_obj.travel_dist / osu_last_obj.travel_time;
            slider_bonus = travel_vel;

            if travel_vel < Self::SLOW_SLIDER_VEL_FLOOR {
                let ratio = (travel_vel / Self::SLOW_SLIDER_VEL_FLOOR).clamp(0.0, 1.0);
                slider_bonus *= 0.55 + 0.45 * ratio;
            }
        }

        // ── Combine ─────────────────────────────────────────────────
        aim_strain += wiggle_bonus * Self::WIGGLE_MULTIPLIER;
        aim_strain += vel_change_bonus * Self::VELOCITY_CHANGE_MULTIPLIER;

        aim_strain += (acute_angle_bonus * Self::ACUTE_ANGLE_MULTIPLIER)
            .max(wide_angle_bonus * Self::WIDE_ANGLE_MULTIPLIER);

        aim_strain *= osu_curr_obj.small_circle_bonus;

        if with_slider_travel_dist {
            aim_strain += slider_bonus * Self::SLIDER_MULTIPLIER;
        }

        // ═════════════════════════════════════════════════════════════
        // Relax RX-specific nerfs and boosts (post-combine)
        // ═════════════════════════════════════════════════════════════

        let eff_bpm = 30_000.0 / osu_curr_obj.adjusted_delta_time;
        let mut active_nerf_count = 0;
        let mut nx_nerf = 0.0;
        let mut slop_nerf = 0.0;
        let mut cross_screen_nerf = 0.0;

        // ── N/X alternating pattern nerf ─────────────────────────────
        // N patterns (zigzag) and X patterns (crossover) are trivial on
        // RX because the cursor just bounces between two positions.
        // Detection: check if consecutive angles alternate between two
        // tight clusters. Nerf fades at high BPM (genuinely hard).
        {
            let nx_strength = detect_nx_pattern(osu_curr_obj, diff_objects, ANGLE_WINDOW);

            if nx_strength > 0.05 {
                // Also check distance consistency — true N/X has constant spacing
                let (dist_mean, dist_stddev, dist_n) =
                    windowed_dist_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);

                let dist_cv = if dist_mean > 0.0 && dist_n >= 3 {
                    dist_stddev / dist_mean
                } else {
                    1.0
                };

                // Constant distance (low CV) amplifies the nerf
                let dist_consistency = (1.0 - (dist_cv / 0.20).clamp(0.0, 1.0)).max(0.0);

                // BPM fade: full nerf below 350, fades to 0 at 500+
                let bpm_fade = 1.0 - ((eff_bpm - 350.0) / 150.0).clamp(0.0, 1.0);

                let nx_severity = nx_strength * dist_consistency * bpm_fade;
                nx_nerf = Self::NX_MAX_NERF * nx_severity;
                if nx_nerf > 0.0 {
                    active_nerf_count += 1;
                }
            }
        }

        // ── Aim slop detection ──────────────────────────────────────
        // Constant velocity + constant distance + low angle variance
        // = mechanical slop. The cursor follows a predictable path.
        // This catches patterns that N/X detection misses (e.g. linear
        // back-and-forth, square patterns, constant-speed circles).
        {
            let (_, angle_stddev, angle_n) =
                windowed_angle_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);
            let (dist_mean, dist_stddev, dist_n) =
                windowed_dist_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);
            let (vel_mean, vel_stddev, vel_n) =
                windowed_vel_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);

            if angle_n >= 4 && dist_n >= 4 && vel_n >= 4 {
                // Angle uniformity: stddev < 0.3 = highly uniform
                let angle_uniformity = (1.0 - (angle_stddev / 0.3).clamp(0.0, 1.0)).max(0.0);

                // Distance uniformity: CV < 0.15 = very constant spacing
                let dist_cv = if dist_mean > 0.0 { dist_stddev / dist_mean } else { 1.0 };
                let dist_uniformity = (1.0 - (dist_cv / 0.15).clamp(0.0, 1.0)).max(0.0);

                // Velocity uniformity: CV < 0.15 = constant speed
                let vel_cv = if vel_mean > 0.0 { vel_stddev / vel_mean } else { 1.0 };
                let vel_uniformity = (1.0 - (vel_cv / 0.15).clamp(0.0, 1.0)).max(0.0);

                // All three must be uniform for the nerf to fire
                let slop_severity = angle_uniformity * dist_uniformity * vel_uniformity;

                // BPM fade: less severe at high BPM
                let bpm_fade = 1.0 - ((eff_bpm - 400.0) / 150.0).clamp(0.0, 1.0);

                slop_nerf = Self::SLOP_MAX_NERF * slop_severity * bpm_fade;
                if slop_nerf > 0.0 {
                    active_nerf_count += 1;
                }
            }
        }

        // ── Tech pattern boost ──────────────────────────────────────
        // High angle variance + high velocity change = genuine tech.
        // These patterns are hard on RX because the cursor must react
        // to unpredictable movements. Small boost to offset the global
        // nerfs that might accidentally clip tech sections.
        {
            let (_, angle_stddev, angle_n) =
                windowed_angle_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);
            let (vel_mean, vel_stddev, vel_n) =
                windowed_vel_stats(osu_curr_obj, diff_objects, ANGLE_WINDOW);

            if angle_n >= 4 && vel_n >= 4 {
                // High angle variance: stddev > 0.6 = varied
                let angle_variety = ((angle_stddev - 0.6) / 0.4).clamp(0.0, 1.0);

                // High velocity variance: CV > 0.25 = real speed changes
                let vel_cv = if vel_mean > 0.0 { vel_stddev / vel_mean } else { 0.0 };
                let vel_variety = ((vel_cv - 0.25) / 0.25).clamp(0.0, 1.0);

                let tech_signal = angle_variety * vel_variety;
                aim_strain *= 1.0 + Self::TECH_MAX_BOOST * tech_signal;
            }
        }

        // ── Cross-screen constant-distance nerf ─────────────────────
        if osu_curr_obj.adjusted_delta_time >= Self::CONSTANT_DIST_BPM_STRAIN_TIME {
            let curr_d = osu_curr_obj.lazy_jump_dist;
            let prev_d = osu_last_obj.lazy_jump_dist;
            let max_d = curr_d.max(prev_d);
            let min_d = curr_d.min(prev_d);

            if max_d > 80.0 {
                let change_ratio = if max_d > 0.0 {
                    (max_d - min_d) / max_d
                } else {
                    1.0
                };

                let is_edge_to_edge = max_d >= Self::EDGE_TO_EDGE_THRESHOLD;

                if !is_edge_to_edge && change_ratio < Self::CONSTANT_DIST_RATIO {
                    let ratio_factor = 1.0 - (change_ratio / Self::CONSTANT_DIST_RATIO);
                    let dist_factor = 1.0
                        - ((max_d - 80.0) / (Self::EDGE_TO_EDGE_THRESHOLD - 80.0))
                            .clamp(0.0, 1.0);
                    let severity = ratio_factor * dist_factor;
                    cross_screen_nerf = 0.15 * severity;
                    if cross_screen_nerf > 0.0 {
                        active_nerf_count += 1;
                    }
                }
            }
        }

        let nerf_stack_decay = Self::nerf_stack_decay(active_nerf_count);
        if nx_nerf > 0.0 {
            aim_strain *= 1.0 - nx_nerf * nerf_stack_decay;
        }
        if slop_nerf > 0.0 {
            aim_strain *= 1.0 - slop_nerf * nerf_stack_decay;
        }
        if cross_screen_nerf > 0.0 {
            aim_strain *= 1.0 - cross_screen_nerf * nerf_stack_decay;
        }

        // ── Final RX calibration ────────────────────────────────────
        aim_strain *= Self::RX_CALIBRATION;

        aim_strain
    }

    const fn calc_wide_angle_bonus(angle: f64) -> f64 {
        smoothstep(angle, f64::to_radians(40.0), f64::to_radians(140.0))
    }

    const fn calc_acute_angle_bonus(angle: f64) -> f64 {
        smoothstep(angle, f64::to_radians(140.0), f64::to_radians(40.0))
    }
}