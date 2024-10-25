macro_rules! impl_mods {
    ($func_name:ident, $const_name:ident) => {
        #[inline]
        fn $func_name(self) -> bool {
            self & Self::$const_name > 0
        }
    };
}

/// Abstract type to define mods.
#[allow(missing_docs)]
pub trait Mods: Copy {
    const NF: u32 = 1 << 0;
    const EZ: u32 = 1 << 1;
    const TD: u32 = 1 << 2;
    const HD: u32 = 1 << 3;
    const HR: u32 = 1 << 4;
    const DT: u32 = 1 << 6;
    const RX: u32 = 1 << 7;
    const HT: u32 = 1 << 8;
    const FL: u32 = 1 << 10;
    const SO: u32 = 1 << 12;
    const AP: u32 = 1 << 13;
    const SV2: u32 = 1 << 29;

    /// If the clock rate is affected by the mods.
    fn change_speed(self) -> bool;
    /// If object time's or positions are affected by the mods.
    fn change_map(self) -> bool;
    /// The clock rate with the mods.
    fn clock_rate(self) -> f64;
    /// Multiplier for beatmap attributes with respect to the mods.
    fn od_ar_hp_multiplier(self) -> f64;
    fn nf(self) -> bool;
    fn ez(self) -> bool;
    fn td(self) -> bool;
    fn hd(self) -> bool;
    fn hr(self) -> bool;
    fn dt(self) -> bool;
    fn rx(self) -> bool;
    fn ht(self) -> bool;
    fn fl(self) -> bool;
    fn so(self) -> bool;
    fn ap(self) -> bool;
    fn sv2(self) -> bool;
}

impl Mods for u32 {
    #[inline]
    fn change_speed(self) -> bool {
        self & (Self::HT | Self::DT) > 0
    }

    #[inline]
    fn change_map(self) -> bool {
        self & (Self::HT | Self::DT | Self::HR | Self::EZ) > 0
    }

    #[inline]
    fn clock_rate(self) -> f64 {
        if self & Self::DT > 0 {
            1.5
        } else if self & Self::HT > 0 {
            0.75
        } else {
            1.0
        }
    }

    #[inline]
    fn od_ar_hp_multiplier(self) -> f64 {
        if self & Self::HR > 0 {
            1.4
        } else if self & Self::EZ > 0 {
            0.5
        } else {
            1.0
        }
    }

    impl_mods!(nf, NF);
    impl_mods!(ez, EZ);
    impl_mods!(td, TD);
    impl_mods!(hd, HD);
    impl_mods!(hr, HR);
    impl_mods!(dt, DT);
    impl_mods!(rx, RX);
    impl_mods!(ht, HT);
    impl_mods!(fl, FL);
    impl_mods!(so, SO);
    impl_mods!(ap, AP);
    impl_mods!(sv2, SV2);
}
