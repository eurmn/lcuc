//! Exact expected value of rerolling a 3-card offering (sampling without replacement).

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RerollEval {
    pub expected_best: f64,
    pub gain: f64,
    pub improve_probability: f64,
    pub pool_size: usize,
    pub drawable: bool,
}

/// C(n, k) as f64. Exact for the pool sizes a Mayhem tier can hold (~100).
pub fn choose(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut acc = 1.0;
    for i in 1..=k {
        acc *= (n - k + i) as f64 / i as f64;
    }
    acc
}

/// EV of the best card after rerolling into `pool_scores`, versus keeping `current_best`.
pub fn evaluate_reroll(pool_scores: &[f64], current_best: f64, draw_count: usize) -> RerollEval {
    let pool_size = pool_scores.len();
    if draw_count == 0 || pool_size < draw_count {
        return RerollEval {
            expected_best: current_best,
            gain: 0.0,
            improve_probability: 0.0,
            pool_size,
            drawable: false,
        };
    }

    let mut sorted = pool_scores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let total = choose(pool_size, draw_count);
    let mut expected_best = 0.0;
    for j in draw_count..=pool_size {
        expected_best += sorted[j - 1] * choose(j - 1, draw_count - 1) / total;
    }

    let not_better = sorted.iter().filter(|s| **s <= current_best).count();
    let improve_probability = 1.0 - choose(not_better, draw_count) / total;

    RerollEval {
        expected_best,
        gain: expected_best - current_best,
        improve_probability: improve_probability.clamp(0.0, 1.0),
        pool_size,
        drawable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn choose_is_exact_for_tier_sized_pools() {
        close(choose(4, 3), 4.0);
        close(choose(5, 3), 10.0);
        close(choose(3, 3), 1.0);
        close(choose(2, 3), 0.0);
    }

    #[test]
    fn expected_best_is_a_probability_weighted_maximum() {
        let ev = evaluate_reroll(&[0.0, 0.1, 0.2, 0.3], 0.15, 3);
        close(ev.expected_best, 0.275);
        close(ev.gain, 0.125);
        close(ev.improve_probability, 1.0);
        assert!(ev.drawable);
    }

    #[test]
    fn improve_probability_accounts_for_cards_already_beaten() {
        let ev = evaluate_reroll(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.35, 3);
        close(ev.expected_best, 0.35);
        close(ev.gain, 0.0);
        close(ev.improve_probability, 0.6);
    }

    #[test]
    fn matches_brute_force_enumeration() {
        let pool = [0.3, -0.1, 0.0, 0.25, 0.1];
        let ev = evaluate_reroll(&pool, 0.0, 3);
        let mut total = 0.0;
        let mut count = 0.0;
        for i in 0..pool.len() {
            for j in (i + 1)..pool.len() {
                for k in (j + 1)..pool.len() {
                    total += pool[i].max(pool[j]).max(pool[k]);
                    count += 1.0;
                }
            }
        }
        close(ev.expected_best, total / count);
    }

    #[test]
    fn reroll_is_impossible_without_enough_unseen_cards() {
        let ev = evaluate_reroll(&[0.4, 0.5], 0.1, 3);
        assert!(!ev.drawable);
        close(ev.gain, 0.0);
        close(ev.improve_probability, 0.0);
    }
}
