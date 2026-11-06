# Handoff — ARAM Mayhem overlay + calculation rework (v3)

Date: 2026-11-06
Branch: `mayhem-overlay-v3` (from `rework` @ `0800d7b`)
Commits: `ed471af` math core → `59bd1ce` engine + native overlay → `bdf2299` HTML overlay removed → `7869b6c` docs

---

## 1. State in one paragraph

The ARAM Mayhem calculator now has one statistical core (`mayhem_math`) instead of three
divergent ones, and the overlay renders its verdicts natively (layered window + GDI text)
as a right-edge row stack. The dead HTML overlay is deleted. 64 lib tests pass; the
offscreen preview renders correctly. Not yet validated in a live game, and the build
strip still depends on the in-app champ-select screen.

---

## 2. What changed

### New: `src-tauri/src/mayhem_math/`

Pure, no IO, no globals, no new crates.

| File | Contents |
|---|---|
| `prob.rs` | `logit`/`sigmoid`, `wilson_lcb`, `logodds_variance` (delta method, Haldane +0.5), `beta_binomial_mean`, `tau_squared` (DerSimonian–Laird), `shrink` / `posterior_variance` (Normal–Normal EB), `band`, `Z80 = 1.2815515655446004` |
| `effects.rs` | `observed_effect`, `sample_effect`, `estimate_tau2` (floored at `TAU2_FLOOR = 0.01`), `pair_effect`, `predict_build` (`L_MAX = 1.5`, `SYNERGY_DECAY = 0.70`) |
| `reroll.rs` | `choose`, `evaluate_reroll` — exact expected max of a 3-card draw without replacement |

Equations (log-odds space):

```
psingle   = (W + 0.5)/(N + 1);   theta = logit(p) - logit(p_champ)
v         = 1/(W+0.5) + 1/(L+0.5)
beta      = tau2/(tau2+v) * theta                   Normal-Normal EB
tau2      = max(0, (Q-(k-1))/(sum w - sum w^2 / sum w))   DL, k>=3, then max(., 0.01)
gamma_xy  = shrink(logit(p_xy) - (logit p_champ + beta_x + beta_y), v_xy + var_x + var_y, 0.01)
L         = logit(p_champ) + 1.5*tanh((sum beta + sum 0.70^k gamma)/1.5)
E[max]    = sum_j s_(j) * C(j-1,2)/C(P,3);   P(improve) = 1 - C(m,3)/C(P,3)
```

Why these: shrinkage strength is estimated per sample instead of hand-tuned pseudo-counts
(`m_1 = 25`, `m_pair = 20`, `0.35` gate, `0.75`/`0.85` weights, `lambda_look` are gone);
the reroll decision is the expected best of a fresh 3-card draw, not a comparison against
the pool mean.

### `src-tauri/src/agent/mayhem_engine.rs` (+342 lines, appended, old code untouched)

- `apply_verdicts(scores, pool_scores, draw, margin) -> (Vec<&'static str>, RerollEval)`
- `decide_offered_cards(index, champ_id, offered, locked_augments, locked_items, tier, seen) -> Vec<CardDecision>`
- Helpers: `resource_counts`, `pair_counts`, `population_tau2`, `score_candidate`, `LockedResource`
- `CardDecision`: effect, win rate + 80% band, WPA, games, verdict (`pick|reroll|skip`), rank,
  reroll gain / probability.
- Constants: `REROLL_SIGNIFICANCE = 0.02` nats (~0.5% WR).
- Tests use a hand-built `MayhemSynergyIndex` (no DB) — 5 tests.

Note: the file already defines `logit`/`sigmoid` with different clamping, so the new code
imports `logit as math_logit`, `sigmoid as math_sigmoid`. Do not remove the aliases.

### Model / wiring

- `models/history.rs`: `OverlayAugmentCard` gained `Default` plus optional fields
  `score, decision, rank, win_rate_low, win_rate_high, reroll_gain, reroll_probability`
  (all `#[serde(default)]`, old JSON still parses — pinned by a serde test).
- `services/live_overlay.rs`:
  - `apply_card_decisions()` runs at the end of `scan_in_game_augment_cards`: builds/loads
    the synergy index, fetches live inventory, calls `decide_offered_cards`, writes the
    decision fields onto each card. Both F8 (live_game loop) and the Tauri command path get it.
  - `set_overlay_vitals(level, gold)` + statics for the HUD; called from `live_game.rs`
    on every in-game poll.
  - `refresh_native_overlay()` builds `rows` via the presenter and `hud_text`.
- `commands/build_engine.rs`: `ensure_index_ready` is now `pub(crate)`.

### New: `src-tauri/src/services/overlay_presenter.rs`

Pure formatting layer (6 tests): `build_rows(cards, max_name_chars)` returns
`OverlayCardRow { slot, accent, header, name, stats, footer }`, sorted by **screen** order
(LEFT/MID/RIGHT), verdict word + colour, `compact_games`. `build_hud_text(champion, level,
gold, has_cards)`.

Deliberate taste decisions: never redraw the game's cards; ≤3 numbers per row; verdict is a
word not just a colour; rows live at `x = gw-344 … gw-24`, outside the OCR crop
(x 15–85%, y 18–72%) so the overlay cannot OCR its own pixels.

### `services/native_overlay.rs` (renderer)

- `OverlayRenderPayload` gained `rows: Vec<OverlayCardRow>` and `hud_text: String` (serde default).
- Removed the block that redrew three card boxes over the game's own cards; replaced with a
  right-edge stack (320×84, gap 8, vertically centred, accent bar, 4 text lines).
- HUD line now comes from `hud_text` (champion • LV • gold • F8), falling back to
  `ARAM MAYHEM • F8 SCAN`.
- `SetProcessDPIAware()` called once at overlay-process start (module is
  `Win32::UI::WindowsAndMessaging`, not `HiDpi` — windows-sys 0.59 placement).
- Added `render_overlay_png(payload, w, h, path)` (Windows) used by the preview bin.

### New dev tool: `src-tauri/src/bin/overlay_preview.rs` (`[[bin]] overlay-preview`)

Renders a demo payload through the real draw path to `src-tauri/overlay-preview.png`
(gitignored). Run: `cd src-tauri && cargo run --bin overlay-preview`.
Not spawned by the app; it is not the overlay.

### Frontend

- Deleted `src/lib/components/overlay/LiveMayhemOverlay.svelte` (572 lines). It was
  unreachable: only mounted when `?window=overlay`, and no such window is ever created.
- `src/main.ts` now always mounts `App`.
- `src/lib/types/champ_select.ts`: optional decision fields added to `OverlayAugmentCard`.

### Docs

`@llm/docs/mayhem/RULES.llm.txt` has a short "Recommender model (v3, overlay)" section.

---

## 3. How to run

```bash
bun run build:overlay      # REQUIRED: builds src-tauri/target/debug/lcu-overlay.exe
bun tauri dev              # app spawns lcu-overlay automatically (lib.rs setup)
```

- `tauri dev` builds only the `lcu-companion` bin. Without `build:overlay`, the app logs a
  spawn failure and no overlay appears.
- The overlay is a separate process: after changing overlay Rust code, rebuild and restart
  the app (`ensure_overlay_process_started` reuses a running child).
- Overlay is invisible until League is in the foreground; calibration happens then.
- First F8 builds the synergy index from SQLite (hundreds of ms to seconds).

## 4. Verification performed

| Check | Result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | **64 passed, 0 failed** |
| `… --lib mayhem_math` | 20 passed |
| `… --lib overlay_presenter` | 6 passed |
| `… --lib mayhem_engine` | 5 passed |
| `… --lib models::history` | 1 passed |
| `cargo run --bin overlay-preview` | wrote PNG; visually checked rows + HUD (HUD clipping fixed by shortening copy) |
| `bun run check` | **8 errors, all pre-existing** in `MayhemAugmentAdvisorCard.svelte` / `MayhemChampSelectView.svelte` (untouched by this work) |

Not done: live in-game smoke test (F8 with real cards), 125% DPI visual check, performance
measurement of `decide_offered_cards`.

---

## 5. Known issues / open threads

1. **Pre-existing svelte-check errors (from the "BAD REWORK FROM GEMINI" commit).**
   Five are unused locals; one is real: `MayhemChampSelectView.svelte:90` uses
   `currentSelectedChampion` before its declaration and reads `.win_rate`, which does not
   exist on `ChampionSummary`. That view can throw at runtime. Fix before shipping the
   in-app page.
2. **Item strip depends on the main window.** `build_recommended_item_sprites` is only
   triggered by `api_emit_overlay_state` from `MayhemChampSelectView`. The overlay alone
   (app minimised / on another tab) shows cards but no items. Follow-up: compute the
   recommendation in `live_game.rs` (every ~10 s, `spawn_blocking`) and push sprites from Rust.
3. **`layout not calibrated yet` warn** while out of game: benign, one per render push.
   Optional cleanup: early-return in `refresh_native_overlay` until `LAST_CALIBRATED_RECT`
   is set (needs a small `is_overlay_calibrated()` accessor in `native_overlay.rs`), or
   downgrade the log to debug.
4. **Reroll is advisory.** The live client doesn't expose whether the reroll was already
   used this round; the row says "REROLL" meaning "the pool is better next draw".
5. **`tau2` floor 0.01** is the only remaining tuned constant besides `L_MAX`/`SYNERGY_DECAY`.
   Sparse augments therefore show small effects. Tune only with evidence.
6. **Old engine path still exists.** `predict_build_winrate`, `compute_candidate_score`,
   `recommended_augments_by_tier` in `mayhem_engine.rs` still use the old WPA-space math and
   are used by the in-app panels. The overlay path is the new one. Unifying them is a
   separate task; do not half-migrate.
7. **Dual snake_case/camelCase DTO mirrors** remain everywhere. Untouched on purpose.

## 6. Where to look

- `src-tauri/src/mayhem_math/` — equations + tests (start here)
- `src-tauri/src/services/native_overlay.rs` — draw path; `render_to_buffer` around the
  "Augment verdict rows" block; preview renderer at the bottom
- `src-tauri/src/services/live_overlay.rs` — `apply_card_decisions`, `refresh_native_overlay`
- `src-tauri/src/services/overlay_presenter.rs` — row/HUD copy and colours
- `src-tauri/src/agent/mayhem_engine.rs` — `decide_offered_cards` (end of file)

## 7. Suggested next steps

1. Fix the champ-select TDZ bug (issue 1) — smallest, highest user impact.
2. Live game test: enter ARAM Mayhem, F8 at level 3, compare LEFT/MID/RIGHT labels with the
   real screen, confirm the PICK row matches intuition; log `decide_offered_cards` duration.
3. Make the item strip self-sufficient (issue 2).
4. Then migrate the in-app panel to the same `mayhem_math` core, deleting the old
   WPA-space scoring.
