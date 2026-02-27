use crate::{model::mods::GameMods, util::float_ext::FloatExt};

pub struct Relax;

pub struct RelaxStreamsNerf {
    pub aim_multiplier: f64,
    pub accuracy_depression: f64,
}

impl Relax {
    /// Applies a nerf to scores with Relax when stream difficulty exceeds aim difficulty.
    /// lower ratio => heavier nerf on both speed and accuracy performance values.
    /// NOTE: logic copied from akatsuki's, but more harsher.
    /// NOTE: I won't intefere with speed deviation, since it's too harsh.
    pub fn calculate_streams_nerf(
        mods: &GameMods,
        aim: f64,
        speed: f64,
        speed_note_count: f64,
        total_hits: f64,
        acc: f64,
    ) -> Option<RelaxStreamsNerf> {
        if !mods.rx() {
            return None;
        }

        let streams_nerf = aim / speed;

        let speed_density = if total_hits > 0.0 {
            speed_note_count / total_hits
        } else {
            0.0
        };

        // NOTE: density threshold scales inversely with streams_nerf.
        let density_threshold =
            0.50 - ((1.05 - streams_nerf).max(0.0) / 1.05) * 0.45;

        let mut aim_multiplier = 1.0;
        let mut acc_depression = 1.0;

        if streams_nerf < 1.05 && speed_density > density_threshold {
            let acc_factor = (1.0 - acc).abs();

            let density_factor =
                (speed_density - density_threshold)
                / (1.0 - density_threshold);

            let density_factor = density_factor.clamp(0.0, 1.0);

            acc_depression = f64::lerp(
                0.82, (0.84 + acc_factor * 0.04).max(0.55), density_factor
            );

            aim_multiplier *= acc_depression;

            // Penalize low accuracy even more :skull:
            if acc < 0.95 {
                let acc_penalty = 1.0 - (0.95 - acc) * 0.3;
                aim_multiplier *= acc_penalty;
            }
        }

        Some(RelaxStreamsNerf {
            aim_multiplier,
            accuracy_depression: acc_depression
        })
    }

    /// Actually unecessary to have this as a separate function
    /// but for consistency with other parts of the codebase.
    pub fn calculate_adjusted_speed_exponent(mods: &GameMods, accuracy_depression: f64) -> f64 {
        if mods.rx() {
            // Relax completely removes tapping skill from the equation,
            // so speed-based PP should scale weaker than normal plays.
            // The 0.83 base is (stolen from akatsuki's) arbitrary but gives a good scaling.
            return 0.83 * accuracy_depression;
        }

        1.1
    }
}
