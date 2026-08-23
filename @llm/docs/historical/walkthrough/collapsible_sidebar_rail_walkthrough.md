# Walkthrough: Collapsible Sidebar Rail with Svelte Motion & Rune Builder Refinements

Implemented synchronized layout animations, 130px collapsed rail by default, symmetrical horizontal exponential easing, eliminated FLIP scale distortion, locked keystone options to a single row, enforced unique row selection rules in the secondary rune tree, non-interactive view mode styling, persistent selected rune page in SQLite, uniform 2px borders with mode-specific colors, and generated native executable app icons matching the titlebar logo.

---

## Key Changes

### 1. Native Executable App Icons ([`src-tauri/icons/`](lcu-companion/src-tauri/icons/))
- Ran Tauri icon generation from the application logo ([`src/assets/logo.png`](lcu-companion/src/assets/logo.png)), replacing default boilerplate template assets.
- Generated full multi-resolution icon sets:
  - Windows Executable Icon (`icon.ico` with 16x16, 32x32, 48x48, 64x64, 128x128, 256x256 layers)
  - macOS App Bundle Icon (`icon.icns`)
  - Linux/PNG Icons (`32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png`)
  - Windows Store / Appx Assets (`StoreLogo.png`, `Square44x44Logo.png`, `Square150x150Logo.png`, `Square310x310Logo.png`, etc.)

### 2. Uniform 2px Borders with Mode-Specific Styling ([`RuneCard.svelte`](lcu-companion/src/lib/components/runes/RuneCard.svelte), [`KeystoneCard.svelte`](lcu-companion/src/lib/components/runes/KeystoneCard.svelte), & [`StatShardsCards.svelte`](lcu-companion/src/lib/components/runes/StatShardsCards.svelte))
- All perk cards, keystones, and stat shards maintain a continuous `border-2` (2px) thickness across all states to guarantee **zero row height changes**:
  - **In Editing Mode**:
    - **Selected**: `border-2 ${theme.borderPrimary}` (active theme accent).
    - **Unselected**: `border-2 border-gray-700` (matching the hover border color).
  - **In View Mode**:
    - **Selected**: `border-2 ${theme.borderPrimary}`.
    - **Unselected**: `border-2 border-gray-800/60` (the standard subtle gray border).

### 3. Persistent Selected Rune Page across App Restarts ([`rune_store.rs`](lcu-companion/src-tauri/src/services/rune_store.rs), [`runes.rs`](lcu-companion/src-tauri/src/commands/runes.rs), & [`App.svelte`](lcu-companion/src/App.svelte))
- Added `get_selected_rune_key` and `set_selected_rune_key` to `RuneStore` persisting `selected_rune_key` into the SQLite `app_settings` table.
- Exposed Tauri IPC commands `api_get_selected_rune_key` and `api_set_selected_rune_key` and created typed API wrappers in `src/lib/api/runes.ts`.
- In `App.svelte`:
  - When the user selects, saves, or imports a rune page, the rune key (e.g. `saved:1` or `client:12345`) is automatically persisted to SQLite.
  - On application startup (`onMount`), the persisted rune key is retrieved and opened immediately if valid, falling back to active client or first saved page only if unavailable.

### 4. Non-Interactive View Mode ([`KeystoneCard.svelte`](lcu-companion/src/lib/components/runes/KeystoneCard.svelte), [`RuneCard.svelte`](lcu-companion/src/lib/components/runes/RuneCard.svelte), [`StatShardsCards.svelte`](lcu-companion/src/lib/components/runes/StatShardsCards.svelte), [`PrimaryPathCard.svelte`](lcu-companion/src/lib/components/runes/PrimaryPathCard.svelte), & [`SecondaryPathCard.svelte`](lcu-companion/src/lib/components/runes/SecondaryPathCard.svelte))
- Propagated `isEditing` prop to all perk, keystone, shard, and style path selector cards.
- When `isEditing` is **false** (View Mode):
  - Buttons are disabled (`disabled={!isEditing}`).
  - Cursors are set to `cursor-default` (removing `cursor-pointer`).
  - Hover highlights, background changes, and border glow on hover are disabled.
  - Non-selected perks render with a subtle, non-hoverable `opacity-35 select-none cursor-default` state.

### 5. Secondary Tree Row Constraint ([`RuneCanvas.svelte`](lcu-companion/src/lib/components/runes/RuneCanvas.svelte), [`SecondaryPathCard.svelte`](lcu-companion/src/lib/components/runes/SecondaryPathCard.svelte), & [`SecondaryTreeCards.svelte`](lcu-companion/src/lib/components/runes/SecondaryTreeCards.svelte))
- Updated `handleSelectSecondaryPerk(perkId: number, rowIndex: number)` to inspect which tier rows the existing secondary perks (`selectedPerkIds[4]` and `selectedPerkIds[5]`) occupy:
  - If a newly clicked perk belongs to a row that already has a perk selected, it **replaces the perk in that row**.
  - If the clicked perk is from a 3rd unoccupied row, it replaces the older selected perk via FIFO.
  - Guarantees that users can never select two perks from the same row in the secondary tree.

---

## Verification Results

- Generated native executable icons (`icon.ico`, `icon.png`, `icon.icns`, and Windows Appx sizes) from [`src/assets/logo.png`](lcu-companion/src/assets/logo.png).
- Built executable and bundle will display the exact same icon as the custom titlebar.
