# Handoff — ARAM Mayhem overlay (v5)

Date: 2026-11-06
Branch: `mayhem-overlay-v3` — **working tree is dirty; none of this session's work is
committed.** Last commit on the branch: `304403b docs: handoff for the mayhem overlay v3
rework`.

---

## 1. State in one paragraph

Card identification is art-fingerprint based (no whole-card OCR), the augment panel is
detected automatically and scanned without pressing F8, and a picked augment is game state
that changes the recommended build and every later recommendation. Augments are keyed by
their **ARAM pool id**, which fixed the universal "0 games" bug (Jeweled Gauntlet was
resolving to its Arena id). Cards whose art Riot ships as a shared placeholder icon are
resolved from card **text** (title, then body) and marked `unconfirmed` instead of being
misnamed. `cargo test --lib` = **81 passed, 0 failed**; `cargo build` clean. Everything was
verified against the two reference screenshots and the real 2.1 GB `global-aram.db` — but
**not yet in a live game**.

Scan cost, release build, warm caches: **57 ms** (1280×720) / **35 ms** (1920×1080) for a
full three-card identification. The old engine spent 81 ms OCR + 354 ms Levenshtein per
scan and blocked the game loop ~40 s on the first F8 (`build_synergy_index` 9.7 s +
`get_mayhem_augments` 28.9 s, both measured).

## 2. What changed in this session

### 2a. Card identification: `services/augment_scan.rs` (new, ~1.5k lines)

Capture the card band → find three cards from the bright bar along their top edge → 32×32
masked art fingerprint per card → dot product against one thumbnail per distinct
`AugmentLargeIconPath` → OCR only what art cannot settle.

- Pool source of truth: `kiwi.bin.json`, `Maps/ModeSpecificData/Augments/*` with
  `__type == "AugmentData"`. On the 2026-11 patch: **242 augments, 194 distinct icons**.
  Cached at `cache/match-history/aram_augment_pool_v2.json` (v2 added descriptions).
- Geometry is pitch-derived, not run-derived: `CARD_WIDTH_FROM_PITCH = 0.866`,
  `ICON_TOP_FROM_RUN = 0.12`, `ICON_SIZE_FRAC = 0.50`, `TITLE_TOP_FROM_ICON = 1.13`,
  `TITLE_HEIGHT_FROM_ICON = 0.40`. Card corners are rounded, so each card's bright run is
  inset differently; the three-centre pitch is stable.
- Reference ground truth, asserted by tests: `0-Screen76.png` = Symphony of War / Clown
  College / Mystic Punch; `Screen15.png` = Skilled Sniper / Tooth Fairy / From Downtown.
- Deleted: `services/dual_matcher.rs`, the old OCR helpers in `live_overlay.rs`, and eight
  scratch integration tests under `src-tauri/tests/`.

### 2b. Shared-art augments (`choose_entry` + body-text OCR)

26 of 242 augments are `IsAbilityAugment` and share **three generic icons**, one per rarity:
6 silver (`Siphon`, `Bolstered`, `It's Go Time`, `Stay Resolute`, `Trusty Weapon`,
`Adaptive Ward`), 14 gold, 6 prismatic. No per-augment icon exists on CDragon (all 26
`{name_id}_large.png` paths 404), so art names the class, never the augment.

- Such cards are resolved by **title**, then by **body text** — `description_rect` spans the
  card's inner width and is OCR'd at 2× because body glyphs are ~11 px at 720p — scored by
  distinctive-word overlap against the pool's tooltip.
- A read may **never** escape a shared-art group. Previously a fuzzy title fell through to a
  global scan of all 242 names, which is how `Siphon` was reported as `Spin Me Right Round`.
- If neither text signal decides, the card is `unconfirmed`: overlay shows `Name ?` and
  `unread card — F9/F10/F11 to set`.
- Two bugs found while validating this on the screenshots: the rect used the *title* width
  (clipping the first and last word of every line), and the OCR upscale only fired under
  70 px tall (body text went in unpixelated). Both fixed.

### 2c. In-game pick flow

1. **Detection.** `live_game` probes the panel every ~300 ms near a checkpoint
   (`augment_scan::probe_panel`: three equal bright runs plus a dimmed backdrop above them,
   so fights cannot trigger it). First sighting scans automatically; automatic failures are
   silent. A reroll is detected without the panel closing by comparing a per-card 4×4 luma
   grid (`signature_changed`, per-card NCC, brightness-invariant so hovering is not a
   reroll), which drops the stale scan and rescans.
2. **Recording.** F9/F10/F11 = pick LEFT/MID/RIGHT, F6 = undo, F8 = scan, F7 = clear. A
   conservative heuristic also records the pick when the panel closes with exactly one card
   that brightened ≥12% **over its own first-probe brightness** on two consecutive probes.
   Comparing cards to each other does not work: the middle card of an untouched panel is
   ~1.16× brighter than its neighbours (measured on both screenshots), so absolute
   brightness flags every middle card. Ambiguous closes leave it to the hotkeys and toast
   "Picked which card? F9 / F10 / F11".
3. **Effect.** `IN_GAME_AUGMENTS` (id + name + slot) lives for the whole game, is cleared at
   game end (two exit paths), feeds every later scan as `locked_augments`, re-prices the
   cards on screen, and triggers `recompute_in_game_build` →
   `compute_mayhem_build_recommendation` → item sprites pushed from Rust. The app UI gets
   `overlay:augments-changed` / `overlay:state-sync`.

Pinned by test `a_picked_augment_changes_the_build`: champion 222, adding Jeweled Gauntlet
moves `build_wpa` 0.00000 → −0.03174 and next items `[6655, 3089, 6653]` →
`[6655, 3508, 6653]`.

### 2d. Augment ids are ARAM ids

The catalog carries **two numeric ids per name** ("Jeweled Gauntlet" = Arena 48, ARAM 1048)
and statistics only exist for the ARAM one. Name hashing never matches statistics — that is
why common augments showed 0 games and why a picked augment changed nothing. Everything now
routes through `aram_augment_id_for` / `aram_augment_name_for` / `augment_numeric_id`, backed
by `ARAM_NAME_TO_ID` (published from the pool, loaded from disk at startup, cleared on cache
invalidation).

Measured for Lillia (876), Jeweled Gauntlet: `261 games, 50.57% WR, WPA −4.29%` against her
`54.86%` baseline over 3,620 games (global: 37,117 games, 49.05%, −0.59%).

### 2e. Loading and empty states

`OverlayAugmentCard.pending` → `reading stats…`; `games == 0` → `no sample yet` (removes the
0% → X% flash). New `unconfirmed` → `Name ?` + `unread card`.

### 2f. App-side commands (no UI yet)

`api_get_overlay_augments`, `api_add_overlay_augment(id, name, slot)`,
`api_undo_overlay_augment`, `api_clear_overlay_augments` are registered in `lib.rs`. The
Svelte side does **not** call them — the manual add/undo escape hatch exists only as commands.

---

### 2g. Three-row build panel (next items / full build / alternatives)

The overlay used to show the recommendation as one strip of unlabelled icons. It now renders
a bottom-right panel with three rows, above the game's minimap:

- **Row 1 — next items.** The engine's top three (it already produced three; the UI only
  showed icons), each with rank, icon, `WR / lift / sample` and the engine's own reason.
- **Row 2 — full build.** Purchase order including boots, six icons.
- **Row 3 — alternatives.** Up to six two-column entries with their lift, i.e. the candidates
  that scored well but lost the ranking. Items with no sample are dropped.

Ranking is unchanged: `(wpa + synergy_lift) * 3000/gold`, `wpa` from the empirical-Bayes
wpa in `mayhem_engine.rs`; the panel only presents it.

Live updates: the panel recomputes on every augment pick (existing path), on match start
(champion id resolved from `liveclientdata`), and on every inventory change — the poller
hashes the sorted item ids (`inventory_fingerprint` / `note_inventory_fingerprint`) and calls
`refresh_in_game_build`. `BUILD_SEQ` stops a slow older run from overwriting a newer panel.

Engine changes: `ItemRecommendationDto` gained `synergy_partner_id` /
`synergy_partner_is_augment` / `synergy_partner_lift` (+ camelCase mirrors) so a consumer can
name the pairing; `reason` is now human ("Pairs with X (+2.1% synergy)", "Standalone: …").
The app UI's `buildEngineRec` payload is deserialized and used directly
(`apply_build_recommendation`); id-only payloads fall back to `apply_build_ids`.

Layout: `BUILD_PANEL_W = 352`, bottom clearance `max(24, 0.235 * height)` for the minimap,
augment rows centred in the band above it. Below ~900 px tall the band cannot hold both, so
the panel drops its alternatives row first (`compact_panel`) rather than overlapping the
cards.

Verification: `cargo test --lib` = **85 passed** (adds panel presenter tests and a render
test that asserts a real icon is blitted into row 1 and that nothing is drawn outside the
panel); `overlay-preview` PNG inspected at 1920×1080. `cargo build --bin lcu-overlay` is
blocked while the app is running (exe locked) — close `lcu-companion` / `lcu-overlay`, then
`bun run build:overlay`.

---

## 3. Architecture notes that still hold

### Recommender core: `src-tauri/src/mayhem_math/`

Pure, no IO, no globals, no new crates. `prob.rs` (`logit`/`sigmoid`, `wilson_lcb`,
`logodds_variance`, `beta_binomial_mean`, `tau_squared` DL, `shrink`, `band`),
`effects.rs` (`observed_effect`, `sample_effect`, `estimate_tau2`, `pair_effect`,
`predict_build`), `reroll.rs` (`evaluate_reroll` — exact expected max of a 3-card draw).

```
psingle   = (W + 0.5)/(N + 1);   theta = logit(p) - logit(p_champ)
v         = 1/(W+0.5) + 1/(L+0.5)
beta      = tau2/(tau2+v) * theta                        Normal-Normal EB
tau2      = max(0, (Q-(k-1))/(sum w - sum w^2 / sum w)), then max(., 0.01)
gamma_xy  = shrink(logit(p_xy) - (logit p_champ + beta_x + beta_y), v_xy + var_x + var_y, 0.01)
L         = logit(p_champ) + 1.5*tanh((sum beta + sum 0.70^k gamma)/1.5)
E[max]    = sum_j s_(j) * C(j-1,2)/C(P,3);   P(improve) = 1 - C(m,3)/C(P,3)
p_glob    = (w_champ_x + 25 * p_glob_resource) / (n_champ_x + 25)   # level-2 prior is the
                                                                     # resource's global effect
score     = (wpa + sum_pair_LCB) * 3000/gold
```

`mayhem_engine.rs` holds `decide_offered_cards` / `apply_verdicts` / `CardDecision`
(`REROLL_SIGNIFICANCE = 0.02` nats). It imports `logit as math_logit` / `sigmoid as
math_sigmoid` because the file already defines its own with different clamping — **keep the
aliases**.

### Renderer and presenter

Native overlay only (`services/native_overlay.rs`, layered window + GDI). `OverlayRenderPayload`
carries `rows: Vec<OverlayCardRow>` and `hud_text`. Rows sit at `x = gw-344 … gw-24`, outside
the scan crop so the overlay can never read its own pixels. `services/overlay_presenter.rs`
is pure formatting (rows sorted LEFT/MID/RIGHT, verdict word + accent, `compact_games`,
`pick_key_hint`, `unconfirmed` rendering). Dev tool: `cargo run --bin overlay-preview`
writes `src-tauri/overlay-preview.png`.

The HTML overlay (`LiveMayhemOverlay.svelte`) is deleted; `src/main.ts` always mounts `App`.

---

## 4. How to run

```bash
bun run build:overlay      # REQUIRED: builds src-tauri/target/debug/lcu-overlay.exe
bun tauri dev              # the app spawns lcu-overlay itself
```

- `tauri dev` only builds the `lcu-companion` bin; without `build:overlay` no overlay appears.
- The overlay is a separate process — rebuild and restart the app after changing its code.
- Overlay is invisible until League is foregrounded (calibration happens then).
- First synergy-index build costs ~10 s, warmed in the background at app start. The scanner
  (pool + art) is warmed by `augment_scan::warm_up()`; art is cached under
  `%LOCALAPPDATA%/lcu-companion/cache/augment_art/`.

## 5. Verification performed

| Check | Result |
|---|---|
| `cargo test --lib` | **81 passed, 0 failed** |
| `cargo build` (dev) | clean; 13 pre-existing `non_snake_case` DTO warnings |
| `augment_scan::identifies_reference_cards_end_to_end` | all six reference cards identified (art 0.82–0.96) |
| Release timing, warm | **57 ms** @1280×720, **35 ms** @1080p |
| Description OCR on the real captures | body text reads correctly at both resolutions |
| `a_picked_augment_changes_the_build` | WPA and item list both move |
| `jeweled_gauntlet_resolves_to_the_aram_id_and_has_games` | id 1048, games > 0 |
| `bun run check` | 8 errors, **all pre-existing** in `MayhemAugmentAdvisorCard.svelte` / `MayhemChampSelectView.svelte` (untouched) |

Not done: any live-game run, 125% DPI visual check, measurement of `decide_offered_cards`.

## 6. Known issues / open threads

0. **Data starvation: fixed, but the model moved for every champion.** The engine used to
   ingest `LIMIT 100000` of 600k participant rows, leaving champion×item cells at 1–9 games,
   and it pooled those cells toward the *champion baseline* instead of the item's own global
   effect. Three changes (all verified on champions 1/103/222/517 before/after):
   full ingestion (600k rows, ~51 s once, then a 0.6 s disk cache load), level-2 pooling on
   `p_glob_resource`, and synergy scored on the pair LCB. Sample sizes now read in the
   thousands (3089 on champion 1: n=1 → n=1632), `+10.8%` noise synergies are gone, and
   items missing from the catalogue no longer win on a 1.5× cost multiplier. Card verdicts
   also shifted (they share `evaluate_resource`). Remaining noise is at the alternative row
   (`3100` with n=7 still shows +7%), which is the prior doing its job on a thin cell.
1. **Verdict understates augments with thin per-champion samples.** `build_synergy_index`
   reads only `ORDER BY rowid DESC LIMIT 100000` rows, so per-champion×augment counts are
   starved: Lillia × Jeweled Gauntlet has `games_pers = 0`, 54 global matches, verdict
   −0.13%, while the projection holds 261 champion games at −4.29%. The panel line and the
   verdict therefore disagree, and it is worst for champions with little data — exactly what
   a user notices. Options: blend the champion-scoped projection when `games_pers == 0`, or
   raise/replace the row cap. **Moves verdicts for every champion — decide before changing.**
2. **Auto-pick is unvalidated live.** `probe_panel` thresholds (`bar > 90.0 && above <
   bar * 0.75`) and the ≥12% own-baseline rule were tuned on the two screenshots plus
   synthetic frames. If the real selection animation differs, hotkeys are the fallback and
   the toast says so.
3. **Description OCR is unvalidated on a real shared-art panel.** Geometry, OCR quality and
   scoring are tested with real card art, but there is no Siphon/Cassiopeia screenshot. The
   log line `card N: art_top=… art_unique=false shorts=6 title=… chose=…` reports what it read.
4. **Pre-existing svelte-check errors** (from the "BAD REWORK FROM GEMINI" commit): five
   unused locals plus a real TDZ bug at `MayhemChampSelectView.svelte:90`
   (`currentSelectedChampion` read before declaration, `.win_rate` not on `ChampionSummary`).
   That view can throw at runtime.
5. **Item strip self-sufficiency.** Picks push sprites from Rust; the no-pick path still
   relies on the app UI's `api_emit_overlay_state`. Confirm live whether the strip shows with
   the app minimised.
6. **No UI for manual augment add/undo/clear** — commands exist, components do not call them.
7. **Reroll is advisory** — the live client does not expose whether the reroll was already
   used this round.
8. **Old WPA-space engine path still exists** (`predict_build_winrate`,
   `compute_candidate_score`, `recommended_augments_by_tier`); the overlay uses `mayhem_math`.
   Do not half-migrate.
9. **`layout not calibrated yet` warn** out of game: benign, one per render push.

## 7. Where to look

- `src-tauri/src/services/augment_scan.rs` — card detection, art fingerprints, title/body
  OCR, `choose_entry` (shared-art rules), `probe_panel`, `signature_changed`,
  `picked_from_baseline`; reference-screenshot tests at the bottom.
- `src-tauri/src/services/live_overlay.rs` — capture + `scan_in_game_augment_cards_inner`
  (retries, build recompute), `IN_GAME_AUGMENTS` / `record_pick_for_slot` /
  `undo_owned_augment` / `reset_owned_augments`, `OverlayHotkeys`, `recompute_in_game_build`.
- `src-tauri/src/services/live_game.rs` — polling loop, `PanelWatcher` (auto-scan, reroll,
  pick votes, undo), `run_scan(auto)`, `record_pick`.
- `src-tauri/src/services/history.rs` — `get_aram_augment_pool`, `extract_aram_pool`,
  `plain_augment_text`, `publish_pool_ids`, `aram_augment_id_for`,
  `resolve_canonical_augment_ids`, `get_mayhem_augments`.
- `src-tauri/src/agent/mayhem_engine.rs` — `augment_numeric_id`, `decide_offered_cards`,
  `AUGMENT_METADATA`.
- `src-tauri/src/services/overlay_presenter.rs` — row copy/colours, `pick_key_hint`,
  `unconfirmed` rendering.
- `src-tauri/src/mayhem_math/` — equations + tests (start here for model work).
- `@llm/docs/mayhem/RULES.llm.txt` — short rules doc: §"Recommender model (v3, overlay)",
  plus the scanning and pick-flow sections.

## 8. Suggested next steps

1. **Live game pass** in ARAM Mayhem: confirm auto-scan fires without F8, rows match the
   screen, a reroll rescan updates the names, F9/F10/F11 change the build, and
   `unconfirmed` never appears on ordinary (unique-art) cards. Watch the `augment_scan` log
   for `art_top` / `title` / `chose` lines.
2. **Decide the verdict-vs-projection question** (issue 1) — biggest remaining correctness
   gap and the one users notice.
3. Wire the four overlay-augment commands into the champ-select UI (issue 6); fix the TDZ
   bug (issue 4).
4. Only then unify the in-app panel onto `mayhem_math`, deleting the old WPA-space scoring.
