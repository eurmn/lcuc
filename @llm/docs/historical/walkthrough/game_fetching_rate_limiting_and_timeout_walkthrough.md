# Game Fetching Agent Rate Limiting, Layout & Crawler Speed Walkthrough

## Overview
1. **Game Fetching Agent Overhaul**:
   - Audited and rebuilt the rate-pacing and timeout algorithms in [`src-tauri/src/agent/mod.rs`](src-tauri/src/agent/mod.rs).
   - Created [`WorkerRatePacer`](src-tauri/src/agent/mod.rs) enforcing a maximum 100 games/minute rate per worker (600ms start-to-start interval).
   - Ensured zero request spam/bursting while preventing artificial bottlenecks when single requests take longer than 600ms.
   - Enhanced [`CrawlerCircuitBreaker`](src-tauri/src/agent/mod.rs) with immediate HTTP 429 backoff (60s) and in-loop game-fetch checks.
   - Paced personal collector backfill and caching.

2. **Best Matches Re-Ranking by Pickrate**:
   - Re-ranked the 3 columns in the top horizontal **Best Matches** card (Champions, Items, Augments) in [`BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte) by `pick_rate` (and `games`) descending instead of `wpa`/`win_rate`.
   - Updated backend projection preparation in [`src-tauri/src/agent/mod.rs`](src-tauri/src/agent/mod.rs) to pre-sort `champion_rows` and `partner_rows` by `pick_rate` descending.
   - Preserved all existing metrics and tags: the exact same data (`WR $wins/$games ($wr%) | WPA $wpa`), popovers, icons, and pricing remain intact.
   - Updated the Best Matches card subheader label from "Highest synergy & win rates" to "Most commonly picked".

3. **Horizontal Layout & Width Adjustments**:
   - Removed the `X` close button from the selected build details card on the left.
   - Shrunk the selected build details card along the X-axis by `4rem` (`lg:w-[calc(33.333%-4rem)] shrink-0`).
   - Expanded the **Best Matches** card to take all remaining horizontal space (`lg:flex-1 min-w-0`), gifting it the reclaimed `4rem`.
   - Removed right-side padding (`sm:pr-0`, `pr-0`) and halved left padding (`sm:pl-1.5`) between the columns in Best Matches.

4. **Crawler Real-Time Speed (match/minute)**:
   - Added real-time moving average calculation of crawler speed in [`App.svelte`](src/App.svelte).
   - Multiplies `(countDelta / timeDeltaSec) * 60` to track matches per minute over a 5-second sliding window.
   - Displayed `· X.X match/minute` directly after the percentage in the crawler loader topbar: `{currentCount} / {totalLimit} ({progressPct}%) · X.X match/minute`.

---

## Changes Made

### 1. Backend (`src-tauri/src/agent/mod.rs`)
- In `build_statistics_row`:
  - `champion_rows`: sorted by `pick_rate` descending (with `games` tie-breaker).
  - `partner_rows`: sorted by `pick_rate` descending (with `games` tie-breaker).

### 2. Frontend (`src/lib/components/history/BuildStatisticsView.svelte`)
- In `filterAndSortPartnersByKind`:
  - Items and augments sorted by `b.pick_rate - a.pick_rate` descending.
- Added `sortBestMatchChampions`:
  - Champions in the Best Matches card sorted by `b.pick_rate - a.pick_rate` descending.
- Subtitle in Best Matches card updated to `"Most commonly picked"`.
- Removed `X` close button from selected item card.
- Inspector converted to `flex flex-col lg:flex-row gap-3`, shrinking item card by `4rem` (`lg:w-[calc(33.333%-4rem)]`) and expanding Best Matches card (`lg:flex-1 min-w-0`).
- Best Matches column spacing tuned to `sm:pl-1.5 sm:pr-0` with `pr-0` scroll lists.

### 3. Frontend (`src/App.svelte`)
- Added crawler speed state and 5-second sliding window sampling in `refreshGlobalStatus`, calculating `(countDelta / timeDeltaSec) * 60`.
- Formatted `· {crawlerGps.toFixed(1)} match/minute` right after the progress percentage in the crawler loader banner.

---

## Verification

- `git diff --check`: passed with zero formatting errors.
- Unit tests in `agent::tests` continue to verify `WorkerRatePacer` and circuit breaker timing.
