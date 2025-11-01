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

const MAXIMUM_ROTATIONS_PER_SECOND: f64 = 477.0 / 60.0;
const MINIMUM_ROTATIONS_PER_SECOND: f64 = 3.0;

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

    let slider_score = amount_of_big_ticks as f64 * BIG_TICK_SCORE 
        + amount_of_small_ticks as f64 * SMALL_TICK_SCORE;

    (slider_score + spinner_score) / object_count as f64
}

/// Logic borrowed from OsuLegacyScoreSimulator.simulateHit for basic score calculations.
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
    score += SPIN_SCORE * full_spins as i64;

    let mut bonus_spins = (total_half_spins_possible - half_spins_required_before_bonus) / 2;

    // * Reduce amount of bonus spins because we want to represent the more average case, rather than the best one.
    bonus_spins = (bonus_spins - full_spins / 2).max(0);

    score += BONUS_SPIN_SCORE * bonus_spins as i64;

    score as f64
}

pub fn calculate_difficulty_peppy_stars(beatmap: &Beatmap) -> i32 {
    let object_count = beatmap.hit_objects.len();
    
    if object_count == 0 {
        return 0;
    }

    let drain_length = if object_count > 0 {
        let first_obj_time = beatmap.hit_objects.first().unwrap().start_time;
        let last_obj_time = beatmap.hit_objects.last().unwrap().start_time;
        
        let break_length: i32 = beatmap.breaks
            .iter()
            .map(|b| (b.end_time.round() as i32) - (b.start_time.round() as i32))
            .sum();
        
        ((last_obj_time.round() as i32) - (first_obj_time.round() as i32) - break_length) / 1000
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
     *
     * It so happens that in stable, due to .NET Framework internals, float math would be performed
     * using x87 registers and opcodes.
     * .NET (Core) however uses SSE instructions on 32- and 64-bit words.
     * x87 registers are _80 bits_ wide. Which is notably wider than _both_ float and double.
     * Therefore, on a significant number of beatmaps, the rounding would not produce correct values.
     */

    // In Rust, we did not have direct access to 80-bit precision like .NET's `decimal` type.
    // This implementation uses f64 which should be close enough for most cases, but may have
    // slight differences on some edge case beatmaps compared to stable.
    
    let object_to_drain_ratio = if drain_length != 0 {
        ((object_count as f64 / drain_length as f64) * 8.0).clamp(0.0, 16.0)
    } else {
        16.0
    };

    let drain_rate = hp as f64;
    let overall_difficulty = od as f64;
    let circle_size = cs as f64;

    let result = (
        drain_rate + overall_difficulty + circle_size + object_to_drain_ratio
    ) / 38.0 * 5.0;

    result.round() as i32
}
