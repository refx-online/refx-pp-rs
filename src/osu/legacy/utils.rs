use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use crate::{
    Beatmap,
    model::mods::GameMods,
    osu::{
        object::{OsuObjectKind, NestedSliderObjectKind},
        convert::convert_objects,
        difficulty::scaling_factor::ScalingFactor,
    },
};

const BIG_TICK_SCORE: f64 = 30.0;
const SMALL_TICK_SCORE: f64 = 10.0;
const SPIN_SCORE: i64 = 100;
const BONUS_SPIN_SCORE: i64 = 1000;

pub const MAXIMUM_ROTATIONS_PER_SECOND: f64 = 477.0 / 60.0;
pub const MINIMUM_ROTATIONS_PER_SECOND: f64 = 3.0;

pub fn calculate_nested_score_per_object(beatmap: &Beatmap, mods: &GameMods) -> f64 {
    let object_count = beatmap.hit_objects.len();
    
    if object_count == 0 {
        return 0.0;
    }

    let map_attrs = beatmap.attributes()
        .mods(mods.clone())
        .build();
    let scaling_factor = ScalingFactor::new(map_attrs.cs);
    let time_preempt = map_attrs.hit_windows.ar * map_attrs.clock_rate;
    
    let mut attrs = crate::osu::OsuDifficultyAttributes::default();
    let osu_objects = convert_objects(
        beatmap,
        &scaling_factor,
        mods.reflection(),
        time_preempt,
        beatmap.hit_objects.len(),
        &mut attrs,
    );

    let mut amount_of_big_ticks = 0;
    let mut amount_of_small_ticks = 0;
    let mut spinner_score = 0.0;

    for obj in &osu_objects {
        match &obj.kind {
            OsuObjectKind::Slider(slider) => {
                // * 1 for head, 1 for tail
                amount_of_big_ticks += 2;
                
                // * Add slider repeats
                let repeat_count = slider.repeat_count();
                amount_of_big_ticks += repeat_count as i32;
                
                // * Count only the ticks (not repeats or tail)
                let tick_count = slider.nested_objects
                    .iter()
                    .filter(|nested| matches!(nested.kind, NestedSliderObjectKind::Tick))
                    .count();
                amount_of_small_ticks += tick_count as i32;
            }
            OsuObjectKind::Spinner(spinner) => {
                spinner_score += calculate_spinner_score(spinner.duration);
            }
            OsuObjectKind::Circle => {}
        }
    }

    let slider_score = f64::from(amount_of_big_ticks) * BIG_TICK_SCORE 
        + f64::from(amount_of_small_ticks) * SMALL_TICK_SCORE;

    (slider_score + spinner_score) / object_count as f64
}

fn calculate_spinner_score(duration_ms: f64) -> f64 {
    let seconds_duration = duration_ms / 1000.0;

    // * The total amount of half spins possible for the entire spinner.
    let total_half_spins_possible = (seconds_duration * MAXIMUM_ROTATIONS_PER_SECOND * 2.0) as i32;
    
    // * The amount of half spins that are required to successfully complete the spinner (i.e. get a 300).
    let half_spins_required_for_completion = (seconds_duration * MINIMUM_ROTATIONS_PER_SECOND) as i32;
    
    // * To be able to receive bonus points, the spinner must be rotated another 1.5 times.
    let half_spins_required_before_bonus = half_spins_required_for_completion + 3;

    let mut score: i64 = 0;

    let full_spins = total_half_spins_possible / 2;

    // * Normal spin score
    score += SPIN_SCORE * i64::from(full_spins);

    let mut bonus_spins = (total_half_spins_possible - half_spins_required_before_bonus) / 2;

    // * Reduce amount of bonus spins because we want to represent the more average case, rather than the best one.
    bonus_spins = (bonus_spins - full_spins / 2).max(0);

    score += BONUS_SPIN_SCORE * i64::from(bonus_spins);

    score as f64
}

pub fn calculate_difficulty_peppy_stars(beatmap: &Beatmap) -> i32 {
    let object_count = beatmap.hit_objects.len();
    
    if object_count == 0 {
        return 0;
    }

    let drain_length = if object_count > 0 {
        let last_obj_time = beatmap.hit_objects.last().map_or(0.0, |h| h.start_time);
        let first_obj_time = beatmap.hit_objects.first().map_or(0.0, |h| h.start_time);
        
        let break_length = beatmap.total_break_time();
        
        ((last_obj_time - first_obj_time - break_length) / 1000.0) as i32
    } else {
        0
    };

    calculate_difficulty_peppy_stars_from_params(
        beatmap.cs,
        beatmap.od,
        beatmap.hp,
        object_count,
        drain_length,
    )
}

fn calculate_difficulty_peppy_stars_from_params(
    cs: f32,
    od: f32,
    hp: f32,
    object_count: usize,
    drain_length: i32,
) -> i32 {
    /*
     * WARNING: DO NOT TOUCH IF YOU DO NOT KNOW WHAT YOU ARE DOING
     * See: https://github.com/ppy/osu/blob/0f54608ceee7ae1a284dfcb89909d4b55b3dacd1/osu.Game/Rulesets/Objects/Legacy/LegacyRulesetExtensions.cs#L66-L75
     */
    
    // TODO: Use https://docs.rs/rug/latest/rug/, for exact precision?
    //       but it's unecessarily heavy for this.
    // NOTE: Where MANTISSA = 64
    // See: https://en.wikipedia.org/wiki/Extended_precision#x86_extended_precision_format
    // let object_to_drain_ratio = if drain_length != 0 {
    //     let ratio = Float::with_val(MANTISSA, object_count)
    //         / Float::with_val(MANTISSA, drain_length)
    //         * Float::with_val(MANTISSA, 8);
    //
    //     if ratio < 0 {
    //         Float::with_val(MANTISSA, 0)
    //     } else if ratio > 16 {
    //         Float::with_val(MANTISSA, 16)
    //     } else {
    //         ratio
    //     }
    // } else {
    //     Float::with_val(MANTISSA, 16)
    // };

    // Using rust_decimal is good (close) enough.
    let object_to_drain_ratio = if drain_length != 0 {
        let ratio = Decimal::from_usize(object_count).unwrap()
            / Decimal::from_i32(drain_length).unwrap()
            * dec!(8);
        ratio.clamp(dec!(0), dec!(16))
    } else {
        dec!(16)
    };
    
    // Mimic (decimal)(double)x casting for precision
    // See: https://github.com/ppy/osu/blob/0f54608ceee7ae1a284dfcb89909d4b55b3dacd1/osu.Game/Rulesets/Objects/Legacy/LegacyRulesetExtensions.cs#L89-L91
    let drain_rate = Decimal::from_f64(f64::from(hp)).unwrap();
    let overall_difficulty = Decimal::from_f64(f64::from(od)).unwrap();
    let circle_size = Decimal::from_f64(f64::from(cs)).unwrap();
    
    let result = (drain_rate + overall_difficulty + circle_size + object_to_drain_ratio)
        / dec!(38)
        * dec!(5);
    
    result.round().to_i32().unwrap()
}

#[cfg(test)]
mod tests {
    use crate::Beatmap;

    use super::*;

    #[test]
    fn peppy_stars() {
        let map = Beatmap::from_path("./resources/2625853.osu").unwrap();

        let peppy_stars = calculate_difficulty_peppy_stars(&map);
        let expected = 3;

        assert_eq!(peppy_stars, expected);
    }
}