use std::panic::{self, UnwindSafe};

use refx_pp::{
    catch::{CatchPerformance, CatchPerformanceAttributes},
    mania::{ManiaPerformance, ManiaPerformanceAttributes},
    osu::{OsuPerformance, OsuPerformanceAttributes},
    taiko::{TaikoPerformance, TaikoPerformanceAttributes},
    Beatmap,
};

use self::common::*;

mod common;

macro_rules! test_cases {
    ( $mode:ident: $path:ident {
        $( $( $mods:ident )+ => {
            $( $key:ident: $value:expr $( , )? )*
        } ;)*
    } ) => {
        let map = Beatmap::from_path(common::$path).unwrap();

        $(
            let mods = 0 $( + $mods )*;
            let (calc, expected) = test_cases!(@$mode { map, $( $key: $value, )* });
            let actual = calc.mods(mods).calculate().unwrap();
            run(&actual, &expected, mods);
        )*
    };
    ( @Osu {
        $map:ident,
        pp: $pp:expr,
        pp_acc: $pp_acc:expr,
        pp_aim: $pp_aim:expr,
        pp_flashlight: $pp_flashlight:expr,
        pp_speed: $pp_speed:expr,
        effective_miss_count: $effective_miss_count:expr,
        speed_deviation: $speed_deviation:expr,
        aim_estimated_slider_breaks: $aim_estimated_slider_breaks:expr,
        speed_estimated_slider_breaks: $speed_estimated_slider_breaks:expr,
    }) => {
        (
            OsuPerformance::from(&$map).lazer(true),
            OsuPerformanceAttributes {
                pp: $pp,
                pp_acc: $pp_acc,
                pp_aim: $pp_aim,
                pp_flashlight: $pp_flashlight,
                pp_speed: $pp_speed,
                effective_miss_count: $effective_miss_count,
                speed_deviation: $speed_deviation,
                aim_estimated_slider_breaks: $aim_estimated_slider_breaks,
                speed_estimated_slider_breaks: $speed_estimated_slider_breaks,
                ..Default::default()
            },
        )
    };
    ( @Taiko {
        $map: ident,
        pp: $pp:expr,
        pp_acc: $pp_acc:expr,
        pp_difficulty: $pp_difficulty:expr,
        effective_miss_count: $effective_miss_count:expr,
        estimated_unstable_rate: $estimated_unstable_rate:expr,
    }) => {
        (
            TaikoPerformance::from(&$map),
            TaikoPerformanceAttributes {
                pp: $pp,
                pp_acc: $pp_acc,
                pp_difficulty: $pp_difficulty,
                effective_miss_count: $effective_miss_count,
                estimated_unstable_rate: $estimated_unstable_rate,
                ..Default::default()
            },
        )
    };
    ( @Catch {
        $map:ident,
        pp: $pp:expr,
    }) => {
        (
            CatchPerformance::from(&$map),
            CatchPerformanceAttributes {
                pp: $pp,
                ..Default::default()
            },
        )
    };
    ( @Mania {
        $map:ident,
        pp: $pp:expr,
        pp_difficulty: $pp_difficulty:expr,
    }) => {
        (
            ManiaPerformance::from(&$map),
            ManiaPerformanceAttributes {
                pp: $pp,
                pp_difficulty: $pp_difficulty,
                ..Default::default()
            },
        )
    };
}

#[test]
fn basic_osu() {
    #[cfg(target_os = "windows")]
    test_cases! {
        Osu: OSU {
            NM => {
                pp: 271.27311209442854,
                pp_acc: 97.62287463107766,
                pp_aim: 98.93969440570889,
                pp_flashlight: 0.0,
                pp_speed: 65.97387258620209,
                effective_miss_count: 0.0,
                speed_deviation: Some(11.855079578025586),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            HD => {
                pp: 364.56497395798357,
                pp_acc: 105.43270460156388,
                pp_aim: 147.80814449752506,
                pp_flashlight: 0.0,
                pp_speed: 99.57012085136238,
                effective_miss_count: 0.0,
                speed_deviation: Some(11.855079578025586),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            EZ HD => {
                pp: 208.53031776113863,
                pp_acc: 16.6270597231239,
                pp_aim: 107.51075437922465,
                pp_flashlight: 0.0,
                pp_speed: 74.29468671347082,
                effective_miss_count: 0.0,
                speed_deviation: Some(23.1539101317497),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            HR => {
                pp: 403.72299164930644,
                pp_acc: 161.55575439788055,
                pp_aim: 147.50036358525037,
                pp_flashlight: 0.0,
                pp_speed: 80.74831318600936,
                effective_miss_count: 0.0,
                speed_deviation: Some(8.823851275303134),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            DT => {
                pp: 735.8143240866732,
                pp_acc: 184.09450675506795,
                pp_aim: 302.2306653116079,
                pp_flashlight: 0.0,
                pp_speed: 225.42353095885375,
                effective_miss_count: 0.0,
                speed_deviation: Some(7.873979522967204),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            DT HR => {
                pp: 1117.5479792048222,
                pp_acc: 257.56571695089025,
                pp_aim: 513.9559236214852,
                pp_flashlight: 0.0,
                pp_speed: 306.79439386984944,
                effective_miss_count: 0.0,
                speed_deviation: Some(5.835415978964492),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            FL => {
                pp: 399.89950661817414,
                pp_acc: 99.57533212369923,
                pp_aim: 98.93969440570889,
                pp_flashlight: 132.28811994208644,
                pp_speed: 65.97387258620209,
                effective_miss_count: 0.0,
                speed_deviation: Some(11.855079578025586),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            HD FL => {
                pp: 530.4913533836134,
                pp_acc: 107.54135869359516,
                pp_aim: 147.80814449752506,
                pp_flashlight: 171.61406165164138,
                pp_speed: 99.57012085136238,
                effective_miss_count: 0.0,
                speed_deviation: Some(11.855079578025586),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            AP FL => {
                pp: 136.95930541920208,
                pp_acc: 99.57533212369923,
                pp_aim: 0.0,
                pp_flashlight: 21.166099190733835,
                pp_speed: 6.2175437944275185,
                effective_miss_count: 0.0,
                speed_deviation: Some(12.13549995250626),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            AP EZ FL => {
                pp: 31.961364603820975,
                pp_acc: 15.703334182950348,
                pp_aim: 0.0,
                pp_flashlight: 9.898414479880506,
                pp_speed: 5.0964686481270975,
                effective_miss_count: 0.0,
                speed_deviation: Some(23.764692778482623),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
        }
    };
}

#[test]
fn basic_osu_rx() {
    #[cfg(target_os = "windows")]
    test_cases! {
        Osu: OSU_RX {
            RX DT => {
                pp: 3785.4961660628933,
                pp_acc: 328.79732860865414,
                pp_aim: 3001.0723727195955,
                pp_flashlight: 0.0,
                pp_speed: 591.469656001033,
                effective_miss_count: 0.0,
                speed_deviation: Some(4.982527280600511),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            RX HD DT => {
                pp: 4905.249787174736,
                pp_acc: 355.1011148973465,
                pp_aim: 3950.8622533392077,
                pp_flashlight: 0.0,
                pp_speed: 779.5929349165175,
                effective_miss_count: 0.0,
                speed_deviation: Some(4.982527280600511),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            RX DT HR => {
                pp: 4553.519920703758,
                pp_acc: 328.79732860865414,
                pp_aim: 3680.426248231621,
                pp_flashlight: 0.0,
                pp_speed: 595.8862283966607,
                effective_miss_count: 0.0,
                speed_deviation: Some(4.983049299279781),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
        }
    }
}

#[test]
fn basic_osu_precision() {
    #[cfg(target_os = "windows")]
    test_cases! {
        Osu: OSU_PRECISION {
            NM => {
                pp: 1728.8400958712048,
                pp_acc: 110.08622031074181,
                pp_aim: 1321.181690151598,
                pp_flashlight: 0.0,
                pp_speed: 161.1095465579516,
                effective_miss_count: 0.0,
                speed_deviation: Some(11.064670108647732),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            DT => {
                pp: 5865.418886046156,
                pp_acc: 180.48973670161635,
                pp_aim: 4594.93464322394,
                pp_flashlight: 0.0,
                pp_speed: 587.3103401933898,
                effective_miss_count: 0.0,
                speed_deviation: Some(7.373090709552876),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            RX HR => {
                pp: 2677.390104638371,
                pp_acc: 119.72772727278569,
                pp_aim: 2237.9905160901353,
                pp_flashlight: 0.0,
                pp_speed: 168.66386777265313,
                effective_miss_count: 0.0,
                speed_deviation: Some(10.433212319308803),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
            HR => {
                pp: 2777.887942712284,
                pp_acc: 119.72772727278569,
                pp_aim: 2237.9905160901353,
                pp_flashlight: 0.0,
                pp_speed: 168.66386777265313,
                effective_miss_count: 0.0,
                speed_deviation: Some(10.433212319308803),
                aim_estimated_slider_breaks: 0.0,
                speed_estimated_slider_breaks: 0.0,
            };
        }
    }
}

#[test]
fn basic_taiko() {
    test_cases! {
        Taiko: TAIKO {
            NM => {
                pp: 104.65974235594882,
                pp_acc: 67.01508452097738,
                pp_difficulty: 30.951117266143964,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(148.44150180469418),
            };
            HD => {
                pp: 113.35231886537841,
                pp_acc: 67.01508452097738,
                pp_difficulty: 31.72489519779756,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(148.44150180469418),
            };
            HR => {
                pp: 125.39316057548226,
                pp_acc: 83.3355298805701,
                pp_difficulty: 33.77220597125385,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(122.99438720960376),
            };
            DT => {
                pp: 217.2255599983772,
                pp_acc: 119.35453575917016,
                pp_difficulty: 85.09547264616562,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(98.96100120312946),
            };
        }
    };
}

#[test]
fn convert_taiko() {
    #[cfg(target_os = "windows")]
    test_cases! {
        Taiko: OSU {
            NM => {
                pp: 321.96508788209525,
                pp_acc: 150.50068595207387,
                pp_difficulty: 152.95500113793892,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(85.75868894575865),
            };
            HD => {
                pp: 326.0279405978374,
                pp_acc: 150.50068595207387,
                pp_difficulty: 156.7788761663874,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(85.75868894575865),
            };
            HR => {
                pp: 400.1259115798042,
                pp_acc: 187.46770845243455,
                pp_difficulty: 189.65602547641478,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(72.67685680089848),
            };
            DT => {
                pp: 688.6809319343615,
                pp_acc: 274.8702821415836,
                pp_difficulty: 373.46911205993484,
                effective_miss_count: 0.0,
                estimated_unstable_rate: Some(57.17245929717244),
            };
        }
    }
}

#[test]
fn basic_catch() {
    test_cases! {
        Catch: CATCH {
            NM => { pp: 113.85903714373046 };
            HD => { pp: 136.63084457247655 };
            HD HR => { pp: 231.7403429678108 };
            DT => { pp: 247.18402249125842 };
        }
    };
}

#[test]
fn convert_catch() {
    test_cases! {
        Catch: OSU {
            NM => { pp: 232.52175944328079 };
            HD => { pp: 256.35523645996665 };
            HD HR => { pp: 327.71861407740374 };
            DT => { pp: 503.47065792054815 };
        }
    };
}

#[test]
fn basic_mania() {
    test_cases! {
        Mania: MANIA {
            NM => { pp: 108.92297471705167, pp_difficulty: 108.92297471705167 };
            EZ => { pp: 54.46148735852584, pp_difficulty: 108.92297471705167 };
            DT => { pp: 224.52717042937203, pp_difficulty: 224.52717042937203 };
        }
    };
}

#[test]
fn convert_mania() {
    test_cases! {
        Mania: OSU {
            NM => { pp: 101.39189449271568, pp_difficulty: 101.39189449271568 };
            EZ => { pp: 50.69594724635784, pp_difficulty: 101.39189449271568 };
            DT => { pp: 198.46891237015896, pp_difficulty: 198.46891237015896 };
        }
    };
}

fn run<A>(actual: &A, expected: &A, mods: u32)
where
    A: AssertEq,
    for<'a> &'a A: UnwindSafe,
{
    if panic::catch_unwind(|| actual.assert_eq(expected)).is_err() {
        panic!("Mods: {mods}");
    }
}

impl AssertEq for OsuPerformanceAttributes {
    fn assert_eq(&self, expected: &Self) {
        let Self {
            difficulty: _,
            pp,
            pp_acc,
            pp_aim,
            pp_flashlight,
            pp_speed,
            effective_miss_count,
            speed_deviation,
            aim_estimated_slider_breaks,
            speed_estimated_slider_breaks,
            combo_based_estimated_miss_count: _,
            score_based_estimated_miss_count: _,
        } = self;

        assert_eq_float(*pp, expected.pp);
        assert_eq_float(*pp_acc, expected.pp_acc);
        assert_eq_float(*pp_aim, expected.pp_aim);
        assert_eq_float(*pp_flashlight, expected.pp_flashlight);
        assert_eq_float(*pp_speed, expected.pp_speed);
        assert_eq_float(*effective_miss_count, expected.effective_miss_count);
        assert_eq_option(*speed_deviation, expected.speed_deviation);
        assert_eq_float(*aim_estimated_slider_breaks, expected.aim_estimated_slider_breaks);
        assert_eq_float(*speed_estimated_slider_breaks, expected.speed_estimated_slider_breaks);
    }
}

impl AssertEq for TaikoPerformanceAttributes {
    fn assert_eq(&self, expected: &Self) {
        let Self {
            difficulty: _,
            pp,
            pp_acc,
            pp_difficulty,
            effective_miss_count,
            estimated_unstable_rate,
        } = self;

        assert_eq_float(*pp, expected.pp);
        assert_eq_float(*pp_acc, expected.pp_acc);
        assert_eq_float(*pp_difficulty, expected.pp_difficulty);
        assert_eq_float(*effective_miss_count, expected.effective_miss_count);
        assert_eq_option(*estimated_unstable_rate, expected.estimated_unstable_rate);
    }
}

impl AssertEq for CatchPerformanceAttributes {
    fn assert_eq(&self, expected: &Self) {
        let Self { difficulty: _, pp } = self;

        assert_eq_float(*pp, expected.pp);
    }
}

impl AssertEq for ManiaPerformanceAttributes {
    fn assert_eq(&self, expected: &Self) {
        let Self {
            difficulty: _,
            pp,
            pp_difficulty,
        } = self;

        assert_eq_float(*pp_difficulty, expected.pp_difficulty);
        assert_eq_float(*pp, expected.pp);
    }
}
