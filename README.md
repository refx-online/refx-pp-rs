[![crates.io](https://img.shields.io/crates/v/rosu-pp.svg)](https://crates.io/crates/rosu-pp) [![docs](https://docs.rs/rosu-pp/badge.svg)](https://docs.rs/rosu-pp)

# rosu-pp

<!-- cargo-rdme start -->

Library to calculate difficulty and performance attributes for all [osu!] gamemodes.

A large part of `rosu-pp` is a port of [osu!lazer]'s difficulty and performance calculation
with emphasis on a precise translation to Rust for the most [accurate results](#accuracy)
while also providing a significant [boost in performance](#speed).

Last commits of the ported code:
  - [osu!lazer] : `79b737bc270c8361261a9edd43b380f5326c3848` (2025-02-27)
  - [osu!tools] : `152c5d90f73f4d7eabcf4047ecb939c1b621db85` (2025-02-28)

News posts of the latest updates: <https://osu.ppy.sh/home/news/2025-03-06-performance-points-star-rating-updates>

### Usage

```rust
// Decode the map
let map = refx_pp::Beatmap::from_path("./resources/2785319.osu").unwrap();

// Whereas osu! simply times out on malicious maps, rosu-pp does not. To
// prevent potential performance/memory issues, it is recommended to check
// beforehand whether a map is too suspicious for further calculation.
if let Err(sus) = map.check_suspicion() {
    panic!("{sus:?}");
}

// Calculate difficulty attributes
let diff_attrs = refx_pp::Difficulty::new()
    .mods(8 + 16) // HDHR
    .calculate(&map);

let stars = diff_attrs.stars();

// Calculate performance attributes
let perf_attrs = refx_pp::Performance::new(diff_attrs)
    // To speed up the calculation, we used the previous attributes.
    // **Note** that this should only be done if the map and all difficulty
    // settings stay the same, otherwise the final attributes will be incorrect!
    .mods(24) // HDHR, must be the same as before
    .combo(789)
    .accuracy(99.2)
    .misses(2)
    .calculate();

let pp = perf_attrs.pp();

// Again, we re-use the previous attributes for maximum efficiency.
let max_pp = perf_attrs.performance()
    .mods(24) // Still the same
    .calculate()
    .pp();

println!("Stars: {stars} | PP: {pp}/{max_pp}");
```

### Gradual calculation

Gradually calculating attributes provides an efficient way to process each hitobject
separately and calculate the attributes only up to that point.

For difficulty attributes, there is `GradualDifficulty` which implements `Iterator`
and for performance attributes there is `GradualPerformance` which requires the current
score state.

```rust
use refx_pp::{Beatmap, GradualPerformance, Difficulty, any::ScoreState};

let map = Beatmap::from_path("./resources/1028484.osu").unwrap();

let mut gradual = Difficulty::new()
    .mods(16 + 64) // HRDT
    .clock_rate(1.2)
    .gradual_performance(&map);

let mut state = ScoreState::new(); // empty state, everything is on 0.

// The first 10 hitresults are 300s
for _ in 0..10 {
    state.n300 += 1;
    state.max_combo += 1;
    let attrs = gradual.next(state.clone()).unwrap();
    println!("PP: {}", attrs.pp());
}

// Fast-forward to the end
state.max_combo = ...
state.n300 = ...
state.n_katu = ...
...
let attrs = gradual.last(state).unwrap();
println!("PP: {}", attrs.pp());
```

### Accuracy

`rosu-pp` was tested against millions of real scores and delivered
values that matched osu!lazer perfectly down to the last decimal place.

However, there is one small caveat: the values are only this precise on debug mode.
On release mode, Rust's compiler performs optimizations that produce the tiniest discrepancies
due to floating point inaccuracies. With this in mind, `rosu-pp` is still as accurate as can
be without targeting the .NET compiler itself.
Realistically, the inaccuracies in release mode are negligibly small.

### Speed

An important factor for `rosu-pp` is the calculation speed. Optimizations and an accurate translation
unfortunately don't always go hand-in-hand. Nonetheless, performance improvements are still
snuck in wherever possible, providing a significantly faster runtime than the native C# code.

Results of a rudimentary [benchmark] of osu!lazer and rosu-pp:
```txt
osu!lazer:
Decoding maps:            Median: 325.18ms | Mean: 325.50ms
Calculating difficulties: Median: 568.63ms | Mean: 575.97ms
Calculating performances: Median: 256.00µs | Mean: 240.40µs

rosu-pp:
Decoding maps:            Median: 46.03ms | Mean: 47.13ms
Calculating difficulties: Median: 82.11ms | Mean: 84.27ms
Calculating performances: Median: 40.57µs | Mean: 43.41µs
```

### Features

| Flag          | Description         | Dependencies
| ------------- | ------------------- | ------------
| `default`     | No features enabled |
| `raw_strains` | With this feature, internal strain values will be stored in a plain `Vec`. This introduces an out-of-memory risk on maliciously long maps (see [/b/3739922](https://osu.ppy.sh/b/3739922)), but comes with a ~5% gain in performance. |
| `sync`        | Some gradual calculation types can only be shared across threads if this feature is enabled. This feature adds a small performance penalty. |
| `tracing`     | Any error encountered during beatmap decoding will be logged through `tracing::error`. If this feature is **not** enabled, errors will be ignored. | [`tracing`]

### Bindings

Using `rosu-pp` from other languages than Rust:
- Python: [refx-pp-py]

[osu!]: https://osu.ppy.sh/home
[osu!lazer]: https://github.com/ppy/osu
[osu!tools]: https://github.com/ppy/osu-tools
[`tracing`]: https://docs.rs/tracing
[refx-pp-py]: https://github.com/refx-online/refx-pp-py
[benchmark]: https://gist.github.com/MaxOhn/625af10011f6d7e13a171b08ccf959ff

<!-- cargo-rdme end -->
