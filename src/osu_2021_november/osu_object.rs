use std::{cmp::Ordering, convert::identity};

use crate::{
    section::hit_objects::{BorrowedCurve, CurveBuffers},
    util::Pos,
};
use crate::{
    model::{
        control_point::{DifficultyPoint, TimingPoint},
        hit_object::{HitObject, HitObjectKind, HoldNote, Slider, Spinner},
    },
    Beatmap,
};

use crate::util::control_points::{difficulty_point_at, timing_point_at};

use super::OsuDifficultyAttributes;

const LEGACY_LAST_TICK_OFFSET: f64 = 36.0;
const BASE_SCORING_DISTANCE: f64 = 100.0;

#[derive(Clone, Debug)]
pub(crate) struct OsuObject {
    pub(crate) time: f64,
    pub(crate) pos: Pos,
    pub(crate) stack_height: f32,
    pub(crate) kind: OsuObjectKind,
}

#[derive(Clone, Debug)]
pub(crate) enum OsuObjectKind {
    Circle,
    Slider {
        end_time: f64,
        end_pos: Pos,
        lazy_end_pos: Pos,
        nested_objects: Vec<NestedObject>,
    },
    Spinner {
        end_time: f64,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct NestedObject {
    pub(crate) pos: Pos,
    pub(crate) time: f64,
    pub(crate) kind: NestedObjectKind,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum NestedObjectKind {
    Repeat,
    Tail,
    Tick,
}

pub(crate) struct ObjectParameters<'a> {
    pub(crate) map: &'a Beatmap,
    pub(crate) attributes: &'a mut OsuDifficultyAttributes,
    pub(crate) ticks: Vec<(Pos, f64)>,
    pub(crate) curve_bufs: CurveBuffers,
}

impl OsuObject {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(h: &HitObject, hr: bool, params: &mut ObjectParameters<'_>) -> Self {
        let ObjectParameters {
            map,
            attributes: attrs,
            ticks,
            curve_bufs,
        } = params;

        attrs.max_combo += 1; // hitcircle, slider head, or spinner
        let mut pos = h.pos;

        if hr {
            pos.y = 384.0 - pos.y;
        }

        match &h.kind {
            HitObjectKind::Circle => {
                attrs.n_circles += 1;

                Self {
                    time: h.start_time,
                    pos,
                    stack_height: 0.0,
                    kind: OsuObjectKind::Circle,
                }
            }
            HitObjectKind::Slider(Slider {
                expected_dist,
                repeats,
                control_points,
                node_sounds: _,
            }) => {
                attrs.n_sliders += 1;

                let beat_len = timing_point_at(&map.timing_points, h.start_time)
                    .map_or(TimingPoint::DEFAULT_BEAT_LEN, |timing| timing.beat_len);

                let slider_vel = difficulty_point_at(&map.difficulty_points, h.start_time)
                    .map_or(DifficultyPoint::DEFAULT_SLIDER_VELOCITY, |difficulty| {
                        difficulty.slider_velocity
                    });

                let span_count = (*repeats + 1) as f64;

                let mut tick_dist = 100.0 * map.slider_multiplier / map.slider_tick_rate;

                // * prior to v8, speed multipliers don't adjust for how many ticks are generated over the same distance.
                // * this results in more (or less) ticks being generated in <v8 maps for the same time duration.
                if map.version >= 8 {
                    tick_dist /= (100.0 / slider_vel).clamp(10.0, 1000.0) / 100.0;
                }

                // Build the curve w.r.t. the control points
                let curve = BorrowedCurve::new(control_points, *expected_dist, curve_bufs);

                let velocity =
                    (BASE_SCORING_DISTANCE * map.slider_multiplier * slider_vel) / beat_len;

                let end_time = h.start_time + span_count * curve.dist() / velocity;
                let duration = end_time - h.start_time;
                let span_duration = duration / span_count;

                // * A very lenient maximum length of a slider for ticks to be generated.
                // * This exists for edge cases such as /b/1573664 where the beatmap has
                // * been edited by the user, and should never be reached in normal usage.
                let max_len = 100_000.0;

                let len = curve.dist().min(max_len);
                tick_dist = tick_dist.clamp(0.0, len);
                let min_dist_from_end = velocity * 10.0;

                let mut curr_dist = tick_dist;

                ticks.clear();
                ticks.reserve((len / tick_dist) as usize);
                let mut nested_objects =
                    Vec::with_capacity((len * span_count / tick_dist) as usize);

                // Ticks of the first span
                while curr_dist < len - min_dist_from_end {
                    let progress = curr_dist / len;

                    let curr_time = h.start_time + progress * span_duration;
                    let mut curr_pos = h.pos + curve.position_at(progress);

                    if hr {
                        curr_pos.y = 384.0 - curr_pos.y;
                    }

                    let tick = NestedObject {
                        pos: curr_pos,
                        time: curr_time,
                        kind: NestedObjectKind::Tick,
                    };

                    nested_objects.push(tick);
                    ticks.push((curr_pos, curr_time));

                    curr_dist += tick_dist;
                }

                // Other spans
                for span_idx in 1..=*repeats {
                    let progress = (span_idx % 2 == 1) as u8 as f64;
                    let span_idx_f64 = span_idx as f64;

                    // Repeat point
                    let curr_time = h.start_time + span_duration * span_idx_f64;
                    let mut curr_pos = h.pos + curve.position_at(progress);

                    if hr {
                        curr_pos.y = 384.0 - curr_pos.y;
                    }

                    let repeat = NestedObject {
                        pos: curr_pos,
                        time: curr_time,
                        kind: NestedObjectKind::Repeat,
                    };

                    nested_objects.push(repeat);

                    // Ticks
                    if span_idx & 1 == 1 {
                        // S-------->R | Span 0
                        //  2  4  6  8 | => span_duration = 8
                        // R<--------- | Span 1
                        // 16 14 12 10 | => offset = 1 * span_duration
                        // --------->R | Span 2
                        // 18 20 22 24 | => not reverse; simple case
                        // T<--------- | Span 3
                        // 32 30 28 26 | => offset = 3 * span_duration
                        //
                        //  n = offset + tick
                        // 26 =   24   +   2
                        // 28 =   24   +   4
                        // 30 =   24   +   6
                        // 32 =   24   +   8

                        let offset = span_idx_f64 * span_duration;

                        let tick_iter = ticks.iter().rev().zip(ticks.iter()).map(
                            |((rev_pos, _), (_, time))| NestedObject {
                                pos: *rev_pos,
                                time: offset + time,
                                kind: NestedObjectKind::Tick,
                            },
                        );

                        nested_objects.extend(tick_iter);
                    } else {
                        let tick_iter = ticks.iter().map(|(pos, time)| NestedObject {
                            pos: *pos,
                            time: time + span_duration * span_idx_f64,
                            kind: NestedObjectKind::Tick,
                        });

                        nested_objects.extend(tick_iter);
                    }
                }

                // Slider tail
                let final_span_start_time = h.start_time + *repeats as f64 * span_duration;
                let final_span_end_time = (h.start_time + duration / 2.0)
                    .max(final_span_start_time + span_duration - LEGACY_LAST_TICK_OFFSET);

                let progress = (*repeats % 2 == 0) as u8 as f64;
                let mut end_pos = h.pos + curve.position_at(progress);

                if hr {
                    end_pos.y = 384.0 - end_pos.y;
                }

                // * we need to use the LegacyLastTick here for compatibility reasons (difficulty).
                // * it is *okay* to use this because the TailCircle is not used for any meaningful purpose in gameplay.
                // * if this is to change, we should revisit this.
                let legacy_last_tick = NestedObject {
                    pos: end_pos,
                    time: final_span_end_time,
                    kind: NestedObjectKind::Tail,
                };

                // On very short buzz sliders it can happen that the
                // legacy last tick is not the last object time-wise
                match nested_objects.last() {
                    Some(last) if last.time > final_span_end_time => {
                        let idx = nested_objects
                            .binary_search_by(|nested| {
                                nested
                                    .time
                                    .partial_cmp(&final_span_end_time)
                                    .unwrap_or(Ordering::Equal)
                            })
                            .map_or_else(identity, identity);

                        nested_objects.insert(idx, legacy_last_tick);
                    }
                    _ => nested_objects.push(legacy_last_tick),
                };

                attrs.max_combo += nested_objects.len() as u32;

                let lazy_travel_time = final_span_end_time - h.start_time;
                let mut end_time_min = lazy_travel_time / span_duration;

                if end_time_min % 2.0 >= 1.0 {
                    end_time_min = 1.0 - end_time_min % 1.0;
                } else {
                    end_time_min %= 1.0;
                }

                // * temporary lazy end position until a real result can be derived.
                let mut lazy_end_pos = h.pos + curve.position_at(end_time_min);

                if hr {
                    lazy_end_pos.y = 384.0 - lazy_end_pos.y;
                }

                Self {
                    time: h.start_time,
                    pos,
                    stack_height: 0.0,
                    kind: OsuObjectKind::Slider {
                        end_time,
                        end_pos,
                        lazy_end_pos,
                        nested_objects,
                    },
                }
            }
            HitObjectKind::Spinner(Spinner { duration })
            | HitObjectKind::Hold(HoldNote { duration }) => {
                attrs.n_spinners += 1;

                Self {
                    time: h.start_time,
                    pos,
                    stack_height: 0.0,
                    kind: OsuObjectKind::Spinner {
                        end_time: h.start_time + duration,
                    },
                }
            }
        }
    }

    #[inline]
    pub(crate) fn end_time(&self) -> f64 {
        match &self.kind {
            OsuObjectKind::Circle => self.time,
            OsuObjectKind::Slider { end_time, .. } => *end_time,
            OsuObjectKind::Spinner { end_time } => *end_time,
        }
    }

    #[inline]
    pub(crate) fn end_pos(&self) -> Pos {
        match &self.kind {
            OsuObjectKind::Circle | OsuObjectKind::Spinner { .. } => self.pos,
            OsuObjectKind::Slider { end_pos, .. } => *end_pos,
        }
    }

    #[inline]
    pub(crate) fn lazy_end_pos(&self, stack_offset: Pos) -> Pos {
        match &self.kind {
            OsuObjectKind::Circle | OsuObjectKind::Spinner { .. } => self.pos,
            OsuObjectKind::Slider { lazy_end_pos, .. } => *lazy_end_pos + stack_offset,
        }
    }

    #[inline]
    pub(crate) fn is_circle(&self) -> bool {
        matches!(self.kind, OsuObjectKind::Circle)
    }

    #[inline]
    pub(crate) fn is_slider(&self) -> bool {
        matches!(self.kind, OsuObjectKind::Slider { .. })
    }

    #[inline]
    pub(crate) fn is_spinner(&self) -> bool {
        matches!(self.kind, OsuObjectKind::Spinner { .. })
    }
}
