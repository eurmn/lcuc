//! Pure probability primitives. No IO, no globals, no time.

/// One-sided 90% normal quantile (= lower limit of an 80% two-sided band).
pub const Z80: f64 = 1.281_551_565_544_600_4;

/// Keeps logits finite without distorting realistic win rates.
#[inline]
pub fn clamp_prob(p: f64) -> f64 {
    p.clamp(1e-4, 1.0 - 1e-4)
}

#[inline]
pub fn logit(p: f64) -> f64 {
    let p = clamp_prob(p);
    (p / (1.0 - p)).ln()
}

#[inline]
pub fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let e = x.exp();
        e / (1.0 + e)
    }
}

/// Lower bound of the Wilson score interval for `wins / games`.
pub fn wilson_lcb(wins: f64, games: f64, z: f64) -> f64 {
    if games <= 0.0 {
        return 0.5;
    }
    let p = (wins / games).clamp(1e-4, 1.0 - 1e-4);
    let z2 = z * z;
    let num =
        p + z2 / (2.0 * games) - z * ((p * (1.0 - p) + z2 / (4.0 * games)) / games).sqrt();
    let den = 1.0 + z2 / games;
    (num / den).clamp(0.0, 1.0)
}

/// Variance of the log-odds of a binomial proportion (delta method, Haldane +0.5).
pub fn logodds_variance(wins: f64, losses: f64) -> f64 {
    1.0 / (wins.max(0.0) + 0.5) + 1.0 / (losses.max(0.0) + 0.5)
}

/// Beta-Binomial posterior mean.
pub fn beta_binomial_mean(wins: f64, games: f64, prior_mean: f64, prior_strength: f64) -> f64 {
    ((wins + prior_strength * prior_mean) / (games + prior_strength)).clamp(1e-4, 1.0 - 1e-4)
}

/// DerSimonian-Laird method-of-moments estimate of the between-effect variance.
pub fn tau_squared(effects: &[f64], variances: &[f64]) -> f64 {
    let k = effects.len();
    if k < 3 || variances.len() != k {
        return 0.0;
    }
    let weights: Vec<f64> = variances.iter().map(|v| 1.0 / v.max(1e-9)).collect();
    let sw: f64 = weights.iter().sum();
    if sw <= 0.0 {
        return 0.0;
    }
    let mean: f64 = effects
        .iter()
        .zip(&weights)
        .map(|(e, w)| e * w)
        .sum::<f64>()
        / sw;
    let q: f64 = effects
        .iter()
        .zip(&weights)
        .map(|(e, w)| w * (e - mean).powi(2))
        .sum();
    let denom = sw - weights.iter().map(|w| w * w).sum::<f64>() / sw;
    if denom <= 1e-9 {
        return 0.0;
    }
    ((q - (k as f64 - 1.0)) / denom).max(0.0)
}

/// Normal-Normal empirical Bayes posterior mean of a single effect.
pub fn shrink(observed: f64, variance: f64, tau2: f64) -> f64 {
    if variance <= 0.0 {
        return 0.0;
    }
    observed * (tau2 / (tau2 + variance))
}

/// Posterior variance after Normal-Normal shrinkage.
pub fn posterior_variance(variance: f64, tau2: f64) -> f64 {
    if variance <= 0.0 {
        0.0
    } else {
        tau2 * variance / (tau2 + variance)
    }
}

/// Probability band for a log-odds estimate with standard error `se`.
pub fn band(log_odds: f64, se: f64, z: f64) -> (f64, f64) {
    (sigmoid(log_odds - z * se), sigmoid(log_odds + z * se))
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
    fn logit_and_sigmoid_are_inverse() {
        for p in [0.05, 0.2, 0.5, 0.8, 0.95] {
            close(sigmoid(logit(p)), p);
        }
        close(logit(0.5), 0.0);
    }

    #[test]
    fn wilson_matches_reference_values() {
        close(wilson_lcb(17.0, 20.0, Z80), 0.7215521343756917);
        close(wilson_lcb(10.0, 40.0, Z80), 0.17330240864727037);
    }

    #[test]
    fn wilson_handles_empty_sample() {
        close(wilson_lcb(0.0, 0.0, Z80), 0.50);
    }

    #[test]
    fn logodds_variance_shrinks_with_sample_size() {
        close(logodds_variance(10.0, 10.0), 0.19047619047619047);
        assert!(logodds_variance(100.0, 100.0) < logodds_variance(10.0, 10.0));
    }

    #[test]
    fn tau_squared_recovers_known_heterogeneity() {
        let effects = [0.05, 0.10, 0.40, 0.45];
        let variances = [0.005; 4];
        close(tau_squared(&effects, &variances), 0.03666666666666667);
    }

    #[test]
    fn tau_squared_requires_three_effects() {
        close(tau_squared(&[0.1, 0.2], &[0.01, 0.01]), 0.0);
    }

    #[test]
    fn shrink_pulls_towards_zero() {
        close(shrink(0.4, 0.19, 0.01), 0.02);
        close(posterior_variance(0.19, 0.01), 0.0095);
    }

    #[test]
    fn band_is_centred_on_the_point_estimate() {
        let (lo, hi) = band(0.0, 0.1, Z80);
        assert!(lo < 0.5 && hi > 0.5);
        close(lo + hi, 1.0);
    }
}
