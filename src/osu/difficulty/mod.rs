use std::{cmp, pin::Pin};

use rosu_map::section::general::GameMode;
use skills::{aim::Aim, flashlight::Flashlight, speed::Speed, strain::OsuStrainSkill};

use crate::{
    any::difficulty::{skills::StrainSkill, Difficulty},
    model::{
        beatmap::BeatmapAttributes, 
        beatmap::HitWindows, 
        mode::ConvertError, 
        mods::GameMods
    },
    osu::{
        convert::convert_objects,
        difficulty::{object::OsuDifficultyObject, scaling_factor::ScalingFactor},
        object::OsuObject,
    },
    Beatmap,
};

use self::skills::OsuSkills;

use super::attributes::OsuDifficultyAttributes;

pub mod gradual;
mod object;
pub mod scaling_factor;
pub mod skills;

// * This is being adjusted to keep the final pp value scaled around what it used to be when changing things.
const PERFORMANCE_BASE_MULTIPLIER: f64 = 1.15;
const DIFFICULTY_MULTIPLIER: f64 = 0.0675;
const STAR_RATING_MULTIPLIER: f64 = 0.0265;

const HD_FADE_IN_DURATION_MULTIPLIER: f64 = 0.4;
const HD_FADE_OUT_DURATION_MULTIPLIER: f64 = 0.3;

pub fn difficulty(
    difficulty: &Difficulty,
    map: &Beatmap,
) -> Result<OsuDifficultyAttributes, ConvertError> {
    let map = map.convert_ref(GameMode::Osu, difficulty.get_mods())?;

    let DifficultyValues { skills, mut attrs } = DifficultyValues::calculate(difficulty, &map);

    let mods = difficulty.get_mods();
    let hit_windows = map
        .attributes()
        .difficulty(difficulty)
        .hit_windows();

    DifficultyValues::eval(&mut attrs, mods, &skills, &hit_windows);

    Ok(attrs)
}

pub fn calculate_difficulty_multiplier(mods: &GameMods, total_hits: u32, spinner_count: u32) -> f64 {
    let mut multiplier = PERFORMANCE_BASE_MULTIPLIER;

    if mods.so() && total_hits > 0 {
        multiplier *= 1.0 - ((spinner_count as f64 / total_hits as f64).powf(0.85));
    }

    multiplier
}

pub struct OsuDifficultySetup {
    scaling_factor: ScalingFactor,
    map_attrs: BeatmapAttributes,
    attrs: OsuDifficultyAttributes,
    time_preempt: f64,
}

impl OsuDifficultySetup {
    pub fn new(difficulty: &Difficulty, map: &Beatmap) -> Self {
        let clock_rate = difficulty.get_clock_rate();
        let map_attrs = map.attributes().difficulty(difficulty).build();
        let scaling_factor = ScalingFactor::new(map_attrs.cs);

        let attrs = OsuDifficultyAttributes {
            ar: map_attrs.ar,
            hp: map_attrs.hp,
            great_hit_window: map_attrs.hit_windows.od_great,
            ok_hit_window: map_attrs.hit_windows.od_ok.unwrap_or(0.0),
            meh_hit_window: map_attrs.hit_windows.od_meh.unwrap_or(0.0),
            ..Default::default()
        };

        let time_preempt = f64::from((map_attrs.hit_windows.ar * clock_rate) as f32);

        Self {
            scaling_factor,
            map_attrs,
            attrs,
            time_preempt,
        }
    }
}

pub struct DifficultyValues {
    pub skills: OsuSkills,
    pub attrs: OsuDifficultyAttributes,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &Difficulty, map: &Beatmap) -> Self {
        let mods = difficulty.get_mods();
        let take = difficulty.get_passed_objects();

        let OsuDifficultySetup {
            scaling_factor,
            map_attrs,
            mut attrs,
            time_preempt,
        } = OsuDifficultySetup::new(difficulty, map);

        let mut osu_objects = convert_objects(
            map,
            &scaling_factor,
            mods.reflection(),
            time_preempt,
            take,
            &mut attrs,
        );

        let osu_object_iter = osu_objects.iter_mut().map(Pin::new);

        let diff_objects =
            Self::create_difficulty_objects(difficulty, &scaling_factor, osu_object_iter);

        let mut skills = OsuSkills::new(mods, &scaling_factor, &map_attrs, time_preempt);

        // The first hit object has no difficulty object
        let take_diff_objects = cmp::min(map.hit_objects.len(), take).saturating_sub(1);

        for hit_object in diff_objects.iter().take(take_diff_objects) {
            skills.process(hit_object, &diff_objects);
        }

        Self { skills, attrs }
    }

    /// Process the difficulty values and store the results in `attrs`.
    pub fn eval(attrs: &mut OsuDifficultyAttributes, mods: &GameMods, skills: &OsuSkills, hit_windows: &HitWindows) {
        let OsuSkills {
            aim,
            aim_no_sliders,
            speed,
            flashlight,
        } = skills;

        let aim_difficulty_value = aim.cloned_difficulty_value();

        let aim_difficult_strain_count = aim.count_top_weighted_strains(aim_difficulty_value);

        let difficult_sliders = aim.get_difficult_sliders();
        
        let speed_difficulty_value = speed.cloned_difficulty_value();
        let speed_difficult_strain_count = speed.count_top_weighted_strains(speed_difficulty_value);

        let aim_no_sliders_difficulty_value = aim_no_sliders.cloned_difficulty_value();

        let aim_no_sliders_top_weighted_slider_count = aim_no_sliders.count_top_weighted_sliders();
        let aim_no_sliders_difficult_strain_count = aim_no_sliders.count_top_weighted_strains(aim_no_sliders_difficulty_value);

        let aim_top_weighted_slider_factor = 
            aim_no_sliders_top_weighted_slider_count
            / f64::max(
                1.0,
                aim_no_sliders_difficult_strain_count - aim_no_sliders_top_weighted_slider_count,
            );
        
        let speed_top_weighted_slider_factor = speed.count_top_weighted_sliders();

        let speed_top_weighted_slider_factor = 
            speed_top_weighted_slider_factor
            / f64::max(
                1.0,
                speed_difficult_strain_count - speed_top_weighted_slider_factor,
            );

        let flashlight_difficulty_value = flashlight.cloned_difficulty_value();

        let total_hits = attrs.n_circles + attrs.n_sliders + attrs.n_spinners;
        let spinner_count = attrs.n_spinners;

        let preempt = hit_windows.ar;

        let approach_rate = if preempt > 1200.0 {
            (1800.0 - preempt) / 120.0
        } else {
            (1200.0 - preempt) / 150.0 + 5.0
        };
        
        let overall_difficulty = (80.0 - hit_windows.od_great) / 6.0;

        let aim_rating = Self::compute_aim_rating(
            aim_difficulty_value,
            mods,
            total_hits,
            approach_rate,
            overall_difficulty,
        );

        let aim_rating_no_sliders = Self::compute_aim_rating(
            aim_no_sliders.cloned_difficulty_value(),
            mods,
            total_hits,
            approach_rate,
            overall_difficulty,
        );

        let speed_rating = Self::compute_speed_rating(
            speed_difficulty_value,
            mods,
            total_hits,
            approach_rate,
            overall_difficulty,
        );

        let flashlight_rating = Self::compute_flashlight_rating(
            flashlight_difficulty_value,
            mods,
            total_hits,
            overall_difficulty,
        );

        let slider_factor = if aim_rating > 0.0 {
            aim_rating_no_sliders / aim_rating
        } else {
            1.0
        };

        let base_aim_performance = Aim::difficulty_to_performance(aim_rating);
        let base_speed_performance = Speed::difficulty_to_performance(speed_rating);
        let base_flashlight_performance = Flashlight::difficulty_to_performance(flashlight_rating);

        let base_performance = ((base_aim_performance).powf(1.1) 
            + (base_speed_performance).powf(1.1) 
            + (base_flashlight_performance).powf(1.1))
            .powf(1.0 / 1.1);

        let multiplier =
            calculate_difficulty_multiplier(mods, total_hits, spinner_count);

        let star_rating = if base_performance > 0.00001 {
            multiplier.cbrt() * STAR_RATING_MULTIPLIER 
                * ((100_000.0 / 2.0_f64.powf(1.0 / 1.1) * base_performance).cbrt() + 4.0)
        } else {
            0.0
        };

        attrs.aim = aim_rating;
        attrs.aim_difficult_slider_count = difficult_sliders;
        attrs.speed = speed_rating;
        attrs.flashlight = flashlight_rating;
        attrs.slider_factor = slider_factor;
        attrs.aim_difficult_strain_count = aim_difficult_strain_count;
        attrs.speed_difficult_strain_count = speed_difficult_strain_count;
        attrs.aim_top_weighted_slider_factor = aim_top_weighted_slider_factor;
        attrs.speed_top_weighted_slider_factor = speed_top_weighted_slider_factor;
        attrs.stars = star_rating;
        attrs.speed_note_count = speed.relevant_note_count();
    }

    fn compute_aim_rating(
        aim_difficulty_value: f64,
        mods: &GameMods,
        total_hits: u32,
        approach_rate: f64,
        overall_difficulty: f64,
    ) -> f64 {
        if mods.ap() {
            return 0.0;
        }

        let mut aim_rating = f64::sqrt(aim_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if mods.td() {
            aim_rating = aim_rating.powf(0.8);
        }

        if mods.mg() {
            let magnetised_strength = mods.attraction_strength().unwrap_or(0.0);
            aim_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, total_hits as f64 / 2000.0)
            + if total_hits > 2000 {
                f64::log10(total_hits as f64 / 2000.0) * 0.5
            } else {
                0.0
            };

        let ar_factor = if approach_rate > 10.33 {
            0.3 * (approach_rate - 10.33)
        } else if approach_rate < 8.0 {
            0.05 * (8.0 - approach_rate)
        } else {
            0.0
        };

        rating_multiplier *= 1.0 + ar_factor * approach_rate_length_bonus;

        if mods.hd() {
            // * We want to give more reward for lower AR when it comes to aim and HD. This nerfs high AR and buffs lower AR.
            rating_multiplier *= 1.0 + 0.04 * (12.0 - approach_rate);
        }

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, overall_difficulty).powf(2.0) / 2500.0;

        aim_rating * rating_multiplier.cbrt()
    }

    fn compute_speed_rating(
        speed_difficulty_value: f64,
        mods: &GameMods,
        total_hits: u32,
        approach_rate: f64,
        overall_difficulty: f64,
    ) -> f64 {
        let mut speed_rating = f64::sqrt(speed_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if mods.ap() {
            speed_rating *= 0.5;
        }

        if mods.mg() {
            // * Reduce speed rating because of the speed distance scaling, with maximum reduction being 0.7x
            let magnetised_strength = mods.attraction_strength().unwrap_or(0.0);
            speed_rating *= 1.0 - magnetised_strength * 0.3;
        }

        let mut rating_multiplier = 1.0;

        let approach_rate_length_bonus = 0.95 + 0.4 * f64::min(1.0, total_hits as f64 / 2000.0)
            + if total_hits > 2000 {
                f64::log10(total_hits as f64 / 2000.0) * 0.5
            } else {
                0.0
            };

        let ar_factor = if mods.ap() {
            0.0
        } else if approach_rate > 10.33 {
            0.3 * (approach_rate - 10.33)
        } else {
            0.0
        };

        rating_multiplier *= 1.0 + ar_factor * approach_rate_length_bonus;

        if mods.hd() {
            // * We want to give more reward for lower AR when it comes to aim and HD. This nerfs high AR and buffs lower AR.
            rating_multiplier *= 1.0 + 0.04 * (12.0 - approach_rate);
        }

        rating_multiplier *= 0.95 + f64::max(0.0, overall_difficulty).powf(2.0) / 750.0;

        speed_rating * rating_multiplier.cbrt()
    }

    fn compute_flashlight_rating(
        flashlight_difficulty_value: f64,
        mods: &GameMods,
        total_hits: u32,
        overall_difficulty: f64,
    ) -> f64 {
        if !mods.fl() {
            return 0.0;
        }

        let mut flashlight_rating = f64::sqrt(flashlight_difficulty_value) * DIFFICULTY_MULTIPLIER;

        if mods.td() {
            flashlight_rating = flashlight_rating.powf(0.8);
        }

        if mods.ap() {
            flashlight_rating *= 0.4;
        }

        if mods.mg() {
            let magnetised_strength = mods.attraction_strength().unwrap_or(0.0);
            flashlight_rating *= 1.0 - magnetised_strength;
        }

        let mut rating_multiplier = 1.0;

        // * Account for shorter maps having a higher ratio of 0 combo/100 combo flashlight radius.
        rating_multiplier *= 0.7 + 0.1 * f64::min(1.0, total_hits as f64 / 200.0)
            + if total_hits > 200 {
                0.2 * f64::min(1.0, (total_hits - 200) as f64 / 200.0)
            } else {
                0.0
            };

        // * It is important to consider accuracy difficulty when scaling with accuracy.
        rating_multiplier *= 0.98 + f64::max(0.0, overall_difficulty).powf(2.0) / 2500.0;

        flashlight_rating * rating_multiplier.sqrt()
    }

    pub fn create_difficulty_objects<'a>(
        difficulty: &Difficulty,
        scaling_factor: &ScalingFactor,
        osu_objects: impl ExactSizeIterator<Item = Pin<&'a mut OsuObject>>,
    ) -> Vec<OsuDifficultyObject<'a>> {
        let take = difficulty.get_passed_objects();
        let clock_rate = difficulty.get_clock_rate();

        let mut osu_objects_iter = osu_objects
            .map(|h| OsuDifficultyObject::compute_slider_cursor_pos(h, scaling_factor.radius))
            .map(Pin::into_ref);

        let Some(mut last) = osu_objects_iter.next().filter(|_| take > 0) else {
            return Vec::new();
        };

        let mut last_last = None;

        osu_objects_iter
            .enumerate()
            .map(|(idx, h)| {
                let diff_object = OsuDifficultyObject::new(
                    h.get_ref(),
                    last.get_ref(),
                    last_last.as_deref(),
                    clock_rate,
                    idx,
                    scaling_factor,
                );

                last_last = Some(last);
                last = h;

                diff_object
            })
            .collect()
    }
}
