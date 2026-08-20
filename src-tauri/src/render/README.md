# HTML Renderers (`src-tauri/src/render`)

This module contains pure, deterministic server-side HTML rendering functions for HTMX components in LCU Companion.

---

## Render Modules

### 1. `profile.rs`
- `render_profile_card`: Generates HTML for the summoner card containing profile avatar, Riot ID, summoner level, and challenge tier icon placeholder.
- `render_profile_challenge_tier_icon`: Generates the challenge tier icon badge.

### 2. `runes_sidebar.rs`
- `render_runes_sidebar`: Generates the full dual-section sidebar partial containing:
  1. **League Client Pages**: Clickable cards with active pulse indicator and **Import** action button.
  2. **Companion Saved Runes**: Clickable SQLite-saved cards with **Delete** action button.
  - Supports `selected_key` highlighting (glowing border, gradient, and left indicator line).

### 3. `rune_tree.rs`
- `render_rune_tree`: Generates the complete rune tree canvas partial (`#current-rune-tree-container`).
- Formats primary keystone buttons (`h-20 w-20`), regular perk tiers (`h-11 w-11`), secondary sub-style tiers, and 3 tiers of stat mod icons (`h-8 w-8`).

### 4. `shell.rs`
- `render_shell`: Generates the top navigation header and tab views for Runes and Tools.
