use super::stars::{stars, OsuDifficultyAttributes, OsuPerformanceAttributes};
use crate::{Beatmap, Mods};

/// Calculator for pp on osu!standard maps.
///
/// # Example
///
/// ```
/// # use rosu_pp::{OsuPP, Beatmap};
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
/// let attrs = OsuPP::new(&map)
///     .mods(8 + 64) // HDDT
///     .combo(1234)
///     .misses(1)
///     .accuracy(98.5) // should be set last
///     .calculate();
///
/// println!("PP: {} | Stars: {}", attrs.pp(), attrs.stars());
///
/// let next_result = OsuPP::new(&map)
///     .attributes(attrs) // reusing previous results for performance
///     .mods(8 + 64)      // has to be the same to reuse attributes
///     .accuracy(99.5)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", next_result.pp(), next_result.stars());
/// ```
#[derive(Clone, Debug)]
pub struct OsuPP<'m> {
    map: &'m Beatmap,
    attributes: Option<OsuDifficultyAttributes>,
    mods: u32,
    combo: Option<usize>,
    acc: Option<f32>,

    n300: Option<usize>,
    n100: Option<usize>,
    n50: Option<usize>,
    n_misses: usize,
    passed_objects: Option<usize>,
}

impl<'m> OsuPP<'m> {
    /// Creates a new calculator for the given map.
    #[inline]
    pub fn new(map: &'m Beatmap) -> Self {
        Self {
            map,
            attributes: None,
            mods: 0,
            combo: None,
            acc: None,

            n300: None,
            n100: None,
            n50: None,
            n_misses: 0,
            passed_objects: None,
        }
    }

    /// [`OsuAttributeProvider`] is implemented by [`DifficultyAttributes`](crate::osu::DifficultyAttributes)
    /// and by [`PpResult`](crate::PpResult) meaning you can give the
    /// result of a star calculation or a pp calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: impl OsuAttributeProvider) -> Self {
        if let Some(attributes) = attributes.attributes() {
            self.attributes.replace(attributes);
        }

        self
    }

    /// Specify mods through their bit values.
    ///
    /// See [https://github.com/ppy/osu-api/wiki#mods](https://github.com/ppy/osu-api/wiki#mods)
    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }

    /// Specify the max combo of the play.
    #[inline]
    pub fn combo(mut self, combo: usize) -> Self {
        self.combo.replace(combo);

        self
    }

    /// Specify the amount of 300s of a play.
    #[inline]
    pub fn n300(mut self, n300: usize) -> Self {
        self.n300.replace(n300);

        self
    }

    /// Specify the amount of 100s of a play.
    #[inline]
    pub fn n100(mut self, n100: usize) -> Self {
        self.n100.replace(n100);

        self
    }

    /// Specify the amount of 50s of a play.
    #[inline]
    pub fn n50(mut self, n50: usize) -> Self {
        self.n50.replace(n50);

        self
    }

    /// Specify the amount of misses of a play.
    #[inline]
    pub fn misses(mut self, n_misses: usize) -> Self {
        self.n_misses = n_misses;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects.replace(passed_objects);

        self
    }

    /// Generate the hit results with respect to the given accuracy between `0` and `100`.
    ///
    /// Be sure to set `misses` beforehand!
    /// In case of a partial play, be also sure to set `passed_objects` beforehand!
    pub fn accuracy(mut self, acc: f32) -> Self {
        let n_objects = self.passed_objects.unwrap_or(self.map.hit_objects.len());

        let acc = acc / 100.0;

        if self.n100.or(self.n50).is_some() {
            let mut n100 = self.n100.unwrap_or(0);
            let mut n50 = self.n50.unwrap_or(0);

            let placed_points = 2 * n100 + n50 + self.n_misses;
            let missing_objects = n_objects - n100 - n50 - self.n_misses;
            let missing_points =
                ((6.0 * acc * n_objects as f32).round() as usize).saturating_sub(placed_points);

            let mut n300 = missing_objects.min(missing_points / 6);
            n50 += missing_objects - n300;

            if let Some(orig_n50) = self.n50.filter(|_| self.n100.is_none()) {
                // Only n50s were changed, try to load some off again onto n100s
                let difference = n50 - orig_n50;
                let n = n300.min(difference / 4);

                n300 -= n;
                n100 += 5 * n;
                n50 -= 4 * n;
            }

            self.n300.replace(n300);
            self.n100.replace(n100);
            self.n50.replace(n50);
        } else {
            let misses = self.n_misses.min(n_objects);
            let target_total = (acc * n_objects as f32 * 6.0).round() as usize;
            let delta = target_total - (n_objects - misses);

            let mut n300 = delta / 5;
            let mut n100 = delta % 5;
            let mut n50 = n_objects - n300 - n100 - misses;

            // Sacrifice n300s to transform n50s into n100s
            let n = n300.min(n50 / 4);
            n300 -= n;
            n100 += 5 * n;
            n50 -= 4 * n;

            self.n300.replace(n300);
            self.n100.replace(n100);
            self.n50.replace(n50);
        }

        let acc = (6 * self.n300.unwrap() + 2 * self.n100.unwrap() + self.n50.unwrap()) as f32
            / (6 * n_objects) as f32;

        self.acc.replace(acc);

        self
    }

    fn assert_hitresults(&mut self) {
        if self.acc.is_none() {
            let n_objects = self.passed_objects.unwrap_or(self.map.hit_objects.len());

            let remaining = n_objects
                .saturating_sub(self.n300.unwrap_or(0))
                .saturating_sub(self.n100.unwrap_or(0))
                .saturating_sub(self.n50.unwrap_or(0))
                .saturating_sub(self.n_misses);

            if remaining > 0 {
                if self.n300.is_none() {
                    self.n300.replace(remaining);
                    self.n100.get_or_insert(0);
                    self.n50.get_or_insert(0);
                } else if self.n100.is_none() {
                    self.n100.replace(remaining);
                    self.n50.get_or_insert(0);
                } else if self.n50.is_none() {
                    self.n50.replace(remaining);
                } else {
                    *self.n300.as_mut().unwrap() += remaining;
                }
            } else {
                self.n300.get_or_insert(0);
                self.n100.get_or_insert(0);
                self.n50.get_or_insert(0);
            }

            let numerator = self.n50.unwrap() + self.n100.unwrap() * 2 + self.n300.unwrap() * 6;
            self.acc.replace(numerator as f32 / n_objects as f32 / 6.0);
        }
    }

    /// Returns an object which contains the pp and [`DifficultyAttributes`](crate::osu::DifficultyAttributes)
    /// containing stars and other attributes.
    pub fn calculate(mut self) -> OsuPerformanceAttributes {
        if self.attributes.is_none() {
            let attributes = stars(self.map, self.mods, self.passed_objects);
            self.attributes.replace(attributes);
        }

        // Make sure the hitresults and accuracy are set
        self.assert_hitresults();

        let total_hits = self.total_hits() as f32;
        let mut multiplier = 1.09;

        let effective_miss_count = self.calculate_effective_miss_count();

        // SO penalty
        if self.mods.so() {
            multiplier *=
                1.0 - (self.attributes.as_ref().unwrap().n_spinners as f32 / total_hits).powf(0.85);
        }

        let aim_value = self.compute_aim_value(total_hits, effective_miss_count);
        let speed_value = self.compute_speed_value(total_hits, effective_miss_count);
        let acc_value = self.compute_accuracy_value(total_hits);
        let flashlight_value = self.compute_flashlight_value(total_hits, effective_miss_count);

        let nodt_bonus = match !self.mods.change_speed() {
            true => 1.03,
            false => 1.0,
        };

        let pp = (aim_value.powf(1.1 * nodt_bonus)
            + speed_value.powf(0.83)
            + acc_value.powf(1.14 * nodt_bonus))
            + flashlight_value.powf(1.1)
        .powf(1.0 / 1.1)
            * multiplier;

        OsuPerformanceAttributes {
            difficulty: self.attributes.unwrap(),
            pp_acc: acc_value as f64,
            pp_aim: aim_value as f64,
            pp_flashlight: flashlight_value as f64,
            pp_speed: speed_value as f64,
            pp: pp as f64,
            effective_miss_count: effective_miss_count as f64,
        }
    }

    fn compute_aim_value(&self, total_hits: f32, effective_miss_count: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();
    
        let raw_aim = if self.mods.td() {
            attributes.aim_strain.powf(0.85) as f32 
        } else {
            attributes.aim_strain as f32
        };
    
        let mut aim_value = (5.0 * (raw_aim / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0;
    
        let len_bonus = 0.9  
            + 0.45 * (total_hits / 2000.0).min(1.0) 
            + (total_hits > 2000.0) as u8 as f32 * 0.55 * (total_hits / 2000.0).log10(); 
        aim_value *= len_bonus;
    
        if effective_miss_count > 0.0 {
            let miss_penalty = self.calculate_miss_penalty(effective_miss_count);
            aim_value *= miss_penalty * 0.95; 
        }
    
        let mut ar_factor = if attributes.ar > 10.33 {
            0.32 * (attributes.ar - 10.33) 
        } else {
            0.0
        };
    
        if attributes.ar < 8.0 {
            ar_factor = 0.03 * (8.0 - attributes.ar); 
        }
    
        aim_value *= 1.0 + ar_factor as f32 * len_bonus;
    
        if self.mods.hd() {
            aim_value *= 1.0 + 0.06 * (11.0 - attributes.ar) as f32;
        }
    
        if self.mods.fl() {
            aim_value *= 1.0
                + 0.35 * (total_hits / 200.0).min(1.0) 
                + (total_hits > 200.0) as u8 as f32
                    * 0.3
                    * ((total_hits - 200.0) / 300.0).min(1.0)
                + (total_hits > 500.0) as u8 as f32 * (total_hits - 500.0) / 1500.0; 
        }
    
        if self.mods.ez() {
            let mut base_buff = 1.1_f32;
    
            if attributes.ar <= 8.0 {
                base_buff += (7.0 - attributes.ar as f32) / 90.0;
            }
    
            aim_value *= base_buff;
        }
    
        aim_value *= 0.35 + self.acc.unwrap() / 1.9;
        aim_value *= 0.99 + attributes.od as f32 * attributes.od as f32 / 2400.0;
    
        aim_value
    }

    fn compute_speed_value(&self, total_hits: f32, effective_miss_count: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();
    
        let mut speed_value =
            (6.0 * (attributes.speed_strain as f32 / 0.0675).max(1.0) - 4.0).powi(3) / 55_000.0;
    
        // Longer maps are worth more
        let len_bonus = 1.1
            + 0.7 * (total_hits / 1500.0).min(1.0)
            + (total_hits > 1500.0) as u8 as f32 * 0.7 * (total_hits / 1500.0).log10();
        speed_value *= len_bonus;
    
        // Penalize misses
        if effective_miss_count > 0.0 {
            let miss_penalty = self.calculate_miss_penalty(effective_miss_count).powf(0.863);
            speed_value *= miss_penalty;
        }
    
        // AR bonus
        if attributes.ar > 10.33 {
            let mut ar_factor = if attributes.ar > 10.33 {
                0.5 * (attributes.ar - 10.33)
            } else {
                0.0
            };
    
            if attributes.ar < 8.0 {
                ar_factor = 0.04 * (8.0 - attributes.ar);
            }
    
            speed_value *= 1.0 + ar_factor as f32 * len_bonus;
        }
    
        // HD bonus
        if self.mods.hd() {
            speed_value *= 1.0 + 0.1 * (11.0 - attributes.ar) as f32;
        }
    
        // Scaling the speed value with accuracy and OD
        speed_value *= (1.1 + attributes.od as f32 * attributes.od as f32 / 600.0)
            * self
                .acc
                .unwrap()
                .powf((14.0 - attributes.od.max(8.0) as f32) / 2.0);
    
        speed_value *= 0.95_f32.powf((self.n50.unwrap() as f32 - total_hits / 500.0).max(0.0));
    
        speed_value
    }    

    fn compute_accuracy_value(&self, total_hits: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();
        let n_circles = attributes.n_circles as f32;
        let n300 = self.n300.unwrap_or(0) as f32;
        let n100 = self.n100.unwrap_or(0) as f32;
        let n50 = self.n50.unwrap_or(0) as f32;

        let better_acc_percentage = (n_circles > 0.0) as u8 as f32
            * (((n300 - (total_hits - n_circles)) * 6.0 + n100 * 2.0 + n50) / (n_circles * 6.0))
                .max(0.0);

        let mut acc_value =
            1.52163_f32.powf(attributes.od as f32) * better_acc_percentage.powi(24) * 2.83;

        // Bonus for many hitcircles
        acc_value *= ((n_circles as f32 / 1000.0).powf(0.3)).min(1.15);

        let acc_scaling_factor = if better_acc_percentage < 0.96 {
            let min_acc = 0.70;
            let max_acc = 0.96;
            let min_scale = 0.60;
            let max_scale = 0.90;
    
            if better_acc_percentage < min_acc {
                min_scale
            } else if better_acc_percentage < max_acc {
                let progress = (better_acc_percentage - min_acc) / (max_acc - min_acc);
                let smooth_progress = progress * progress;
                min_scale + (max_scale - min_scale) * smooth_progress
            } else {
                max_scale
            }
        } else {
            1.0
        };

        // HD bonus
        if self.mods.hd() {
            acc_value *= 1.08;
        }

        // FL bonus
        if self.mods.fl() {
            acc_value *= 1.02;
        }

        acc_value *= acc_scaling_factor;

        acc_value
    }

    fn compute_flashlight_value(&self, total_hits: f32, effective_miss_count: f32) -> f32 {
        if !self.mods.fl() {
            return 0.0;
        }
    
        let attributes = self.attributes.as_ref().unwrap();
        let mut flashlight_value = (attributes.aim_strain as f32 / 2.0).powi(2) * 30.0;
    
        // * Penalize misses by assessing # of misses relative to the total # of objects. 
        // * Default a 3% reduction for any # of misses.
        if effective_miss_count > 0.0 {
            flashlight_value *= 0.98
                * (1.0 - (effective_miss_count / total_hits).powf(0.725))
                    .powf(effective_miss_count.powf(0.825));
        }
        
        // * Scale the flashlight value with combo.
        flashlight_value *= self.combo.filter(|_| attributes.max_combo > 0)
            .map(|combo| 
                ((combo as f32 / attributes.max_combo as f32)
                .powf(0.8))
                .min(1.0))
                .unwrap_or(1.0);
        
        // * Account for shorter maps having a higher ratio of 0 combo/100 combo flashlight radius.
        flashlight_value *= 0.8
            + 0.15 * (total_hits / 200.0).min(1.0)
            + (total_hits > 200.0) as u8 as f32 * 0.25 * ((total_hits - 200.0) / 200.0).min(1.0);
    
        // * Scale the flashlight value with accuracy _slightly_.
        flashlight_value *= 0.6 + self.acc.unwrap() / 1.8;
    
        // * It is important to also consider accuracy difficulty when doing that.
        flashlight_value *= 1.0 + attributes.od.powi(2) as f32 / 2400.0;
    
        flashlight_value
    }

    #[inline]
    fn total_hits(&self) -> usize {
        let n_objects = self.passed_objects.unwrap_or(self.map.hit_objects.len());

        (self.n300.unwrap_or(0) + self.n100.unwrap_or(0) + self.n50.unwrap_or(0) + self.n_misses)
            .min(n_objects)
    }

    #[inline]
    fn calculate_miss_penalty(&self, effective_miss_count: f32) -> f32 {
        let total_hits = self.total_hits() as f32;

        0.97 * (1.0 - (effective_miss_count / total_hits).powf(0.5))
            .powf(1.0 + (effective_miss_count / 1.5))
    }

    #[inline]
    fn calculate_effective_miss_count(&self) -> f32 {
        self.n_misses as f32
    }
}

/// Provides attributes for an osu! beatmap.
pub trait OsuAttributeProvider {
    /// Returns the attributes of the map.
    fn attributes(self) -> Option<OsuDifficultyAttributes>;
}

impl OsuAttributeProvider for OsuDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self)
    }
}

impl OsuAttributeProvider for OsuPerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self.difficulty)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Beatmap;

    #[test]
    fn osu_only_accuracy() {
        let map = Beatmap::default();

        let total_objects = 1234;
        let target_acc = 97.5;

        let calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .accuracy(target_acc);

        let numerator = 6 * calculator.n300.unwrap_or(0)
            + 2 * calculator.n100.unwrap_or(0)
            + calculator.n50.unwrap_or(0);
        let denominator = 6 * total_objects;
        let acc = 100.0 * numerator as f32 / denominator as f32;

        assert!(
            (target_acc - acc).abs() < 1.0,
            "Expected: {} | Actual: {}",
            target_acc,
            acc
        );
    }

    #[test]
    fn osu_accuracy_and_n50() {
        let map = Beatmap::default();

        let total_objects = 1234;
        let target_acc = 97.5;
        let n50 = 30;

        let calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .n50(n50)
            .accuracy(target_acc);

        assert!(
            (calculator.n50.unwrap() as i32 - n50 as i32).abs() <= 4,
            "Expected: {} | Actual: {}",
            n50,
            calculator.n50.unwrap()
        );

        let numerator = 6 * calculator.n300.unwrap_or(0)
            + 2 * calculator.n100.unwrap_or(0)
            + calculator.n50.unwrap_or(0);
        let denominator = 6 * total_objects;
        let acc = 100.0 * numerator as f32 / denominator as f32;

        assert!(
            (target_acc - acc).abs() < 1.0,
            "Expected: {} | Actual: {}",
            target_acc,
            acc
        );
    }

    #[test]
    fn osu_missing_objects() {
        let map = Beatmap::default();

        let total_objects = 1234;
        let n300 = 1000;
        let n100 = 200;
        let n50 = 30;

        let mut calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .n300(n300)
            .n100(n100)
            .n50(n50);

        calculator.assert_hitresults();

        let n_objects = calculator.n300.unwrap()
            + calculator.n100.unwrap()
            + calculator.n50.unwrap()
            + calculator.n_misses;

        assert_eq!(
            total_objects, n_objects,
            "Expected: {} | Actual: {}",
            total_objects, n_objects
        );
    }
}