# Rune Tree Component Plan (SSR + HTMX)

This document records the architecture and implementation for the Rune Tree UI component.

## Goal

Render a **real rune tree** on the right side of the Runes page using:
- live LCU current rune page data
- CDragon rune style/perk metadata and icons
- Server-side rendering (SSR) and HTMX partial swaps (no custom client JS)

## Implementation Details

1. **Backend Service** ([`api/services/rune_tree.py`](api/services/rune_tree.py)):
   - Fetches and caches CDragon perks (`perks.json`) and styles (`perkstyles.json`).
   - Resolves active rune selections for:
     - **Primary Tree**: Keystone + 3 regular rows (4 tiers).
     - **Secondary Tree**: 3 regular rows with unselected tiers filtered out (2 active tiers).
     - **Stat Mods**: 3 tiers (Offense, Flex, Defense) evaluated by exact row index so shared IDs (e.g. Adaptive Force, Scaling Health) only highlight in their respective tier.

2. **Server-Side Renderers** ([`api/web/templates.py`](api/web/templates.py)):
   - `render_rune_tree(tree, error)`: Renders [`templates/components/rune_tree.html`](templates/components/rune_tree.html).
   - Layout:
     - Left column: Primary Tree.
     - Right column: Secondary Tree (top) + Stat Mods (bottom).
   - Selected runes styled with `ring-2 ring-blue-500 border-blue-400 bg-gray-700/80`.
   - Compact Stat Mods without text titles or row borders.

3. **HTMX Integration** ([`templates/components/runes.html`](templates/components/runes.html)):
   - Triggered on load:
     ```html
     <div
       id="current-rune-tree-container"
       class="w-full max-w-5xl animate-pulse rounded-xl border border-gray-800 bg-gray-900 p-5"
       hx-get="/ui/rune-tree/current"
       hx-trigger="load"
       hx-swap="outerHTML"
     >
       ...
     </div>
     ```
   - Refresh button in the component performs an in-place swap:
     `hx-get="/ui/rune-tree/current" hx-target="#current-rune-tree-container" hx-swap="outerHTML"`

4. **Routes** ([`api/main.py`](api/main.py)):
   - `GET /ui/rune-tree/current` -> Returns server-rendered HTML partial.
   - `GET /api/rune-tree/current` -> Returns JSON data contract for programmatic use.
