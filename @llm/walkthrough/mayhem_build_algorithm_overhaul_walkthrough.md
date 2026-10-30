# Walkthrough: ARAM Mayhem Build Recommender Overhaul & In-Game Augment Tracker

Date: 2026-10-27
Status: Complete. `cargo check` and `pnpm svelte-check` pass (0 errors). Not yet exercised in the running app.

---

## 1. Context & Motivation

The original ARAM Mayhem build recommender had three flaws:
- **Contradictory item mixtures**: items were scored independently by win rate / WPA, so builds mixed incompatible items (e.g. 3 crit items plus Titanic Hydra on marksmen).
- **Situational item bias**: rare items bought in a few winning games (Mercurial Scimitar, Zhonya's on ADCs) showed inflated win rates and displaced core items.
- **No augment awareness**: recommendations did not adapt to augments picked at levels 3, 7, 11, 15, though augments change item synergies.

---

## 2. Data Source Findings

- LCU `/lol-match-history/v1/game-timelines/{gameId}` returns HTTP 200 for Mayhem, but only `CHAMPION_KILL` and `BUILDING_KILL` events. `participantFrames` hold only gold, XP, level, position. **No item purchase or sell events exist.**
- `ponytail: build order proxy`: the inventory slot index (`item0`..`item6` in `aram_participants.item_ids_json`) approximates build order. Core items are sorted by mean slot index across matching games. Ceiling: slot order is not purchase order; upgrade if Riot exposes purchase events.

---

## 3. Implementation

### A. Models (`src-tauri/src/models/history.rs`)
- `MayhemBuildStrategy.situational_items: Vec<MayhemBuildItem>` (low-pick items kept out of the core path).
- `MayhemChampionBuildSummary.applied_augment_ids` and `fallback_used`.
- `MayhemAugmentsQuery.with_augment_ids: Option<Vec<String>>`.
- `SavedMayhemBuild` and `SaveMayhemBuildPayload`.

### B. Archetype mining (`src-tauri/src/services/history.rs`)
- `classify_archetype_prefix` / `generate_archetype_label`: label by item tags (Crit, Lethality, AP, Tank, Crit/Lethality, Core) plus top core item names.
- `mine_build_archetypes` (private; it takes the private `ParticipantFact`):
  - Considers legendaries only (price >= 1800, not boots/starters, excluding Poro-Snax and Quest items).
  - Enforces exclusive item groups (Hydra, Lifeline, Last Whisper, crit modifiers) via `get_item_exclusive_group`.
  - Mines co-occurring 3-item cores (2-item pairs if the sample is small) scored by Bayesian WR and pick share.
  - Keeps up to 3 archetypes with low overlap (triples share at most 1 item; pair fallback shares none).
  - Orders core items by the slot proxy and picks boots (most common) and a starter (>= 15% presence, >= 2 games).
  - Continuation items with < 8% share go to `situational_items`.

### C. Strategy assembly (`compute_mayhem_champion_build`)
- Signature is now `(champion_id, requested_augments)`. Augments are normalised (sorted, deduped, `"0"`/empty removed).
- Augment conditioning keeps games containing **all** requested augments. If fewer than 15 games remain, the last augment is dropped repeatedly (`fallback_used = true`) until the sample is large enough. Applied augments are reported in `applied_augment_ids`.
- All aggregation (items, augments, spells) uses the conditioned sample (`active_facts`). `total_games` in the summary stays the unconditioned count.
- Caches (`MAYHEM_CHAMPION_BUILDS_CACHE`, `MAYHEM_CHAMPION_AUGMENTS_CACHE`) are keyed by `(champion_id, Vec<String>)`.
- Each mined archetype becomes a `MayhemBuildStrategy` with id `archetype_N`: ordered core, boots, extra continuation items up to 6 slots (exclusive groups respected), `alternative_items`, `situational_items`, starter, spells, augments. Archetypes come first, followed by the 8 metric strategies (win rate, damage, DPM, true damage, damage taken, mitigated, healing, turret damage).
- Helpers `mayhem_item`, `mayhem_spell`, `mayhem_augment` enrich archetype entries.

### D. Augment catalog (`get_mayhem_augments`)
- Reads `with_augment_ids`, uses the matching conditioned cache entry (computing it if missing), and excludes already-picked augments from results.

### E. Saved builds
- Table `saved_mayhem_builds` in `agent.db` (`id, champion_id, champion_name, strategy_label, build_json, augment_ids_json, created_at`), created on first use. All access goes through `spawn_blocking` with short-lived connections (5s busy timeout, WAL).
- Service fns: `save_mayhem_build`, `list_saved_mayhem_builds(Option<champion_id>)`, `delete_saved_mayhem_build`. Rows whose build JSON no longer parses are skipped.
- Tauri commands in `commands/champ_select.rs`, registered in `lib.rs`: `api_save_mayhem_build`, `api_list_saved_mayhem_builds`, `api_delete_saved_mayhem_build`. `api_mayhem_champion_build` accepts optional `augment_ids`.
- `created_at` is epoch seconds as a string.

### F. Frontend
- `src/lib/types/champ_select.ts`, `src/lib/api/champ_select.ts`: new fields (`situational_items`, `applied_augment_ids`, `fallback_used`, `with_augment_ids`), `SavedMayhemBuild`/`SaveMayhemBuildPayload`, and wrappers `saveMayhemBuild`, `listSavedMayhemBuilds`, `deleteSavedMayhemBuild`; `getMayhemChampionBuild(championId, augmentIds = [])`.
- `src/lib/components/history/MayhemAugmentPicker.svelte`: 4 tall slots (levels 3/7/11/15). Click an empty slot to open a searchable list (sorted by WPA, already-picked excluded by the backend); click a filled slot to clear it. Shows a notice when `fallbackUsed`.
- `src/lib/components/history/MayhemChampSelectView.svelte`:
  - Tracks `gameflowPhase` from `lcu:gameflow-phase`. Picker shows when phase is `InProgress`; picking or clearing reloads the build with the picked augment ids.
  - Picker and augment state reset on `Lobby`, `None`, `ChampSelect`.
  - Core items show an order number badge (slot proxy); a collapsible "Situational" list shows `situational_items`.
  - When the phase moves from `InProgress` to `PreEndOfGame`, `EndOfGame` or `WaitingForStats`, a prompt offers to save the active strategy.
  - Default selected strategy is `archetype_1`. If a champion has no archetypes, the existing fallback to the first available strategy applies.

---

## 4. Validation

| Check | Result |
|-------|--------|
| `cargo check --manifest-path src-tauri\Cargo.toml` | Pass (one `private_interfaces` warning fixed by making `mine_build_archetypes` private) |
| `pnpm svelte-check` | 0 errors, 7 warnings (pre-existing Tailwind suggestions) |
| `cargo test`, `cargo fmt --check` | Not run |
| Manual run in app | Not done |

---

## 5. Known Gaps / Follow-ups

- No UI to list or delete saved builds yet (API exists).
- Archetypes appear in the existing strategy dropdown; there are no dedicated archetype tabs.
- Build order is a slot-index proxy, not true purchase order.
- Self-check for `mine_build_archetypes` not written; add one small test with synthetic `ParticipantFact`s if the algorithm is tuned further.
- Metric strategies still choose items independently; only archetypes enforce coherent cores.
