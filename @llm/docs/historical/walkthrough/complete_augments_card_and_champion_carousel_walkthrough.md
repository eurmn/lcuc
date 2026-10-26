# Complete Augments Card, Champion Selection Carousel & Slot-Adjusted Pick-Rate Walkthrough

## Summary of Changes

### 1. Dedicated Complete Augments Card (`MayhemAugmentsCard.svelte`)
- **Separation of Concerns**:
  - Removed the inline top augments list from the exportable champion build card because augments cannot be exported to client item sets and are in-game situational choices.
  - Positioned `MayhemAugmentsCard.svelte` alongside the Main Champion Card in a two-column responsive grid (`xl:col-span-7` for build and `xl:col-span-5` for augments).
- **Interactive Column Sorting**:
  - Table headers are now interactive button columns with ascending/descending chevron indicators:
    - **Augment**: Sort alphabetically by name.
    - **Tier**: Sort by rarity rank (Prismatic > Gold > Silver).
    - **Sample / Pick %**: Sort by total games sampled and slot-adjusted pick rate.
    - **Win Rate**: Sort by Bayesian credibility-smoothed win rate (prevents 1-game 100% flukes from overtaking high-sample augments).
    - **WPA**: Sort by Bayesian credibility-smoothed WPA (champion synergy).
  - Synchronized with the toolbar sort dropdown and direction toggle button.
- **Comprehensive Filtering & Navigation**:
  - **Tier Filter**: Quick pills for All Tiers, Prismatic (`#f2c2ff`), Gold (`#e3c49f`), and Silver (`#a3b9bd`) conforming to the official Mayhem augment color palette.
  - **Role & Class Categories**: Filter by Physical / AD, Magic / AP, Tank / Defense, and Utility / Speed.
  - **Scope Filter**: Toggle between "All Catalog" and "Played on Champion Only".
  - **Hide Low Pick-Rate Support**: Respects the global/champ-select low pick-rate toggle, filtering out noisy <0.5% picks while preserving valid sample augments.
  - **Live Search**: Debounced search by augment name or keywords in effect tooltips with instant clear button.
  - **Fast Server-Side Pagination**: Page controls (First, Prev, Next, Last) with configurable page sizes (10, 15, 25).
- **Rich Augment Inspection**:
  - Hovering over any augment row triggers a rich `GameEntityPopover` displaying the parsed CommunityDragon description, formula evaluation, dynamic stats, and official tier glow.
  - Displays champion-specific win rate, WPA, and slot-adjusted pick rate when available, alongside global ARAM benchmarks when champion sample size is limited.

### 2. Slot & Sample-Size Adjusted Pick-Rate Model (`src-tauri/src/agent/mod.rs` & `history.rs`)
- **Resource In-Game Slot Accounting**:
  - **Augments**: Given at levels 3, 7, 11, and 15 (4 slots per player-game). Because an augment cannot be repeated, picking it once locks it for the entire match.
  - **Items**: 5 regular item slots + 1 exclusive boot slot.
  - **Champions**: 1 champion slot per player-game.
  - **Combos**: Guarded by minimum sample co-occurrence games.
- **Slot-Adjusted Pick Rate Formula**:
  - Across $N$ matches, total slot opportunities equal $N \times S$, where $S$ is slot capacity (4 for augments, 5 for non-boot items, 1 for boots, 1 for champions).
  - Prior slot share $p_0$ is defined by catalog domain size ($1/250$ for augments, $1/150$ for items, $1/7$ for boots, $1/168$ for champions).
  - Bayesian shrinkage with pseudo-slot weight $M_{\text{slots}} = 20.0$ prevents small samples (1 to 4 games) from yielding artificial 100% or 50% pick rates:
    $$\text{slot\_pick\_rate} = \frac{g + M_{\text{slots}} \cdot p_0}{N \cdot S + M_{\text{slots}}}$$
    $$\text{pick\_rate} = (S \cdot \text{slot\_pick\_rate}).\text{clamp}(0.0, 1.0)$$
- **Credibility Shrinkage for Win Rate and WPA**:
  - Solves the small-sample bias where 1–4 game flukes (e.g. 1 win / 1 game = 100% WR, +50% raw WPA) previously outranked reliable staples (such as 140 games with +6% WPA, or 64 games with +11% WPA).
  - Prior weight $M = 30.0$ incorporating global augment performance $WR_{\text{prior}} = WR_C + \text{global\_wpa}$:
    $$\widehat{WR} = \frac{\text{wins} + M \cdot WR_{\text{prior}}}{\text{games} + M}$$
    $$\widehat{WPA} = \widehat{WR} - WR_C$$
  - Result: 64 games (+11% WPA) and 140 games (+6% WPA) reliably rank at the top, while 1–4 game flukes receive appropriate shrinkage (+1.6% to +5.9% Bayesian WPA).

### 3. Champion Carousel on Top of Build & Champion Selection View
- **Horizontal Bar Scrolling Carousel**:
  - Integrated `ChampionCarousel.svelte` directly on top of the Main Champion Card in `MayhemChampSelectView.svelte`.
  - Nested in an inner padded tray (`rounded-xl border border-gray-800/80 bg-gray-950/40 p-2`) with internal carousel padding (`py-1.5 px-2`), preventing icon rings and hover glow scaling from being clipped.
  - Filtered to ARAM Mayhem champions with quick search input and instant clear.
  - Highlights the currently active/inspected champion with a purple ring and glow effect.
  - Seamless champion switching: Clicking any champion in the carousel immediately updates the build inspection card and re-calculates synergy rankings in the augment card.

### 4. Integrated Minimalistic Header with Synced X-Axis
- **Seamless Layout Integration**:
  - Removed the external banner container, borders, and decorative icon.
  - Positioned the minimalistic status header directly at the very beginning of the main content container (`flex-1 p-6 space-y-6`), perfectly synchronizing its horizontal margins and padding with the cards below.
  - Background matches the main application canvas (`bg-transparent` / `bg-gray-950`).
- **Clear Status Indicators**:
  - **LCU Client Status**: Real-time connected/offline indicator listening to `lcu:connected` and `lcu:disconnected` with initial profile check.
  - **Champ Select Status**: Clear state badge showing `Active (X/5)` during champ select, `Idle` when client is connected but outside champ select, or `Offline` when disconnected.
  - **Hide Low Pick-rate Control**: Positioned before the status badges, with window storage synchronization across tabs.

### 5. Backend & API Integration
- Backend endpoint `api_mayhem_augments` in [`src-tauri/src/commands/champ_select.rs`](src-tauri/src/commands/champ_select.rs) queries the full augment catalog with Bayesian WPA scoring against the selected champion.
- Strong typing and frontend wrappers in [`src/lib/types/champ_select.ts`](src/lib/types/champ_select.ts) and [`src/lib/api/champ_select.ts`](src/lib/api/champ_select.ts).
- Registered `api_mayhem_augments` in [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs) commands dispatch table.

### 6. Item Exclusivity Groups & Conflict Prevention
- **Incompatible Items Detection**:
  - Identified League of Legends unique passive exclusivity groups from CommunityDragon `lol.stringtable.json`:
    - **Hydra / Cleave**: Ravenous Hydra (3074), Titanic Hydra (3748), Profane Hydra (6698), Stridebreaker (6631), Tiamat (3077).
    - **Lifeline**: Sterak's Gage (3053), Maw of Malmortius (3156), Immortal Shieldbow (6673), Hexdrinker (3155).
    - **Last Whisper**: Lord Dominik's Regards (3036), Serylda's Grudge (6694), Mortal Reminder (3033), Terminus (3302).
    - **Crit Modifiers**: Infinity Edge (3031), Navori Flickerblade (6675).
    - **Spellblade**: Trinity Force (3078), Lich Bane (3100), Iceborn Gauntlet (6662), Essence Reaver (3508), Bloodsong (3869).
    - **Tear / Mana Charge**: Manamune (3004), Archangel's Staff (3003), Winter's Approach (3119).
    - **Immolate**: Sunfire Aegis (3068), Hollow Radiance (6664), Bami's Cinder (3751).
    - **Boots**: Maximum of 1 tier 2 boots per build.
- **Dynamic Build Fallback**:
  - During the 6-core-item assembly loop, when an item belongs to an exclusivity group that is already claimed by an earlier chosen item, the algorithm skips the duplicate and selects the next highest-scoring candidate item.

### 7. Adaptive Candidate Thresholds & Strategy Alignment
- **Adaptive Sample Scaling for Niche Champions**:
  - Replaced rigid hard clamps with adaptive sample thresholds:
    - If `total_games <= 15`: `min_item_games = 1`, `min_aug_games = 1`.
    - If `total_games <= 50`: `min_item_games = 2`, `min_aug_games = 2`.
    - Otherwise: clamped between 3 and 20.
  - Ensures alternative items are available even for rare champions with low total sample sizes.
- **Strategy-Aligned Candidates**:
  - The alternative items list directly inherits the active strategy's scoring metric (Win Rate, Damage Dealt, DPM, True Damage, Damage Taken, Damage Mitigated, Healing, or Turret Damage with $M=10.0$ Bayesian shrinkage), presenting the next best situational items.
- **Games Sampled Labels**:
  - Both the active strategy button and every dropdown option display the exact games sampled (`{strat.sample_games.toLocaleString()} games`), providing full transparency into statistical confidence.
- **Item Slot Pick-Rate**:
  - Starter items, 6 core items, and alternative items display their slot-adjusted Bayesian pick rate alongside their WPA and price.
