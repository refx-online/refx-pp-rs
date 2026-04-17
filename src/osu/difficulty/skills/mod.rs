use self::{aim::Aim, flashlight::Flashlight, reading::Reading, speed::Speed};
use super::{object::OsuDifficultyObject, scaling_factor::ScalingFactor};
use crate::{
    any::difficulty::skills::{HarmonicSkill, StrainSkill, VariableLengthStrainSkill},
    model::{beatmap::BeatmapAttributes, mods::GameMods},
};

pub mod aim;
pub mod flashlight;
pub mod reading;
pub mod speed;
pub mod strain;

pub struct OsuSkills {
    pub aim: Aim,
    pub aim_no_sliders: Aim,
    pub speed: Speed,
    pub flashlight: Flashlight,
    pub reading: Reading,
}

impl OsuSkills {
    pub fn new(
        mods: &GameMods,
        scaling_factor: &ScalingFactor,
        map_attrs: &BeatmapAttributes,
    ) -> Self {
        let hit_window = 2.0 * map_attrs.hit_windows.od_great;

        let radius = scaling_factor.radius;

        let aim = Aim::new(true, mods.td(), radius);
        let aim_no_sliders = Aim::new(false, mods.td(), radius);
        let speed = Speed::new(hit_window);
        let flashlight = Flashlight::new(mods.hd(), radius);
        let reading = Reading::new(mods.hd());

        Self {
            aim,
            aim_no_sliders,
            speed,
            flashlight,
            reading,
        }
    }

    pub fn process(&mut self, curr: &OsuDifficultyObject<'_>, objects: &[OsuDifficultyObject<'_>]) {
        self.aim.process(curr, objects);
        self.aim_no_sliders.process(curr, objects);
        self.speed.process(curr, objects);
        self.flashlight.process(curr, objects);
        self.reading.process(curr, objects);
    }
}
