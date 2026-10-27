# Walkthrough: ARAM Mayhem Situational Items Tooltips & Popularity Sorting
Date: 2026-10-27
Status: Complete. `cargo check`, `cargo test`, and `svelte-check` pass (0 errors).

## 1. Context & Motivation
In ARAM Mayhem Champion Selection -> Recommended Items, situational items previously rendered as bare icon images with only native browser `title` attributes. They lacked rich tooltips (`GameEntityPopover`) and were sorted with a composite win rate and log-games formula instead of popularity (game count).

## 2. Changes
### Backend (`src-tauri`)
- `src-tauri/src/services/history.rs`:
  - Updated `situational_items` sorting in `mine_build_archetypes` to sort descending by number of games (`b.1.cmp(&a.1)`), breaking ties by win rate and pick rate.
  - Fixed `sorted_augs` iterator type mismatch in archetype mining.
  - Added unit test `test_situational_items_sorted_by_popularity`.

### Frontend (`src`)
- `src/lib/components/common/GameEntityPopover.svelte`:
  - Added optional `games` prop to display game count alongside price and item class in the popover header.
- `src/lib/components/history/MayhemChampSelectView.svelte`:
  - Added reactive `sortedSituationalItems` derived state to enforce popularity sorting (n of games descending).
  - Wrapped each situational item with `id="sit-item-${sit.id}-${idx}"` and attached `<GameEntityPopover>`.
  - Fixed `formatPct(aug.win_rate ?? 0)` type check.

## 3. Validation
- `cargo check --manifest-path src-tauri\Cargo.toml` -> Pass (0 errors).
- `cargo test --manifest-path src-tauri\Cargo.toml -- test_situational_items_sorted_by_popularity` -> Pass (1 test passed).
- `npx svelte-check --tsconfig ./tsconfig.json` -> Pass (0 errors).
