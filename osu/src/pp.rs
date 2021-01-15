use super::DifficultyAttributes as Attributes;

use parse::{Beatmap, Mods};

pub struct PpResult {
    pub pp: f32,
    pub attributes: Attributes,
}

pub trait PpProvider {
    fn pp(&self) -> PpCalculator;
}

impl PpProvider for Beatmap {
    #[inline]
    fn pp(&self) -> PpCalculator {
        PpCalculator::new(self)
    }
}

// TODO: Allow partial plays
pub struct PpCalculator<'m> {
    map: &'m Beatmap,
    attributes: Option<Attributes>,
    mods: u32,
    combo: Option<usize>,
    acc: Option<f32>,

    n300: Option<usize>,
    n100: Option<usize>,
    n50: Option<usize>,
    n_misses: usize,

    stars_func: Option<Box<dyn Fn(&Beatmap, u32) -> Attributes>>,
}

impl<'m> PpCalculator<'m> {
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

            stars_func: None,
        }
    }

    #[inline]
    pub fn attributes(mut self, attributes: Attributes) -> Self {
        self.attributes.replace(attributes);

        self
    }

    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }

    #[inline]
    pub fn combo(mut self, combo: usize) -> Self {
        self.combo.replace(combo);

        self
    }

    #[inline]
    pub fn n300(mut self, n300: usize) -> Self {
        self.n300.replace(n300);

        self
    }

    #[inline]
    pub fn n100(mut self, n100: usize) -> Self {
        self.n100.replace(n100);

        self
    }

    #[inline]
    pub fn n50(mut self, n50: usize) -> Self {
        self.n50.replace(n50);

        self
    }

    #[inline]
    pub fn misses(mut self, n_misses: usize) -> Self {
        self.n_misses = n_misses;

        self
    }

    #[inline]
    pub fn stars_function(mut self, func: impl Fn(&Beatmap, u32) -> Attributes + 'static) -> Self {
        self.stars_func.replace(Box::new(func));

        self
    }

    /// Generate the hit results with respect to the given accuracy between `0` and `100`.
    ///
    /// Be sure to set `misses` beforehand!
    pub fn accuracy(mut self, acc: f32) -> Self {
        let n_objects = self.map.hit_objects.len();
        let acc = acc / 100.0;

        if self.n100.or(self.n50).is_some() {
            self.n300.replace(
                n_objects - self.n100.unwrap_or(0) - self.n50.unwrap_or(0) - self.n_misses,
            );
            self.n100.get_or_insert(0);
            self.n50.get_or_insert(0);
        } else {
            let target_total = (acc * n_objects as f32 * 6.0).round() as usize;
            let delta = target_total - (n_objects - self.n_misses);

            self.n300.replace(delta / 5);
            self.n100.replace(delta % 5);

            // println!(
            //     "{} - {} - {} - {}",
            //     n_objects,
            //     self.n300.unwrap(),
            //     self.n100.unwrap(),
            //     self.n_misses
            // );

            self.n50
                .replace(n_objects - self.n300.unwrap() - self.n100.unwrap() - self.n_misses);
        }

        let acc = (6 * self.n300.unwrap() + 2 * self.n100.unwrap() + self.n50.unwrap()) as f32
            / (6 * n_objects) as f32;

        self.acc.replace(acc);

        // println!(
        //     "n300: {:?} | n100: {:?} | n50: {:?} | nMiss: {:?} => {}",
        //     self.n300, self.n100, self.n50, self.n_misses, acc
        // );

        self
    }

    pub fn calculate(mut self) -> PpResult {
        if self.attributes.is_none() {
            let stars_func = self
                .stars_func
                .take()
                .unwrap_or_else(|| Box::new(super::no_sliders_no_leniency::stars));

            let attribtes = stars_func(self.map, self.mods);

            // println!("> stars={}", attribtes.stars);

            self.attributes.replace(attribtes);
        }

        if self.acc.is_none() {
            let n_objects = self.map.hit_objects.len();

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
            }

            // println!(
            //     "n300: {:?} | n100: {:?} | n50: {:?} | nMiss: {:?}",
            //     self.n300, self.n100, self.n50, self.n_misses
            // );

            let numerator = self.n50.unwrap() + self.n100.unwrap() * 2 + self.n300.unwrap() * 6;
            self.acc.replace(numerator as f32 / n_objects as f32 / 6.0);
        }

        let total_hits = self.total_hits();
        let mut multiplier = 1.12;

        if self.mods.nf() {
            multiplier *= (1.0 - 0.02 * self.n_misses as f32).max(0.9);
        }

        if self.mods.so() {
            let n_spinners = self.attributes.as_ref().unwrap().n_spinners;
            multiplier *= 1.0 - (n_spinners as f32 / total_hits as f32).powf(0.85);
        }

        let aim_value = self.compute_aim_value(total_hits as f32);
        let speed_value = self.compute_speed_value(total_hits as f32);
        let acc_value = self.compute_accuracy_value(total_hits);

        // println!(
        //     "aim={} | speed={} | acc={}",
        //     aim_value, speed_value, acc_value
        // );

        let pp = (aim_value.powf(1.1) + speed_value.powf(1.1) + acc_value.powf(1.1))
            .powf(1.0 / 1.1)
            * multiplier;

        PpResult {
            pp,
            attributes: self.attributes.unwrap(),
        }
    }

    fn compute_aim_value(&self, total_hits: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();

        // println!("aim_strain={}", attributes.aim_strain);

        // TD penalty
        let raw_aim = if self.mods.td() {
            attributes.aim_strain.powf(0.8)
        } else {
            attributes.aim_strain
        };

        // println!("raw={}", raw_aim);

        let mut aim_value = (5.0 * (raw_aim / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0;

        // println!("init: {}", aim_value);

        // Longer maps are worth more
        let len_bonus = 0.95
            + 0.4 * (total_hits / 2000.0).min(1.0)
            + (total_hits > 2000.0) as u8 as f32 * 0.5 * (total_hits / 2000.0).log10();
        aim_value *= len_bonus;

        // println!("len bonus: {} => {}", len_bonus, aim_value);

        // Penalize misses
        if self.n_misses > 0 {
            aim_value *= 0.97
                * (1.0 - (self.n_misses as f32 / total_hits).powf(0.775))
                    .powi(self.n_misses as i32);
        }

        // println!("miss penalty: {}", aim_value);

        // println!(
        //     "combo={:?} | max_combo={}",
        //     self.combo, attributes.max_combo
        // );

        // Combo scaling
        if let Some(combo) = self.combo.filter(|_| attributes.max_combo > 0) {
            aim_value *= ((combo as f32 / attributes.max_combo as f32).powf(0.8)).min(1.0);
        }

        // println!("combo scaling: {}", aim_value);

        // AR bonus
        let mut ar_factor = 0.0;
        if attributes.ar > 10.33 {
            ar_factor += 0.4 * (attributes.ar - 10.33);
        } else if attributes.ar < 8.0 {
            ar_factor += 0.1 * (8.0 - attributes.ar);
        }
        aim_value *= 1.0 + ar_factor.min(ar_factor * total_hits / 1000.0);

        // println!("ar bonus: {} => {}", ar_factor, aim_value);

        // HD bonus
        if self.mods.hd() {
            aim_value *= 1.0 + 0.04 * (12.0 - attributes.ar);
        }

        // println!("hd bonus: {}", aim_value);

        // FL bonus
        if self.mods.fl() {
            aim_value *= 1.0
                + 0.35 * (total_hits / 200.0).min(1.0)
                + (total_hits > 200.0) as u8 as f32 * 0.3 * ((total_hits - 200.0) / 300.0).min(1.0)
                + (total_hits > 500.0) as u8 as f32 * (total_hits - 500.0) / 1200.0;
        }

        // println!("fl bonus: {}", aim_value);

        // Scale with accuracy
        aim_value *= 0.5 + self.acc.unwrap() / 2.0;
        aim_value *= 0.98 + attributes.od * attributes.od / 2500.0;

        // println!("> acc: {:?}", self.acc);

        // println!("final: {}", aim_value);

        aim_value
    }

    fn compute_speed_value(&self, total_hits: f32) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();

        // println!("speed_strain={}", attributes.speed_strain);

        let mut speed_value =
            (5.0 * (attributes.speed_strain / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0;

        // println!(
        //     "curr={} | modified={}",
        //     speed_value,
        //     (5.0 * (2.0994549379474163_f32 / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0
        // );

        // println!("init: {}", speed_value);

        // Longer maps are worth more
        let len_bonus = 0.95
            + 0.4 * (total_hits / 2000.0).min(1.0)
            + (total_hits > 2000.0) as u8 as f32 * 0.5 * (total_hits / 2000.0).log10();
        speed_value *= len_bonus;

        // println!("len bonus: {} => {}", len_bonus, speed_value);

        // Penalize misses
        if self.n_misses > 0 {
            speed_value *= 0.97
                * (1.0 - (self.n_misses as f32 / total_hits).powf(0.775))
                    .powf((self.n_misses as f32).powf(0.875));
        }

        // println!("miss penalty: {}", speed_value);

        // Combo scaling
        if let Some(combo) = self.combo.filter(|_| attributes.max_combo > 0) {
            speed_value *= ((combo as f32 / attributes.max_combo as f32).powf(0.8)).min(1.0);
        }

        // println!("combo scaling: {}", speed_value);

        // AR bonus
        if attributes.ar > 10.33 {
            let ar_factor = 0.4 * (attributes.ar - 10.33);
            speed_value *= 1.0 + ar_factor.min(ar_factor * total_hits / 1000.0);
        }

        // println!("ar bonus: {}", speed_value);

        // HD bonus
        if self.mods.hd() {
            speed_value *= 1.0 + 0.04 * (12.0 - attributes.ar);
        }

        // println!("hidden bonus: {}", speed_value);

        // Scaling the speed value with accuracy and OD
        let od_factor = 0.95 + attributes.od * attributes.od / 750.0;
        let acc_factor = self
            .acc
            .unwrap()
            .powf((14.5 - attributes.od.max(8.0)) / 2.0);
        speed_value *= od_factor * acc_factor;

        // println!("acc & od scaling: {}", speed_value);

        // Penalize n50s
        speed_value *= 0.98_f32.powf(
            (self.n50.unwrap_or(0) as f32 >= total_hits / 500.0) as u8 as f32
                * (self.n50.unwrap_or(0) as f32 - total_hits / 500.0),
        );

        // println!("final: {}", speed_value);

        speed_value
    }

    fn compute_accuracy_value(&self, total_hits: usize) -> f32 {
        let attributes = self.attributes.as_ref().unwrap();
        let n_circles = attributes.n_circles;

        // println!("n_circles={}", n_circles);

        let better_acc_percentage = (n_circles > 0) as u8 as f32
            * (((self.n300.unwrap() - (total_hits - n_circles)) * 6
                + self.n100.unwrap_or(0) * 2
                + self.n50.unwrap_or(0)) as f32
                / (n_circles * 6) as f32)
                .max(0.0);

        // println!("better_acc_percentage={}", better_acc_percentage);

        let attributes = self.attributes.as_ref().unwrap();

        let mut acc_value = 1.52163_f32.powf(attributes.od) * better_acc_percentage.powi(24) * 2.83;

        // println!(
        //     "1.52163^{} * {}^24 * 2.83 = {}",
        //     attributes.od, better_acc_percentage, acc_value
        // );

        // println!("init: {}", acc_value);

        // Bonus for many hitcircles
        acc_value *= ((n_circles as f32 / 1000.0).powf(0.3)).min(1.15);

        // HD bonus
        if self.mods.hd() {
            acc_value *= 1.08;
        }

        // FL bonus
        if self.mods.fl() {
            acc_value *= 1.02;
        }

        acc_value
    }

    #[inline]
    fn total_hits(&self) -> usize {
        self.n300.unwrap_or(0) + self.n100.unwrap_or(0) + self.n50.unwrap_or(0) + self.n_misses
    }
}
