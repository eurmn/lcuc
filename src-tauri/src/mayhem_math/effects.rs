//! Effect estimation: single resources, pair interactions, whole-build prediction.

use std::cmp::Ordering;

use super::prob::{
    band, logit, logodds_variance, posterior_variance, shrink, sigmoid, tau_squared, Z80,
};

/// Floor for the between-effect variance. Keeps effects finite and non-zero when the
/// sample is too small or too homogeneous to estimate heterogeneity.
pub const TAU2_FLOOR: f64 = 0.01;

/// Saturation of the total log-odds shift (roughly +/-62% win rate at the extremes).
pub const L_MAX: f64 = 1.5;

/// Geometric decay applied to pair interactions, strongest first.
pub const SYNERGY_DECAY: f64 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SingleEffect {
    pub observed: f64,
    pub variance: f64,
    pub effect: f64,
    pub posterior_var: f64,
    pub games: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairEffect {
    pub observed: f64,
    pub variance: f64,
    pub effect: f64,
    pub posterior_var: f64,
    pub games: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildPrediction {
    pub p: f64,
    pub wpa: f64,
    pub p_low: f64,
    pub p_high: f64,
    pub log_odds: f64,
}

fn observed_logodds_delta(base_logodds: f64, wins: u32, games: u32) -> (f64, f64) {
    let w = wins as f64;
    let n = games as f64;
    let p_hat = (w + 0.5) / (n + 1.0);
    let observed = logit(p_hat) - base_logodds;
    let variance = logodds_variance(w, (n - w).max(0.0));
    (observed, variance)
}

/// Observed log-odds lift of `wins / games` over `p_base`, plus its variance.
pub fn observed_effect(p_base: f64, wins: u32, games: u32) -> (f64, f64) {
    observed_logodds_delta(logit(p_base), wins, games)
}

/// Shrink one observed effect given the population heterogeneity `tau2`.
pub fn sample_effect(p_base: f64, wins: u32, games: u32, tau2: f64) -> SingleEffect {
    let (observed, variance) = observed_effect(p_base, wins, games);
    SingleEffect {
        observed,
        variance,
        effect: shrink(observed, variance, tau2),
        posterior_var: posterior_variance(variance, tau2),
        games,
    }
}

/// DerSimonian-Laird heterogeneity across `(wins, games)` samples, floored.
pub fn estimate_tau2(p_base: f64, samples: &[(u32, u32)]) -> f64 {
    let mut effects = Vec::with_capacity(samples.len());
    let mut variances = Vec::with_capacity(samples.len());
    for &(wins, games) in samples {
        if games == 0 {
            continue;
        }
        let (effect, variance) = observed_effect(p_base, wins, games);
        effects.push(effect);
        variances.push(variance);
    }
    tau_squared(&effects, &variances).max(TAU2_FLOOR)
}

/// Interaction of two resources whose independent log-odds is `base_logodds`.
pub fn pair_effect(
    base_logodds: f64,
    wins: u32,
    games: u32,
    var_a: f64,
    var_b: f64,
    tau2: f64,
) -> PairEffect {
    let (observed, sampling_var) = observed_logodds_delta(base_logodds, wins, games);
    let variance = sampling_var + var_a.max(0.0) + var_b.max(0.0);
    PairEffect {
        observed,
        variance,
        effect: shrink(observed, variance, tau2),
        posterior_var: posterior_variance(variance, tau2),
        games,
    }
}

/// Predict the win rate of a full loadout.
pub fn predict_build(p_champ: f64, main: &[SingleEffect], pairs: &[PairEffect]) -> BuildPrediction {
    let l0 = logit(p_champ);
    let main_sum: f64 = main.iter().map(|e| e.effect).sum();
    let main_var: f64 = main.iter().map(|e| e.posterior_var).sum();

    let mut ordered: Vec<&PairEffect> = pairs.iter().collect();
    ordered.sort_by(|a, b| {
        b.effect
            .abs()
            .partial_cmp(&a.effect.abs())
            .unwrap_or(Ordering::Equal)
    });

    let mut syn_sum = 0.0;
    let mut syn_var = 0.0;
    for (k, pe) in ordered.iter().enumerate() {
        let decay = SYNERGY_DECAY.powi(k as i32);
        syn_sum += decay * pe.effect;
        syn_var += decay * decay * pe.posterior_var;
    }

    let delta = main_sum + syn_sum;
    let saturation = (delta / L_MAX).tanh();
    let log_odds = l0 + L_MAX * saturation;
    let se = (main_var + syn_var).sqrt() * (1.0 - saturation * saturation).max(0.0);
    let p = sigmoid(log_odds).clamp(0.01, 0.99);
    let (p_low, p_high) = band(log_odds, se, Z80);

    BuildPrediction {
        p,
        wpa: p - p_champ,
        p_low,
        p_high,
        log_odds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn observed_effect_is_the_log_odds_lift() {
        let (effect, variance) = observed_effect(0.5, 12, 20);
        close(effect, 0.38566248081198456);
        close(variance, 0.1976470588235294);
    }

    #[test]
    fn tau2_is_floored_for_small_populations() {
        let tau2 = estimate_tau2(0.5, &[(12, 20), (8, 20), (5, 20), (15, 20)]);
        close(tau2, 0.5300122754240378);
        close(estimate_tau2(0.5, &[(12, 20)]), TAU2_FLOOR);
    }

    #[test]
    fn sample_effect_matches_hand_computation() {
        let eff = sample_effect(0.5, 12, 20, 0.5300122754240378);
        close(eff.effect, 0.2809087156316138);
        close(eff.posterior_var, 0.14396210210956653);
        assert_eq!(eff.games, 20);
    }

    #[test]
    fn pair_effect_shrinks_the_interaction() {
        let pe = pair_effect(0.0, 60, 100, 0.02, 0.03, TAU2_FLOOR);
        close(pe.observed, 0.40134139092430215);
        close(pe.effect, 0.03965029305131832);
        close(pe.posterior_var, 0.009012055722436144);
    }

    #[test]
    fn empty_build_equals_champion_baseline() {
        let pred = predict_build(0.52, &[], &[]);
        close(pred.p, 0.52);
        close(pred.wpa, 0.0);
        assert!(pred.p_low <= pred.p && pred.p <= pred.p_high);
    }

    #[test]
    fn build_prediction_matches_reference_numbers() {
        let main = [
            SingleEffect {
                observed: 0.0,
                variance: 0.0,
                effect: 0.18,
                posterior_var: 0.02,
                games: 100,
            },
            SingleEffect {
                observed: 0.0,
                variance: 0.0,
                effect: -0.07,
                posterior_var: 0.03,
                games: 100,
            },
        ];
        let pairs = [PairEffect {
            observed: 0.0,
            variance: 0.0,
            effect: 0.12,
            posterior_var: 0.05,
            games: 40,
        }];
        let pred = predict_build(0.52, &main, &pairs);
        close(pred.p, 0.576459753060874);
        close(pred.wpa, 0.05645975306087403);
        close(pred.p_low, 0.47810789988908514);
        close(pred.p_high, 0.6691046658698558);
    }

    #[test]
    fn positive_effect_raises_the_build_winrate() {
        let base = predict_build(0.5, &[], &[]);
        let boosted = predict_build(
            0.5,
            &[SingleEffect {
                observed: 0.0,
                variance: 0.0,
                effect: 0.25,
                posterior_var: 0.01,
                games: 100,
            }],
            &[],
        );
        assert!(boosted.p > base.p);
        assert!(boosted.p_low < boosted.p && boosted.p < boosted.p_high);
    }
}
