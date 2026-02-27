use crate::{model::mods::GameMods, osu::OsuDifficultyAttributes, util::float_ext::FloatExt};

pub struct Relax;

pub struct RelaxNerf {
    pub aim_multiplier: f64,
    pub speed_exponent: f64,
}

impl Default for RelaxNerf {
    fn default() -> Self {
        Self {
            aim_multiplier: 1.0,
            speed_exponent: 1.1,
        }
    }
}

impl Relax {
    /// Applies a nerf to scores with Relax when stream difficulty exceeds aim difficulty.
    /// lower ratio => heavier nerf on both speed and accuracy performance values.
    /// NOTE: logic copied from akatsuki's, but more harsher.
    /// NOTE: I won't intefere with speed deviation, since it's too harsh.
    pub fn calculate(
        mods: &GameMods,
        attrs: &OsuDifficultyAttributes,
        total_hits: f64,
        acc: f64,
    ) -> RelaxNerf {
        if !mods.rx() {
            return RelaxNerf::default();
        }

        let streams_nerf = attrs.aim / attrs.speed;

        let speed_density = if total_hits > 0.0 {
            attrs.speed_note_count / total_hits
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

        // Relax completely removes tapping skill from the equation,
        // so speed-based PP should scale weaker than normal plays.
        // The 0.83 base is (stolen from akatsuki's) arbitrary but gives a good scaling.
        let speed_exponent = 0.83 * acc_depression;

        RelaxNerf {
            aim_multiplier,
            speed_exponent,
        }
    }
}
