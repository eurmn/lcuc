# ARAM Mayhem — overlay + calculation rework (v3)

Status: plan only. No code has been changed.

---

## 1. Goal

Replace the ARAM Mayhem overlay's three divergent calculation paths with one tested,
closed-form statistical engine, and render its verdicts with the existing **native Rust
layered window** (no HTML over League) as a right-edge card stack plus a compact HUD.

---

## 2. Current context (verified by reading the repo, branch `rework` @ `0800d7b`)

### 2.1 What runs today

```
Live Client API (127.0.0.1:2999)
        │  1.5 s poll, live_game.rs
        ▼
services/live_game.rs  ──F8──►  services/live_overlay.rs::scan_in_game_augment_cards
                                        │  screen capture (GDI BitBlt)
                                        │  Windows.Media.Ocr multi-pass
                                        │  fuzzy match → augment ids
                                        ▼
                              services/history.rs::enrich_overlay_augment_cards
                                        │  wpa = local ad-hoc blend, item combos from bitset miner
                                        ▼
                    models::history::OverlayAugmentCard { wpa, win_rate, games, combos }
                                        │
                        ┌───────────────┴────────────────┐
                        ▼                                ▼
      services/native_overlay.rs                 (dead) LiveMayhemOverlay.svelte
      DIB + UpdateLayeredWindow                  HTML canvas renderer
      draws 3 duplicated card boxes              never mounted: no Tauri window
      at 21% / 41.5% / 62% of screen             with label "overlay" exists
```

* `src-tauri/src/bin/lcu_overlay.rs` → `native_overlay::run_detached_overlay_process()`:
  a **child process** owning one `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST`
  popup window, drawn from `OverlayRenderPayload` over stdin JSON-lines.
* `src/main.ts:11` mounts `LiveMayhemOverlay.svelte` only when
  `location.search` contains `window=overlay`. `src-tauri/tauri.conf.json` declares a
  single window and nothing creates an overlay webview → **that whole component is dead
  code**. Good: the product direction is native drawing, not HTML.
* The in-app planning screen `src/lib/components/history/MayhemChampSelectView.svelte`
  emits `api_emit_overlay_state` (`src-tauri/src/commands/champ_select.rs:427`) which is
  the only path that fills the overlay's item strip (`build_recommended_item_sprites`,
  `services/live_overlay.rs:149`). Cards do **not** depend on it.

### 2.2 Problems this plan fixes

1. **Three calculators.** `agent/mayhem_engine.rs` (the spec'd hierarchical Bayes engine,
   1 819 lines, **zero tests**), `services/history.rs::enrich_overlay_augment_cards`
   (an ad-hoc `(wpa*g + global*15)/(g+15)` blend at line 6564), and `native_overlay.rs`
   which re-sorts raw `wpa` and self-labels "#1 BEST PICK" with no uncertainty at all.
2. **Reroll math is wrong.** `mayhem_engine.rs:1140` sets
   `ev_reroll = mean(pool scores)` and compares a single card's score against it
   (`is_reroll = sc < ev_reroll`, line 1167). A reroll *draws three cards*; the correct
   comparison is the expected maximum of a 3-card draw from the unseen pool.
3. **Arbitrary constants.** `m_1 = 25`, `m_glob = 50`, `m_pair = 20`, `0.35` eligibility
   gate, `0.75` main-effect weight, `0.85` synergy weight, `λ_look ∈ {0.65,0.45,0.25,0.10}`,
   `SE` penalty `0.50`. None are estimated from data and none have tests.
4. **Overlay draws over the game's own cards**, at guessed percentages, which also risks
   OCR reading its own pixels during a rescan (BitBlt captures the composited desktop).
5. **No DPI awareness** (`SetProcessDPIAware` is never called) → misalignment at >100 %
   display scaling, and the text scratch buffer is only 640 px wide.
6. **Dead HTML overlay** (572 lines) duplicates the card layout in canvas, guaranteeing
   drift against the Rust renderer.

### 2.3 Constraints found

* Windows-only rendering; `windows-sys 0.59` already has `Win32_Graphics_Gdi`,
  `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`; new features must be
  added to `src-tauri/Cargo.toml` for HiDPI.
* Project convention (`@llm/docs/context/CURRENT_STATE.md:91`): do not run Cargo builds
  without the user's explicit go-ahead. Every `cargo` command below needs that go-ahead;
  otherwise validate with rust-analyzer diagnostics and mark the task blocked.
* Frontend check: `bun run check` (svelte-check). Build: `bun run build`.
* One Cargo crate: `src-tauri` (lib `lcu_companion_lib` + bins). All new bins go in
  `src-tauri/src/bin/` and must be declared in `src-tauri/Cargo.toml`.

---

## 3. The math (what the engine is actually estimating)

Everything below is in **log-odds** (nats) space. A probability `p` maps to
`logit(p) = ln(p / (1-p))`; back via `sigmoid(x) = 1 / (1 + e^-x)`.

### 3.1 Single resource effect (augment or item) on champion `c`

```
p_c        = (W_c + m_c * 0.5) / (N_c + m_c),      m_c = 30        champion baseline
p̂_r        = (W_rc + 0.5) / (N_rc + 1)                             Haldane-corrected rate
θ̂_r        = logit(p̂_r) - logit(p_c)                               observed lift (log-odds)
v_r        = 1/(W_rc + 0.5) + 1/(N_rc - W_rc + 0.5)                delta-method variance
β_r        = w_r * θ̂_r,   w_r = τ² / (τ² + v_r)                    EB posterior mean
Var(β_r)   = τ² v_r / (τ² + v_r)                                   posterior variance
```

`τ²` is the between-resource variance of true effects. Estimate it once per
(champion, resource type) with **DerSimonian–Laird** method of moments (no iteration):

```
w_i = 1/v_i,  θ̄ = Σ w_i θ̂_i / Σ w_i,  Q = Σ w_i (θ̂_i - θ̄)²,
τ²  = max(0, (Q - (k-1)) / (Σw_i - Σw_i²/Σw_i))         (k >= 3, else 0)
τ²  = max(τ², TAU2_FLOOR)                                TAU2_FLOOR = 0.01
```

The floor is a regularizer, not a fudge factor: with `k < 3` or a homogeneous sample the
raw estimator is 0 and every effect would collapse to 0. `0.01` corresponds to a prior
standard deviation of 0.1 log-odds (~2.5 % win rate), i.e. "no augment is assumed to be
worth more than that until evidence says otherwise".

Why this replaces `m_1 = 25`: shrinkage strength is now data-driven per sample
(`v_r` grows for small `N_rc`), instead of a single hand-tuned pseudo-count.

### 3.2 Pair interaction (item+augment, augment+augment, item+item)

```
base      = logit(p_c) + β_x + β_y                    additive independent model
γ̂_xy      = logit((W_xy + 0.5)/(N_xy + 1)) - base
v_xy      = logodds_var(W_xy, N_xy) + Var(β_x) + Var(β_y)
γ_xy      = (τ²_pair / (τ²_pair + v_xy)) * γ̂_xy,      τ²_pair = TAU2_FLOOR
```

Samples with `N_xy < 5` are skipped (`γ = 0`), which removes the old "eligibility gate"
heuristic: no data ⇒ no claimed interaction.

### 3.3 Build prediction (saturation, decaying double-counted pairs)

```
L_0     = logit(p_c)
main    = Σ_r β_r
pairs   = γ_k sorted by |γ| descending, decayed by 0.70^k
syn     = Σ_k 0.70^k γ_k
Δ       = main + syn
L       = L_0 + L_max * tanh(Δ / L_max),   L_max = 1.5
p_build = sigmoid(L),   WPA = p_build - p_c
se      = sqrt(Σ Var(β_r) + Σ (0.70^k)² Var(γ_k)) * (1 - tanh²(Δ/L_max))
band    = [sigmoid(L - z*se), sigmoid(L + z*se)],   z = 1.2815515655446004
```

`z` is the one-sided 90 % normal quantile (lower limit of an 80 % two-sided band). Keep
calling it "80 %" in UI copy but document it precisely in code.

### 3.4 Exact reroll expected value (the fix)

A reroll replaces the 3 offered cards with 3 distinct cards drawn uniformly from the
unseen pool `U_T` of the same tier (`P = |U_T|`). Scores `s_(1) ≤ … ≤ s_(P)`:

```
P(best after reroll = s_(j)) = C(j-1, 2) / C(P, 3)              j = 3..P
E[max]                      = Σ_j s_(j) * C(j-1,2) / C(P,3)
P(improve)                  = 1 - C(m, 3) / C(P, 3),   m = #|s_i <= s_best|
gain                        = E[max] - s_best
reroll if  P >= 3  and  gain > 0.02      (0.02 nats ≈ 0.5 % win rate)
```

`C(n,k)` is computed exactly in `f64` (`choose()` below) — pool sizes are ~100 per tier,
so no overflow and no `lgamma` dependency.

Worked example used as a test: pool `{0.25, 0.30, 0.35, 0.40}`, best card `0.10`
→ `E[max] = 0.3875`, `P(improve) = 1.0`, `gain = 0.2875` → reroll.

### 3.5 What we deliberately do **not** do

* No gradient boosting / logistic regression on 11 interaction families with ~100
  entities each: the data volume is far too small for that many parameters, and the
  product needs explainable badges, not a black box.
* No lookahead weight `λ_look`. It was a magic number mixing units; the overlay now
  shows the *actual* partner interaction (`SYN: X +1.1%`) instead of folding a
  hypothetical into the score.
* No `upgrade_probability` / Command Line tier-up in this pass (see Open questions).

### 3.6 Sources

* Wilson, E. B. (1927), score interval; recommended for small samples by Brown, Cai &
  DasGupta (2001), *Interval estimation for a binomial proportion*, Statistical Science
  16(2), 101–133. Used for the "is this real?" LCB in the in-app panel.
* DerSimonian, R. & Laird, N. (1986), *Meta-analysis in clinical trials*, Controlled
  Clinical Trials 7(3), 177–188 — the `Q`-based moment estimator of `τ²` used above.
* Normal–Normal empirical Bayes / James–Stein shrinkage: posterior mean
  `τ²/(τ²+v) · θ̂`.
* Sampling without replacement order statistics: `P(max = x_(j)) = C(j-1,k-1)/C(N,k)`
  (standard SRSWOR result; see any order-statistics text, e.g. Siegrist,
  *Probability, Mathematical Statistics, and Stochastic Processes*, ch. 12.4).
* Mode rules: `@llm/docs/mayhem/RULES.llm.txt` (breakpoints Lv 3/7/11/15, 3 cards per
  offering, **1 reroll per offering**, no repeat augments within a game).

---

## 4. Overlay design (native only, no HTML)

One window, three zones, all drawn by `native_overlay.rs`. Nothing is drawn inside the
region the OCR crop reads (`x 15 %–85 %`, `y 18 %–72 %`), so the overlay can never OCR
its own pixels.

```
┌────────────────────────────────────────────── game rect (gw × gh) ──┐
│                                                   ┌───────────────┐ │
│  OCR crop (15–85% x, 18–72% y)                    │ HUD capsule   │ │  340×42 @ (gw-364, 18)
│  ┌──────────────────────────────────────────┐     │ BRAND • LV 12 │ │
│  │        game's own augment cards          │     │ 1350g • F8    │ │
│  └──────────────────────────────────────────┘     ├───────────────┤ │
│                                                   │ item strip    │ │  340×70, 6×36px slots
│                                                   └───────────────┘ │
│                                                   ┌───────────────┐ │
│                                                   │ 1. LEFT PICK  │ │  320×84, gap 8
│                                                   │ Phenomenal E… │ │  stacked, vertically centred
│                                                   │ 58.2% +4.5%   │ │
│                                                   │ SYN: Nashor's │ │
│                                                   └───────────────┘ │
│                                                   ┌───────────────┐ │
│                                                   │ 2. MID REROLL │ │
│                                                   └───────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

Rules for the presentation layer (these are the "taste" decisions):

* **Never redraw the game's cards.** The game already shows them; we show a ranked list
  on the right edge and label rows `LEFT / MID / RIGHT` by OCR slot so the player maps
  row → card in one glance.
* **At most three numbers per row**: win rate, +WPA, sample size. Verdict is a word
  (`PICK`, `REROLL`, `SKIP`), not a colour alone (colour-blind safe).
* Row order is always screen order (LEFT, MID, RIGHT), never score order — spatial
  memory beats ranking. The rank number (`1.`, `2.`) carries the ordering.
* Accents: pick `#10B981`, reroll `#F59E0B`, skip `#94A3B8`, idle HUD border `#06B6D4`.
* HUD shows live vitals (champion, level, gold) and the hotkey hint; status messages and
  the scanning state temporarily replace that line (existing behaviour, keep it).

---

## 5. Architecture / approach

Add a dependency-free, fully unit-tested `mayhem_math` module (probability primitives,
effect estimation, exact reroll EV); add one adapter in `agent/mayhem_engine.rs` that
turns the in-memory synergy index into per-card decisions and attach those decisions to
`OverlayAugmentCard` at scan time; add a pure `services/overlay_presenter.rs` that turns
cards into draw-ready rows, which the native renderer consumes (HTML overlay deleted).

---

## 6. Step-by-step tasks

Conventions used below:
* All commands run from the repo root
  `C:\Users\lucas\workspace\eurmn\lcu-api\lcu-companion`.
* `CT` = `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
* Commit after every green task. Do not batch.
* Never edit `.env`, never commit `dist/`.

---

### T1 — Branch and record the baseline

```bash
git switch -c rework/mayhem-overlay-v3
bun run check 2>&1 | tail -5
CT 2>&1 | tail -20
```

Expected: branch created; record the **exact** svelte-check error/warning count and the
Cargo test summary. They are the baseline for T16 and T17 — do not "fix" pre-existing
failures, just do not add new ones.

No commit.

---

### T2 — `mayhem_math` module skeleton + failing `prob.rs` tests

Create `src-tauri/src/mayhem_math/mod.rs`:

```rust
pub mod effects;
pub mod prob;
pub mod reroll;

pub use effects::{
    estimate_tau2, pair_effect, predict_build, sample_effect, BuildPrediction, PairEffect,
    SingleEffect, L_MAX, SYNERGY_DECAY, TAU2_FLOOR,
};
pub use prob::{
    band, beta_binomial_mean, logit, logodds_variance, posterior_variance, shrink, sigmoid,
    tau_squared, wilson_lcb, Z80,
};
pub use reroll::{choose, evaluate_reroll, RerollEval};
```

Register it in `src-tauri/src/lib.rs` (after `pub mod logger;`):

```rust
pub mod mayhem_math;
```

Create empty `src-tauri/src/mayhem_math/effects.rs` and
`src-tauri/src/mayhem_math/reroll.rs` containing only a doc comment, then create
`src-tauri/src/mayhem_math/prob.rs` with **only** this test module (no functions yet):

```rust
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
        close(tau_squared(&effects, &variances), 0.03666666666666667); // (25 - 3) / 600
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
```

Verify (red):

```bash
CT mayhem_math::prob 2>&1 | tail -15
```

Expected: **compile error** `cannot find function 'sigmoid' in this scope` (and friends).
That is the failing test.

---

### T3 — Implement `prob.rs`

Prepend to `src-tauri/src/mayhem_math/prob.rs` (before the `mod tests` block):

```rust
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
    let num = p + z2 / (2.0 * games)
        - z * ((p * (1.0 - p) + z2 / (4.0 * games)) / games).sqrt();
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
```

Verify (green):

```bash
CT mayhem_math::prob 2>&1 | tail -6
```

Expected: `test result: ok. 8 passed; 0 failed`.

Commit:

```bash
git add src-tauri/src/mayhem_math src-tauri/src/lib.rs
git commit -m "feat(mayhem-math): probability primitives, EB shrinkage, DL tau-squared"
```

---

### T4 — `effects.rs` tests (red)

Add to `src-tauri/src/mayhem_math/effects.rs` (after the doc comment):

```rust
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
            SingleEffect { observed: 0.0, variance: 0.0, effect: 0.18, posterior_var: 0.02, games: 100 },
            SingleEffect { observed: 0.0, variance: 0.0, effect: -0.07, posterior_var: 0.03, games: 100 },
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
```

Run `CT mayhem_math::effects` → expect compile errors for `observed_effect`,
`estimate_tau2`, `sample_effect`, `pair_effect`, `predict_build`, `SingleEffect`,
`PairEffect`, `TAU2_FLOOR`.

---

### T5 — Implement `effects.rs`

Prepend to `src-tauri/src/mayhem_math/effects.rs`:

```rust
//! Effect estimation: single resources, pair interactions, whole-build prediction.

use std::cmp::Ordering;

use super::prob::{
    band, logit, logodds_variance, posterior_variance, shrink, sigmoid, tau_squared, Z80,
};

/// Floor for the between-effect variance. Keeps effects finite and non-zero when the
/// sample is too small or too homogeneous to estimate heterogeneity.
pub const TAU2_FLOOR: f64 = 0.01;

/// Saturation of the total log-odds shift (roughly ±62% win rate at the extremes).
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
```

Run `CT mayhem_math::effects` → `test result: ok. 7 passed; 0 failed`.

Commit:

```bash
git add src-tauri/src/mayhem_math/effects.rs
git commit -m "feat(mayhem-math): empirical Bayes single effects, pair interactions, build prediction"
```

---

### T6 — `reroll.rs` tests (red)

`src-tauri/src/mayhem_math/reroll.rs`:

```rust
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
```

Run `CT mayhem_math::reroll` → compile errors for `choose`, `evaluate_reroll`, `RerollEval`.

Note the third test: pool `{0.0,0.1,0.2,0.3,0.4}`, `C(5,3)=10`, and `P(improve) = 1 - C(4,3)/10 = 0.6`.

---

### T7 — Implement `reroll.rs`

```rust
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
```

Run `CT mayhem_math` → `test result: ok. 20 passed; 0 failed` (8 + 7 + 5).

Commit:

```bash
git add src-tauri/src/mayhem_math/reroll.rs
git commit -m "feat(mayhem-math): exact reroll EV via SRSWOR order statistics"
```

---

### T8 — Verdict assembly in the engine (pure, tested)

File: `src-tauri/src/agent/mayhem_engine.rs`.

Append before `pub fn query_mayhem_combos` and create the test module at the very end of
the file:

```rust
// --- Overlay card decisions (v3 math) ---

use crate::mayhem_math::{
    estimate_tau2, evaluate_reroll, logit, pair_effect, sample_effect, sigmoid, PairEffect,
    RerollEval, SingleEffect, TAU2_FLOOR, Z80,
};

/// Reroll is only advised when the expected log-odds gain clears this threshold
/// (0.02 nats ~ 0.5% win rate).
pub const REROLL_SIGNIFICANCE: f64 = 0.02;

/// Returns one verdict per candidate score and the reroll evaluation it used.
pub fn apply_verdicts(
    candidate_scores: &[f64],
    pool_scores: &[f64],
    draw_count: usize,
    margin: f64,
) -> (Vec<&'static str>, RerollEval) {
    let best = candidate_scores
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let eval = evaluate_reroll(pool_scores, best, draw_count);
    let reroll_best = eval.drawable && eval.gain > margin;

    let verdicts = candidate_scores
        .iter()
        .map(|score| {
            if (score - best).abs() < 1e-9 {
                if reroll_best {
                    "reroll"
                } else {
                    "pick"
                }
            } else {
                "skip"
            }
        })
        .collect();

    (verdicts, eval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_when_the_pool_cannot_beat_the_best_card() {
        let (verdicts, eval) = apply_verdicts(&[0.30, 0.10], &[0.0, 0.05], 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts, vec!["pick", "skip"]);
        assert!(!eval.drawable);
    }

    #[test]
    fn rerolls_when_the_pool_is_clearly_better() {
        let pool = [0.40, 0.35, 0.30, 0.25];
        let (verdicts, eval) = apply_verdicts(&[0.10, 0.05], &pool, 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts[0], "reroll");
        assert_eq!(verdicts[1], "skip");
        assert!(eval.gain > REROLL_SIGNIFICANCE);
        assert_eq!(eval.improve_probability, 1.0);
    }

    #[test]
    fn verdicts_track_the_highest_score_only() {
        let (verdicts, _) = apply_verdicts(&[0.05, 0.02, 0.01], &[], 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts, vec!["pick", "skip", "skip"]);
    }
}
```

Run `CT agent::mayhem_engine` → `test result: ok. 3 passed`.

Commit:

```bash
git add src-tauri/src/agent/mayhem_engine.rs
git commit -m "refactor(mayhem-engine): verdicts from exact reroll EV, drop mean-based rule"
```

---

### T9 — `decide_offered_cards` (index → per-card decisions)

Append to `src-tauri/src/agent/mayhem_engine.rs`, immediately after `apply_verdicts`
(before its test module is fine; keep code above tests):

```rust
#[derive(Debug, Clone)]
pub struct CardDecision {
    pub augment_id: u32,
    pub effect: f64,
    pub win_rate: f64,
    pub win_rate_low: f64,
    pub win_rate_high: f64,
    pub wpa: f64,
    pub games: u32,
    pub verdict: &'static str,
    pub rank: u8,
    pub reroll_gain: f64,
    pub reroll_probability: f64,
}

fn resource_counts(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    id: u32,
    is_augment: bool,
) -> (u32, u32) {
    let kind = if is_augment {
        KIND_AUG_CHAMP
    } else {
        KIND_ITEM_CHAMP
    };
    let c = index.get_counts(pack_combo_key(kind, champ_id, id, 0));
    (c.wins, c.games)
}

fn pair_counts(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    a: (u32, bool),
    b: (u32, bool),
) -> (u32, u32) {
    let (kind, entity_a, entity_b) = match (a.1, b.1) {
        (true, true) => (KIND_AUG_AUG_CHAMP, a.0.min(b.0), a.0.max(b.0)),
        (false, false) => (KIND_ITEM_ITEM_CHAMP, a.0.min(b.0), a.0.max(b.0)),
        (true, false) => (KIND_ITEM_AUG_CHAMP, a.0, b.0),
        (false, true) => (KIND_ITEM_AUG_CHAMP, b.0, a.0),
    };
    let c = index.get_counts(pack_combo_key(kind, champ_id, entity_a, entity_b));
    (c.wins, c.games)
}

fn population_tau2(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    p_champ: f64,
    is_augment: bool,
) -> f64 {
    let kind = if is_augment {
        KIND_AUG_CHAMP
    } else {
        KIND_ITEM_CHAMP
    };
    let mut samples = Vec::new();
    for (key, counts) in index.counts.iter() {
        let (k, champ, _, entity_b) = unpack_combo_key(*key);
        if k == kind && champ == champ_id && entity_b == 0 && counts.games > 0 {
            samples.push((counts.wins, counts.games));
        }
    }
    estimate_tau2(p_champ, &samples)
}

struct LockedResource {
    id: u32,
    is_augment: bool,
    effect: SingleEffect,
}

fn score_candidate(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    p_champ: f64,
    tau2_aug: f64,
    tau2_item: f64,
    locked: &[LockedResource],
    candidate: (u32, bool),
) -> (SingleEffect, f64) {
    let tau2 = if candidate.1 { tau2_aug } else { tau2_item };
    let (wins, games) = resource_counts(index, champ_id, candidate.0, candidate.1);
    let effect = sample_effect(p_champ, wins, games, tau2);
    let base = logit(p_champ);

    let mut pair_sum = 0.0;
    for locked_res in locked {
        let (pw, pn) = pair_counts(
            index,
            champ_id,
            candidate,
            (locked_res.id, locked_res.is_augment),
        );
        if pn < 5 {
            continue;
        }
        let pe: PairEffect = pair_effect(
            base + effect.effect + locked_res.effect.effect,
            pw,
            pn,
            effect.posterior_var,
            locked_res.effect.posterior_var,
            TAU2_FLOOR,
        );
        pair_sum += pe.effect;
    }

    (effect, pair_sum)
}

/// Score every offered card, rank them, and decide pick vs reroll.
pub fn decide_offered_cards(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    offered: &[u32],
    locked_augments: &[u32],
    locked_items: &[u32],
    tier: &str,
    seen: &[u32],
) -> Vec<CardDecision> {
    if offered.is_empty() {
        return Vec::new();
    }

    let p_champ = index.evaluate_champion_baseline(champ_id);
    let l0 = logit(p_champ);
    let tau2_aug = population_tau2(index, champ_id, p_champ, true);
    let tau2_item = population_tau2(index, champ_id, p_champ, false);

    let mut locked: Vec<LockedResource> = Vec::new();
    for &id in locked_augments {
        let (wins, games) = resource_counts(index, champ_id, id, true);
        locked.push(LockedResource {
            id,
            is_augment: true,
            effect: sample_effect(p_champ, wins, games, tau2_aug),
        });
    }
    for &id in locked_items {
        if id == 0 || PORO_SNAX_IDS.contains(&id) {
            continue;
        }
        let (wins, games) = resource_counts(index, champ_id, id, false);
        locked.push(LockedResource {
            id,
            is_augment: false,
            effect: sample_effect(p_champ, wins, games, tau2_item),
        });
    }

    let mut candidate_effects = Vec::with_capacity(offered.len());
    let mut candidate_scores = Vec::with_capacity(offered.len());
    for &id in offered {
        let (effect, pair_sum) = score_candidate(
            index,
            champ_id,
            p_champ,
            tau2_aug,
            tau2_item,
            &locked,
            (id, true),
        );
        candidate_effects.push(effect);
        candidate_scores.push(effect.effect + pair_sum);
    }

    let mut excluded: HashSet<u32> = seen.iter().copied().collect();
    excluded.extend(offered.iter().copied());

    let mut pool_scores = Vec::new();
    let mut dedupe: HashSet<u32> = HashSet::new();
    for meta in AUGMENT_METADATA.values() {
        if meta.tier != tier
            || !dedupe.insert(meta.numeric_id)
            || excluded.contains(&meta.numeric_id)
        {
            continue;
        }
        let (effect, pair_sum) = score_candidate(
            index,
            champ_id,
            p_champ,
            tau2_aug,
            tau2_item,
            &locked,
            (meta.numeric_id, true),
        );
        pool_scores.push(effect.effect + pair_sum);
    }

    let (verdicts, eval) = apply_verdicts(&candidate_scores, &pool_scores, 3, REROLL_SIGNIFICANCE);

    let mut decisions = Vec::with_capacity(offered.len());
    for i in 0..offered.len() {
        let effect = candidate_effects[i];
        let score = candidate_scores[i];
        let log_odds = l0 + effect.effect;
        let p = sigmoid(log_odds).clamp(0.01, 0.99);
        let se = effect.posterior_var.sqrt();
        let rank = candidate_scores.iter().filter(|s| **s > score).count() + 1;
        decisions.push(CardDecision {
            augment_id: offered[i],
            effect: effect.effect,
            win_rate: p,
            win_rate_low: sigmoid(log_odds - Z80 * se),
            win_rate_high: sigmoid(log_odds + Z80 * se),
            wpa: p - p_champ,
            games: effect.games,
            verdict: verdicts[i],
            rank: rank as u8,
            reroll_gain: eval.gain,
            reroll_probability: eval.improve_probability,
        });
    }

    decisions
}
```

Add this test to the existing `#[cfg(test)] mod tests` in the same file:

```rust
    fn synthetic_index() -> MayhemSynergyIndex {
        let mut index = MayhemSynergyIndex::default();
        index.total_matches = 1000;
        index
            .counts
            .insert(pack_combo_key(KIND_CHAMP_BASE, 1, 0, 0), WinGameCount { wins: 500, games: 1000 });
        index
            .counts
            .insert(pack_combo_key(KIND_AUG_CHAMP, 1, 40, 0), WinGameCount { wins: 60, games: 100 });
        index
            .counts
            .insert(pack_combo_key(KIND_AUG_CHAMP, 1, 41, 0), WinGameCount { wins: 45, games: 100 });
        index.id_to_augment_string.insert(40, "Test A".to_string());
        index.id_to_augment_string.insert(41, "Test B".to_string());
        index
    }

    #[test]
    fn picks_the_higher_winrate_augment() {
        let index = synthetic_index();
        let decisions = decide_offered_cards(&index, 1, &[40, 41], &[], &[], "Gold", &[]);
        assert_eq!(decisions.len(), 2);
        assert!(decisions[0].wpa > decisions[1].wpa);
        assert_eq!(decisions[0].rank, 1);
        assert_eq!(decisions[1].rank, 2);
        assert_eq!(decisions[0].verdict, "pick");
        assert_eq!(decisions[1].verdict, "skip");
    }

    #[test]
    fn card_winrate_is_anchored_on_the_champion_baseline() {
        let index = synthetic_index();
        let decisions = decide_offered_cards(&index, 1, &[40], &[], &[], "Gold", &[]);
        let p_champ = index.evaluate_champion_baseline(1);
        assert!((decisions[0].win_rate - p_champ - decisions[0].wpa).abs() < 1e-9);
        assert!(decisions[0].win_rate_low < decisions[0].win_rate);
        assert!(decisions[0].win_rate_high > decisions[0].win_rate);
    }
```

Run `CT agent::mayhem_engine` → `test result: ok. 5 passed`.

Commit:

```bash
git add src-tauri/src/agent/mayhem_engine.rs
git commit -m "feat(mayhem-engine): score offered cards with EB effects and pair interactions"
```

---

### T10 — Attach decisions at scan time (single calculation path)

**10a.** `src-tauri/src/models/history.rs:217` — extend `OverlayAugmentCard` and add
`Default`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverlayAugmentCard {
    pub slot: usize,
    pub id: String,
    pub name: String,
    pub rarity: Option<String>,
    pub wpa: f64,
    pub win_rate: f64,
    pub games: usize,
    pub synergy_items: Vec<OverlaySynergyItem>,
    pub combos: Vec<OverlayComboEntry>,

    // v3 decision payload (all optional: older JSON stays valid)
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub rank: Option<u8>,
    #[serde(default)]
    pub win_rate_low: Option<f64>,
    #[serde(default)]
    pub win_rate_high: Option<f64>,
    #[serde(default)]
    pub reroll_gain: Option<f64>,
    #[serde(default)]
    pub reroll_probability: Option<f64>,
}
```

Fix the two struct literals that now miss fields:
* `src-tauri/src/services/history.rs:6629` — add `..Default::default()` as the last field.
* `src-tauri/src/services/native_overlay.rs:1778` — add `..Default::default()`.

Add to the end of `src-tauri/src/models/history.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_card_decisions_are_optional_in_json() {
        let json = r#"{"slot":0,"id":"1","name":"X","rarity":null,"wpa":0.1,
            "win_rate":0.55,"games":10,"synergy_items":[],"combos":[]}"#;
        let card: OverlayAugmentCard = serde_json::from_str(json).expect("deserialize");
        assert!(card.decision.is_none());
        assert!(card.rank.is_none());
        assert_eq!(card.name, "X");
    }
}
```

**10b.** Make the index helper reachable:
`src-tauri/src/commands/build_engine.rs:10` change
`async fn ensure_index_ready()` → `pub(crate) async fn ensure_index_ready()`.

**10c.** `src-tauri/src/services/live_overlay.rs` — add this function above
`scan_in_game_augment_cards` and call it at the end of the Windows branch:

```rust
async fn apply_card_decisions(
    champion_id: Option<i64>,
    locked_augments: &[String],
    inventory_item_ids: &[u32],
    cards: &mut [OverlayAugmentCard],
) {
    let Some(champ_id) = champion_id.filter(|id| *id > 0) else {
        return;
    };
    if cards.is_empty() {
        return;
    }
    if crate::commands::build_engine::ensure_index_ready()
        .await
        .is_err()
    {
        return;
    }

    let tier = crate::agent::mayhem_engine::get_augment_meta(&cards[0].id).tier;
    let offered: Vec<u32> = cards
        .iter()
        .map(|c| crate::agent::mayhem_engine::parse_or_hash_augment_id(&c.id))
        .collect();
    let locked: Vec<u32> = locked_augments
        .iter()
        .map(|s| crate::agent::mayhem_engine::parse_or_hash_augment_id(s))
        .collect();

    let decisions = {
        let Ok(read) = crate::agent::mayhem_engine::MAYHEM_SYNERGY_INDEX.read() else {
            return;
        };
        let Some(index) = read.as_ref() else {
            return;
        };
        crate::agent::mayhem_engine::decide_offered_cards(
            index,
            champ_id as u16,
            &offered,
            &locked,
            inventory_item_ids,
            &tier,
            &[],
        )
    };

    for (card, decision) in cards.iter_mut().zip(decisions.iter()) {
        card.wpa = decision.wpa;
        card.win_rate = decision.win_rate;
        card.score = Some(decision.effect);
        card.decision = Some(decision.verdict.to_string());
        card.rank = Some(decision.rank);
        card.win_rate_low = Some(decision.win_rate_low);
        card.win_rate_high = Some(decision.win_rate_high);
        card.reroll_gain = Some(decision.reroll_gain);
        card.reroll_probability = Some(decision.reroll_probability);
        if card.games == 0 {
            card.games = decision.games as usize;
        }
    }
}
```

Replace the tail of `scan_in_game_augment_cards` (currently
`Ok(crate::services::history::enrich_overlay_augment_cards(champion_id, matches).await)`)
with:

```rust
        let mut cards = crate::services::history::enrich_overlay_augment_cards(champion_id, matches).await;
        let inventory = crate::services::live_game::fetch_live_game_state()
            .await
            .map(|state| {
                state
                    .items
                    .iter()
                    .map(|i| i.item_id as u32)
                    .collect::<Vec<u32>>()
            })
            .unwrap_or_default();
        apply_card_decisions(champion_id, &with_augment_ids, &inventory, &mut cards).await;
        Ok(cards)
```

Run:

```bash
CT models::history 2>&1 | tail -5
CT services::live_overlay 2>&1 | tail -8
```

Expected: history `1 passed`; live_overlay keeps its existing OCR tests passing
(`test_levenshtein_and_fuzzy_similarity`, `test_is_category_badge_noise`,
`test_match_ocr_lines_to_augments`, plus the screenshot test that may self-skip).

Commit:

```bash
git add src-tauri/src/models/history.rs src-tauri/src/services/history.rs \
        src-tauri/src/services/native_overlay.rs src-tauri/src/services/live_overlay.rs \
        src-tauri/src/commands/build_engine.rs
git commit -m "feat(overlay): attach v3 card decisions during scan"
```

---

### T11 — `overlay_presenter.rs` tests (red)

Create `src-tauri/src/services/overlay_presenter.rs` with only the test module, and add
`pub mod overlay_presenter;` to `src-tauri/src/services/mod.rs` (alphabetical, near
`native_overlay`).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::{OverlayAugmentCard, OverlayComboEntry};

    fn card(slot: usize, name: &str, decision: &str, rank: u8) -> OverlayAugmentCard {
        OverlayAugmentCard {
            slot,
            id: name.to_string(),
            name: name.to_string(),
            rarity: Some("Gold".to_string()),
            wpa: 0.045,
            win_rate: 0.582,
            games: 1240,
            synergy_items: Vec::new(),
            combos: vec![OverlayComboEntry {
                label: "Nashor's Tooth".to_string(),
                win_rate: 0.61,
                wpa: 0.011,
                games: 800,
                items: Vec::new(),
            }],
            decision: Some(decision.to_string()),
            rank: Some(rank),
            ..Default::default()
        }
    }

    #[test]
    fn rows_follow_screen_order_not_score_order() {
        let cards = vec![card(2, "Third", "skip", 3), card(0, "First", "pick", 1)];
        let rows = build_rows(&cards, 26);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].slot, 0);
        assert_eq!(rows[1].slot, 2);
    }

    #[test]
    fn header_carries_rank_slot_and_verdict() {
        let rows = build_rows(&[card(0, "Phenomenal Evil", "pick", 1)], 26);
        assert_eq!(rows[0].header, "1. LEFT  •  PICK");
        assert_eq!(rows[0].name, "Phenomenal Evil");
        assert_eq!(rows[0].stats, "58.2% WR   +4.5%   1.2k");
        assert_eq!(rows[0].footer, "SYN: Nashor's Tooth +1.1%");
        assert_eq!(rows[0].accent, ACCENT_PICK);
    }

    #[test]
    fn verdicts_map_to_expected_accents() {
        let pick = build_rows(&[card(0, "A", "pick", 1)], 26);
        let reroll = build_rows(&[card(1, "B", "reroll", 1)], 26);
        let skip = build_rows(&[card(2, "C", "skip", 2)], 26);
        let wait = build_rows(&[card(0, "D", "", 0)], 26);
        assert_eq!(pick[0].accent, ACCENT_PICK);
        assert_eq!(reroll[0].accent, ACCENT_REROLL);
        assert_eq!(skip[0].accent, ACCENT_SKIP);
        assert_eq!(wait[0].accent, ACCENT_SKIP);
        assert!(wait[0].header.ends_with("WAIT"));
    }

    #[test]
    fn long_names_are_truncated_with_an_ellipsis() {
        let rows = build_rows(&[card(0, "Supercalifragilisticexpialidocious", "pick", 1)], 10);
        assert_eq!(rows[0].name.chars().count(), 10);
        assert!(rows[0].name.ends_with('…'));
    }

    #[test]
    fn games_are_compacted_above_one_thousand() {
        assert_eq!(compact_games(999), "999");
        assert_eq!(compact_games(1000), "1.0k");
        assert_eq!(compact_games(1240), "1.2k");
    }

    #[test]
    fn hud_text_shows_vitals_and_hotkeys() {
        assert_eq!(
            build_hud_text("Jinx", 12, 1350.4, true),
            "JINX  •  LV 12  •  1350g  •  F8 RESCAN   F7 HIDE"
        );
        assert_eq!(build_hud_text("", 0, 0.0, false), "ARAM MAYHEM  •  F8 SCAN");
    }
}
```

Run `CT overlay_presenter` → compile errors for `build_rows`, `build_hud_text`,
`compact_games`, `ACCENT_*`.

---

### T12 — Implement `overlay_presenter.rs`

Prepend the following to the file (above the test module):

```rust
//! Turns scored cards into draw-ready rows for the native overlay.
//! Pure: no Win32, no IO, so it can be unit-tested on any platform.

use serde::{Deserialize, Serialize};

use crate::models::history::OverlayAugmentCard;

pub const ACCENT_PICK: [u8; 3] = [16, 185, 129];
pub const ACCENT_REROLL: [u8; 3] = [245, 158, 11];
pub const ACCENT_SKIP: [u8; 3] = [148, 163, 184];
pub const ACCENT_IDLE: [u8; 3] = [6, 182, 212];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayCardRow {
    pub slot: usize,
    pub accent: [u8; 3],
    pub header: String,
    pub name: String,
    pub stats: String,
    pub footer: String,
}

fn slot_label(slot: usize) -> &'static str {
    match slot {
        0 => "LEFT",
        1 => "MID",
        _ => "RIGHT",
    }
}

fn verdict_label_and_accent(decision: Option<&str>) -> (&'static str, [u8; 3]) {
    match decision.unwrap_or("") {
        "pick" => ("PICK", ACCENT_PICK),
        "reroll" => ("REROLL", ACCENT_REROLL),
        "skip" => ("SKIP", ACCENT_SKIP),
        _ => ("WAIT", ACCENT_SKIP),
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

pub fn compact_games(games: usize) -> String {
    if games >= 1000 {
        format!("{:.1}k", games as f64 / 1000.0)
    } else {
        games.to_string()
    }
}

/// One row per offered card, in screen order (LEFT, MID, RIGHT).
pub fn build_rows(cards: &[OverlayAugmentCard], max_name_chars: usize) -> Vec<OverlayCardRow> {
    let mut ordered: Vec<&OverlayAugmentCard> = cards.iter().take(3).collect();
    ordered.sort_by_key(|card| card.slot);

    ordered
        .iter()
        .map(|card| {
            let (verdict, accent) = verdict_label_and_accent(card.decision.as_deref());
            let rank = card.rank.unwrap_or(0);
            let header = if rank > 0 {
                format!("{}. {}  •  {}", rank, slot_label(card.slot), verdict)
            } else {
                format!("{}  •  {}", slot_label(card.slot), verdict)
            };
            let stats = format!(
                "{:.1}% WR   {:+.1}%   {}",
                card.win_rate * 100.0,
                card.wpa * 100.0,
                compact_games(card.games)
            );
            let footer = card
                .combos
                .first()
                .map(|combo| {
                    format!(
                        "SYN: {} {:+.1}%",
                        truncate_chars(&combo.label, 22),
                        combo.wpa * 100.0
                    )
                })
                .unwrap_or_default();

            OverlayCardRow {
                slot: card.slot,
                accent,
                header,
                name: truncate_chars(&card.name, max_name_chars),
                stats,
                footer,
            }
        })
        .collect()
}

pub fn build_hud_text(champion: &str, level: u32, gold: f64, has_cards: bool) -> String {
    let champion = if champion.trim().is_empty() {
        "ARAM MAYHEM"
    } else {
        champion.trim()
    };
    let mut text = champion.to_uppercase();
    if level > 0 {
        text.push_str(&format!("  •  LV {level}"));
    }
    if gold > 0.0 {
        text.push_str(&format!("  •  {:.0}g", gold));
    }
    text.push_str(if has_cards {
        "  •  F8 RESCAN   F7 HIDE"
    } else {
        "  •  F8 SCAN"
    });
    text
}
```

Run `CT overlay_presenter` → `test result: ok. 6 passed`.

Commit:

```bash
git add src-tauri/src/services/overlay_presenter.rs src-tauri/src/services/mod.rs
git commit -m "feat(overlay): pure presenter for card rows and HUD text"
```

---

### T13 — Payload carries rows + HUD text; live vitals feed them

**13a.** `src-tauri/src/services/native_overlay.rs:30` — extend the payload:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverlayRenderPayload {
    pub visible: bool,
    pub scanning: bool,
    pub champion_name: String,
    pub strategy_label: String,
    pub cards: Vec<OverlayAugmentCard>,
    #[serde(default)]
    pub rows: Vec<crate::services::overlay_presenter::OverlayCardRow>,
    #[serde(default)]
    pub hud_text: String,
    pub sprites: Vec<OverlayPngSprite>,
    #[serde(default)]
    pub status_message: Option<String>,
    #[serde(default)]
    pub is_error: bool,
}
```

Update the payload literal in `native_overlay.rs`'s test to end with
`..Default::default()` (replacing `status_message: None, is_error: false,`).

**13b.** `src-tauri/src/services/live_overlay.rs` — add vitals state + setter:

```rust
static OVERLAY_LEVEL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static OVERLAY_GOLD: Mutex<f64> = Mutex::new(0.0);

pub fn set_overlay_vitals(level: u32, gold: f64) {
    let changed = OVERLAY_LEVEL.swap(level, Ordering::Relaxed) != level
        || (OVERLAY_GOLD.lock().map(|g| (*g - gold).abs() > 1.0).unwrap_or(false));
    if let Ok(mut g) = OVERLAY_GOLD.lock() {
        *g = gold;
    }
    if changed {
        refresh_native_overlay();
    }
}
```

Note: `Mutex::new` is not `const` in older stable; if the compiler rejects the static,
use `lazy_static!` in the existing block instead (the file already uses `lazy_static`).

**13c.** In `refresh_native_overlay` (`live_overlay.rs:93`), replace the payload build with:

```rust
    let level = OVERLAY_LEVEL.load(Ordering::Relaxed);
    let gold = OVERLAY_GOLD.lock().map(|g| *g).unwrap_or(0.0);
    let rows = crate::services::overlay_presenter::build_rows(&cards, 26);
    let hud_text = crate::services::overlay_presenter::build_hud_text(&champ, level, gold, !cards.is_empty());

    let payload = crate::services::native_overlay::OverlayRenderPayload {
        visible: true,
        scanning,
        champion_name: champ,
        strategy_label: strat,
        cards,
        rows,
        hud_text,
        sprites,
        status_message,
        is_error,
    };
```

**13d.** `src-tauri/src/services/live_game.rs:289` — after
`set_overlay_champion_and_strategy(...)`, add:

```rust
                            crate::services::live_overlay::set_overlay_vitals(state.level, state.current_gold);
```

Run:

```bash
CT native_overlay 2>&1 | tail -8
CT services::live_overlay 2>&1 | tail -6
```

Expected: `native_overlay` 4 passed (IPC roundtrip, layout 1080/1440, BGRA blit, item box
blit); `live_overlay` unchanged.

Commit:

```bash
git add src-tauri/src/services/native_overlay.rs src-tauri/src/services/live_overlay.rs \
        src-tauri/src/services/live_game.rs
git commit -m "feat(overlay): payload rows and live vitals for the HUD"
```

---

### T14 — Native renderer: right-edge row stack (delete the duplicated cards)

**14a.** Replace the HUD text `} else { … }` branch (currently
`native_overlay.rs:1001-1021`, the block that formats
`"LCU OVERLAY • {} • [F8] SCAN [F7] HIDE"`) with:

```rust
            } else {
                let text = if payload.hud_text.is_empty() {
                    "ARAM MAYHEM  •  F8 SCAN".to_string()
                } else {
                    payload.hud_text.clone()
                };
                tr.draw_text_line(
                    buf,
                    w as u32,
                    h as u32,
                    &text,
                    bx + 14,
                    by + 12,
                    FontKind::Bold,
                    220,
                    225,
                    235,
                    Some(bw - 28),
                );
            }
```

**14b.** Delete the whole block that starts at the comment
`// 3. Draw Augment Cards` (line 1184) and ends at the `}` closing
`if !payload.cards.is_empty() {` (line 1398). Keep the final `}` (line 1399) that closes
`render_to_buffer`. Confirm the markers first:

```bash
rg -n "// 3. Draw Augment Cards|fn render\(&mut self\)" src-tauri/src/services/native_overlay.rs
```

Expected: two line numbers ~200 apart; the region between them (excluding the last brace)
is removed.

Paste in its place:

```rust
        // 3. Augment verdict rows: fixed right-edge stack, never over the game's cards.
        if !payload.rows.is_empty() {
            let row_w: i32 = 320;
            let row_h: i32 = 84;
            let gap: i32 = 8;
            let count = payload.rows.len() as i32;
            let stack_h = count * (row_h + gap) - gap;
            let x = (layout.game_width - row_w - 24).max(0);
            let y0 = ((layout.game_height - stack_h) / 2).max(0);

            for (i, row) in payload.rows.iter().enumerate() {
                let rect = [x, y0 + i as i32 * (row_h + gap), row_w, row_h];
                fill_premultiplied_rect(buf, w as u32, h as u32, rect, 9, 13, 22, 235, 8);
                draw_premultiplied_border(
                    buf,
                    w as u32,
                    h as u32,
                    rect,
                    2,
                    row.accent[0],
                    row.accent[1],
                    row.accent[2],
                    240,
                    8,
                );
                fill_premultiplied_rect(
                    buf,
                    w as u32,
                    h as u32,
                    [rect[0], rect[1] + 8, 3, row_h - 16],
                    row.accent[0],
                    row.accent[1],
                    row.accent[2],
                    255,
                    1,
                );

                if let Some(ref mut tr) = self.text_renderer {
                    let pad = 18;
                    let text_w = row_w - pad - 12;
                    let tx = rect[0] + pad;
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &row.header,
                        tx,
                        rect[1] + 8,
                        FontKind::Bold,
                        row.accent[0],
                        row.accent[1],
                        row.accent[2],
                        Some(text_w),
                    );
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &row.name,
                        tx,
                        rect[1] + 26,
                        FontKind::Title,
                        241,
                        245,
                        249,
                        Some(text_w),
                    );
                    tr.draw_text_line(
                        buf,
                        w as u32,
                        h as u32,
                        &row.stats,
                        tx,
                        rect[1] + 48,
                        FontKind::Normal,
                        209,
                        213,
                        219,
                        Some(text_w),
                    );
                    if !row.footer.is_empty() {
                        tr.draw_text_line(
                            buf,
                            w as u32,
                            h as u32,
                            &row.footer,
                            tx,
                            rect[1] + 64,
                            FontKind::Small,
                            148,
                            163,
                            184,
                            Some(text_w),
                        );
                    }
                }
            }
        }
```

**14c.** Add a headless preview so the layout can be inspected without launching League.
At the end of `native_overlay.rs` (above the test module), add:

```rust
#[cfg(windows)]
pub fn render_overlay_png(
    payload: &OverlayRenderPayload,
    width: i32,
    height: i32,
    path: &str,
) -> Result<(), String> {
    let layout = compute_default_anchor_layout(0, 0, width, height);
    let mut context = OverlayWinContext::new(std::ptr::null_mut());
    let total = (width * height * 4) as usize;
    let mut buffer = vec![0u8; total];
    context.render_to_buffer(&mut buffer, &layout, payload);

    // Premultiplied BGRA -> opaque RGBA composited over a dark background.
    let mut out = vec![0u8; total];
    for i in (0..total).step_by(4) {
        let alpha = buffer[i + 3] as u32;
        let inv = 255 - alpha;
        out[i] = (buffer[i + 2] as u32 + 18 * inv / 255).min(255) as u8;
        out[i + 1] = (buffer[i + 1] as u32 + 16 * inv / 255).min(255) as u8;
        out[i + 2] = (buffer[i] as u32 + 16 * inv / 255).min(255) as u8;
        out[i + 3] = 255;
    }
    image::save_buffer(path, &out, width as u32, height as u32, image::ColorType::Rgba8)
        .map_err(|e| e.to_string())
}
```

Create `src-tauri/src/bin/overlay_preview.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    {
        use lcu_companion_lib::models::history::{OverlayAugmentCard, OverlayComboEntry};
        use lcu_companion_lib::services::native_overlay::{render_overlay_png, OverlayRenderPayload};
        use lcu_companion_lib::services::overlay_presenter::{build_hud_text, build_rows};

        let sample = |slot: usize, name: &str, decision: &str, rank: u8, wr: f64, wpa: f64| {
            OverlayAugmentCard {
                slot,
                id: name.to_string(),
                name: name.to_string(),
                rarity: Some("Gold".to_string()),
                wpa,
                win_rate: wr,
                games: 1240,
                synergy_items: Vec::new(),
                combos: vec![OverlayComboEntry {
                    label: "Nashor's Tooth".to_string(),
                    win_rate: 0.61,
                    wpa: 0.011,
                    games: 800,
                    items: Vec::new(),
                }],
                decision: Some(decision.to_string()),
                rank: Some(rank),
                ..Default::default()
            }
        };

        let cards = vec![
            sample(0, "Phenomenal Evil", "pick", 1, 0.582, 0.045),
            sample(1, "Marksmage", "skip", 2, 0.548, 0.012),
            sample(2, "Ethereal Weapon", "skip", 3, 0.521, -0.008),
        ];
        let payload = OverlayRenderPayload {
            visible: true,
            champion_name: "Brand".to_string(),
            hud_text: build_hud_text("Brand", 12, 1350.0, true),
            rows: build_rows(&cards, 26),
            ..Default::default()
        };

        render_overlay_png(&payload, 1920, 1080, "overlay-preview.png").expect("render preview");
        println!("wrote overlay-preview.png");
    }
}
```

Add to `src-tauri/Cargo.toml` after the `lcu-build-engine` entry:

```toml
[[bin]]
name = "overlay-preview"
path = "src/bin/overlay_preview.rs"
```

Verify:

```bash
cd src-tauri && cargo run --bin overlay-preview 2>&1 | tail -3 && cd ..
```

Expected: `wrote overlay-preview.png`. Open
`C:\Users\lucas\workspace\eurmn\lcu-api\lcu-companion\src-tauri\overlay-preview.png`
and confirm: black 1920×1080 canvas, three stacked rounded rows in the right third,
`1. LEFT • PICK` in green, names and stats legible, footer `SYN: Nashor's Tooth +1.1%`,
and nothing drawn in the left two thirds.

Run `CT native_overlay` → 4 passed (payload roundtrip still green after the `Default`
change).

Commit:

```bash
git add src-tauri/src/services/native_overlay.rs src-tauri/src/bin/overlay_preview.rs \
        src-tauri/Cargo.toml
git commit -m "feat(overlay): native right-edge verdict stack + offscreen PNG preview"
```

---

### T15 — DPI awareness (fixes misalignment at >100% scaling)

`src-tauri/Cargo.toml`: add `"Win32_UI_HiDpi"` to the `windows-sys` feature list.

`src-tauri/src/services/native_overlay.rs` in `run_detached_overlay_process`, at the top
of the `#[cfg(windows)]` block, before `GetModuleHandleW`:

```rust
        unsafe {
            // Must run before any window/DC is created: without it GetClientRect is
            // virtualised at 125%/150% scaling and the overlay drifts off the game.
            windows_sys::Win32::UI::HiDpi::SetProcessDPIAware();
        }
```

Verify:

```bash
cd src-tauri && cargo build --bin lcu-overlay 2>&1 | tail -3 && cd ..
```

Expected: `Finished` with no errors. If `SetProcessDPIAware` is not exported by the
`windows-sys` version in `Cargo.lock`, use
`SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` from the same
module instead and log both results.

Manual check: set Windows display scaling to 125 %, start the app, enter a game, press F8.
The HUD and rows must sit flush against the game's right edge (24 px margin), not offset.

Commit:

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services/native_overlay.rs
git commit -m "fix(overlay): declare per-process DPI awareness before window creation"
```

---

### T16 — Delete the dead HTML overlay

1. `git rm src/lib/components/overlay/LiveMayhemOverlay.svelte` (the directory becomes
   empty; git removes it).
2. Rewrite `src/main.ts` to:

```ts
import { mount } from "svelte";
import App from "./App.svelte";
import "./styles.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Root #app element not found in DOM");
}

document.body.classList.add("bg-gray-900");
target.classList.add("bg-gray-900");

const app = mount(App, { target });

export default app;
```

3. Add the new optional fields to the shared type
   `src/lib/types/champ_select.ts:173` so the in-app debug views can read them:

```ts
export interface OverlayAugmentCard {
  slot: number;
  id: string;
  name: string;
  rarity?: string | null;
  icon_url: string;
  win_rate: number;
  games: number;
  wpa: number;
  tooltip?: string | null;
  synergy_items: OverlaySynergyItem[];
  combos: OverlayComboEntry[];
  score?: number | null;
  decision?: "pick" | "reroll" | "skip" | string | null;
  rank?: number | null;
  win_rate_low?: number | null;
  win_rate_high?: number | null;
  reroll_gain?: number | null;
  reroll_probability?: number | null;
}
```

Verify:

```bash
bun run check 2>&1 | tail -5
```

Expected: the same error/warning count recorded in T1 (ideally `0 errors`), with no new
diagnostics and no unresolved import of `LiveMayhemOverlay`.

Manual: `bun run dev` still boots the main window; the tray/close behaviour is unchanged.

Commit:

```bash
git add -A src/lib/components/overlay src/main.ts src/lib/types/champ_select.ts
git commit -m "refactor(overlay): remove dead HTML overlay renderer"
```

---

### T17 — End-to-end validation + docs

1. Full suites:

```bash
CT 2>&1 | tail -25
bun run check 2>&1 | tail -5
bun run build 2>&1 | tail -5
```

Expected: no new failures versus T1; every `mayhem_math::*`, `overlay_presenter::*`,
`agent::mayhem_engine::*` test green; `vite build` completes with `built in ...`.

2. Live smoke test (manual, 10 minutes):
   * Start the app, open League, enter an ARAM Mayhem game.
   * Confirm the HUD capsule shows `CHAMPION • LV n • gold g • F8 SCAN`.
   * At level 3 open the augment selection screen, press F8:
     HUD flips to amber `SCANNING AUGMENTS...`, then three rows appear on the right within
     ~2 s with `PICK` on exactly one row and `REROLL` only if the pool is genuinely better.
   * Press F7: rows disappear, HUD stays. Press F8 again: rows return.
   * Alt-tab to another window: overlay hides (existing `is_foreground` gate).
   * Take a screenshot of the game + overlay and compare row labels LEFT/MID/RIGHT with
     the actual card positions.

3. Append a short section to `@llm/docs/mayhem/RULES.llm.txt`:

```
## Recommender model (v3)
- Effects in log-odds, empirical-Bayes shrunk (Normal-Normal, DL tau2, floor 0.01).
- Build prediction: L = logit(p_champ) + sum(beta) + sum(0.70^k * gamma), tanh-saturated at 1.5.
- Reroll EV: exact expected max of a 3-card SRSWOR draw from the unseen tier pool.
- Overlay renders natively (DIB + GDI text); no HTML, no duplicate card boxes.
```

Commit:

```bash
git add @llm/docs/mayhem/RULES.llm.txt
git commit -m "docs(mayhem): document v3 recommender model and overlay architecture"
```

---

## 7. Tests / validation summary

| Layer | File | Tests | Command |
|---|---|---|---|
| Probability primitives | `mayhem_math/prob.rs` | 8 | `CT mayhem_math::prob` |
| Effects / build | `mayhem_math/effects.rs` | 7 | `CT mayhem_math::effects` |
| Reroll EV | `mayhem_math/reroll.rs` | 5 | `CT mayhem_math::reroll` |
| Verdict rule | `agent/mayhem_engine.rs` | 3 | `CT agent::mayhem_engine` |
| Card decisions | `agent/mayhem_engine.rs` | 2 | `CT agent::mayhem_engine` |
| Model compat | `models/history.rs` | 1 | `CT models::history` |
| Presenter | `services/overlay_presenter.rs` | 6 | `CT overlay_presenter` |
| Overlay IPC/layout | `services/native_overlay.rs` | 4 (existing, kept) | `CT native_overlay` |
| Visual | `overlay-preview.png` | manual | `cargo run --bin overlay-preview` |

Invariants that must hold (each has a test above):
* Empty build == champion baseline, WPA 0.
* `p_low <= p <= p_high` for every prediction and card.
* Shrinkage is monotone: more games ⇒ larger `|effect|` for the same observed lift.
* `E[max]` matches brute-force enumeration for a 5-element pool.
* `P(improve) = 0` when every pool card is worse than the current best.
* Verdicts mark exactly one `pick`/`reroll` and all others `skip`.

---

## 8. Risks and tradeoffs

1. **Cargo execution policy.** `CURRENT_STATE.md:91` forbids running Cargo without the
   user's go-ahead. Every `CT`/`cargo run` above is gated on that. If it is denied, the
   task is blocked at T3 — do not invent passing output.
2. **`τ²` floor of 0.01 is a modelling choice.** It is the only remaining tuned constant
   besides `L_MAX` and `SYNERGY_DECAY`. Consequence: rare augments show small effects
   until they accumulate games. Tune only with a dedicated test that shows the ranking
   change, never ad hoc.
3. **Overlay self-capture during OCR.** The row stack sits at
   `x = gw - 344 … gw - 24`, outside the OCR crop (`15 %–85 %`). Never move rows into
   the centre third; if you must, hide the overlay for the capture.
4. **`OverlayAugmentCard` gains fields.** Two struct literals are enumerated (T10) and
   `Default` is added; the serde test pins backward compatibility. Any missed literal is
   a compile error, not silent breakage.
5. **DPI awareness change** can shift the overlay on already-scaled setups. It is one call
   at process start; if visuals regress, revert T15 alone.
6. **Reroll verdict is advisory.** The live client does not expose whether the reroll was
   already used this round. The row says "reroll" meaning "the pool is better"; the
   player still owns the decision. Do not pretend to track state.
7. **Scan latency** grows by one local HTTP call (`fetch_live_game_state`, 800 ms timeout)
   plus ~1 ms of scoring. OCR dominates (~hundreds of ms). Budget: keep
   `decide_offered_cards` under 20 ms for 100 pool candidates × 6 locked resources.
8. **Existing dead weight left in place on purpose:** dual snake_case + camelCase DTO
   mirrors, `card_rects` in `OverlayAnchorLayout` (no longer drawn), and
   `api_emit_overlay_state`. Removing them is a separate cleanup with its own risk; the
   `card_rects` assertions in `native_overlay` tests keep passing untouched.

---

## 9. Out of scope / follow-ups

1. **Make the overlay independent of the main window.** Today the item strip is only
   populated when `MayhemChampSelectView` emits `api_emit_overlay_state`. Follow-up: in
   `live_game.rs`, every ~10 s while in game, compute the recommendation from the engine
   in `spawn_blocking` and push the item sprites directly. This is the single biggest
   remaining structural weakness.
2. **Command Line / Golden Reroll** tier upgrade probability: reintroduce as an explicit
   toggle in the in-app panel only, using the same `evaluate_reroll` on the `T+1` pool.
3. **Augment icons in rows.** `OverlayPngSprite` already supports PNG blits; add
   `perks/champion-augments/<id>.png` sprites (36 px) to the left of each row once the
   text layout is settled.
4. **Reroll state tracking** (see risk 6) if Riot ever exposes the reroll counter.
5. **Tune `m_c = 30`** champion-baseline prior from data (it is currently inherited from
   the previous engine).
