# Migration Outline: HTMX & Rust SSR to Svelte 5

This document outlines the architectural motivation, command transformations, and step-by-step strategy for migrating **LCU Companion** from Server-Side Rendered (SSR) HTML strings in Rust + HTMX 2.0 to a modern **Svelte 5** frontend in `src/` powered by typed JSON Tauri IPC commands.

---

## 1. Motivation & Problem Statement

### Current Architecture (Rust SSR + HTMX)
* The Rust backend generates raw HTML fragments using string templates (`r##"..."##.replace(...)`) in `src-tauri/src/render/`.
* A custom bridge in `src/main.ts` intercepts HTMX `command:*` calls and swaps HTML directly into the DOM.

### Why Migrate?
1. **Maintainability & DX**: Writing large HTML templates inside Rust raw string literals is error-prone, lacks IDE autocomplete, lacks standard HTML/CSS formatting, and requires custom string substitution helpers.
2. **Dev Feedback Loop**: Any layout or style adjustment currently requires recompiling the Rust binary (`cargo check` / `cargo build`) rather than benefiting from instant Vite Hot Module Replacement (HMR).
3. **Separation of Concerns**:
   - **Rust backend** should focus purely on what it does best: OS process discovery, League Client Update (LCU) HTTPS REST calls, WebSocket listeners, SQLite persistence, and disk asset caching.
   - **Frontend (`src/`)** should own component layout, client-side reactivity (editing runes, selections, loading states), and UI transitions.

---

## 2. Target Architecture & Guarantees

```mermaid
graph LR
    subgraph Frontend ["Svelte 5 Frontend (src/)"]
        App["App.svelte"]
        TitleBar["TitleBar.svelte"]
        Profile["ProfileCard.svelte"]
        Sidebar["RunesSidebar.svelte"]
        Tree["RuneCanvas.svelte"]
    end

    subgraph Backend ["Tauri 2.0 Desktop Core (Rust)"]
        IPC["Typed JSON Commands (commands/api_*.rs)"]
        Services["Services (LCU Client, SQLite DB, AssetCache)"]
        Models["Rust Domain Models (Serialize / Deserialize)"]
    end

    App --> TitleBar
    App --> Profile
    App --> Sidebar
    App --> Tree

    Profile -->|invoke('api_summoner_profile')| IPC
    Sidebar -->|invoke('api_runesets')| IPC
    Tree -->|invoke('api_rune_tree_current')| IPC

    IPC --> Services
    Services --> Models
    Models -->|JSON Responses| Frontend
```

### Core Migration Guarantees:
* **Identical Visual Design & DOM**: Svelte components will output the exact same HTML structure, Tailwind CSS 4 utility classes, Flowbite tokens, and Iconify icons (`icon-[heroicons--...]`).
* **Existing Models Reused**: The Rust domain models in `src-tauri/src/models/` (`SummonerProfile`, `ChallengeTierProfile`, `RuneTreePayload`, `RuneSetRecord`) already derive `serde::Serialize` and `serde::Deserialize`. No fundamental database or LCU logic needs to change.
* **Preserved Features**: Lockfile auto-discovery, live WebSocket sync (`OnJsonApiEvent`), custom title bar with window drag region, disk asset caching, and SQLite saved runes library will remain 100% operational.

---

## 3. Tauri Commands: HTML to Typed JSON Transformation

All `ui_*` commands returning raw HTML strings will be superseded by clean `api_*` commands returning strongly typed Rust structs as JSON:

| Current Command (HTML String) | New Command (JSON / Result) | Return Type (Rust Model) | Purpose |
| :--- | :--- | :--- | :--- |
| `ui_profile_card` | `api_summoner_profile` | `Result<SummonerProfile, String>` | Fetches Riot ID, level, and icon asset URL |
| `ui_profile_challenge_tier` | `api_profile_challenge_tier` | `Result<ChallengeTierProfile, String>` | Fetches rank badge / challenge tier icon |
| `ui_runesets` | `api_runesets_overview` | `Result<RunesOverviewPayload, String>` | Returns both saved SQLite runes and client pages |
| `ui_import_client_page` | `api_import_client_page` | `Result<RunesOverviewPayload, String>` | Saves client rune page into SQLite library |
| `ui_export_saved_rune_page` | `api_export_saved_rune_page` | `Result<RuneTreePayload, String>` | Pushes saved SQLite rune page to active client slot |
| `ui_delete_runeset` | `api_delete_runeset` | `Result<RunesOverviewPayload, String>` | Deletes saved rune page from SQLite |
| `ui_rune_tree_current` | `api_rune_tree_current` | `Result<RuneTreePayload, String>` | Returns active rune tree structure with selected perk IDs |
| `ui_rune_tree_client` | `api_rune_tree_client` | `Result<RuneTreePayload, String>` | Loads specific client page into center canvas |
| `ui_rune_tree_saved` | `api_rune_tree_saved` | `Result<RuneTreePayload, String>` | Loads specific saved page into center canvas |
| `ui_rune_tree_select_perk` | `api_rune_tree_select_perk` | `Result<RuneTreePayload, String>` | Updates perk selection in memory and recalculates valid tree |
| `ui_save_rune_tree` | `api_save_rune_tree` | `Result<RuneTreePayload, String>` | Persists edited rune tree to SQLite |

---

## 4. Planned Svelte Component Structure (`src/`)

```text
src/
├── lib/
│   ├── api/                    # Typed Tauri invoke wrappers (TypeScript)
│   │   ├── client.ts           # invoke() helper with error handling
│   │   ├── profile.ts          # Profile and challenge tier fetchers
│   │   └── runes.ts            # Rune tree, runesets, and perk operations
│   ├── stores/                 # Svelte 5 runes / reactive state
│   │   ├── appState.svelte.ts  # Active selected page key, edit mode flag
│   │   └── runeTree.svelte.ts  # Current tree payload & perk selection state
│   ├── types/                  # TypeScript interfaces mirroring Rust models
│   │   ├── profile.ts
│   │   └── runes.ts
│   └── components/
│       ├── TitleBar.svelte     # Draggable title bar with logo & window controls
│       ├── profile/
│       │   └── ProfileCard.svelte
│       ├── sidebar/
│       │   ├── RunesSidebar.svelte
│       │   ├── SavedRunesList.svelte
│       │   └── ClientRunesList.svelte
│       └── runes/
│           ├── RuneCanvas.svelte
│           ├── StyleTreeSection.svelte
│           ├── KeystoneSlot.svelte
│           ├── TierSlot.svelte
│           ├── StatModsSection.svelte
│           └── PerkIcon.svelte
├── App.svelte                  # Main grid layout (TitleBar + Sidebar + Canvas)
├── main.ts                     # Svelte app bootstrap
└── styles.css                  # Tailwind CSS v4 + Iconify plugin import
```

---

## 5. Phased Migration Strategy

### Phase 1: Tooling & Svelte 5 Setup
1. Install `@sveltejs/vite-plugin-svelte` and `svelte` in `package.json`.
2. Update `vite.config.ts` to include the Svelte plugin alongside `@tailwindcss/vite`.
3. Verify Tailwind CSS v4, Iconify (`@iconify-json/heroicons`), and `@tauri-apps/api` build cleanly.

### Phase 2: Rust IPC JSON Endpoints
1. Expose clean typed `#[tauri::command]` functions returning `Result<T, String>` using existing domain models in `src-tauri/src/models/`.
2. Register the new commands in `src-tauri/src/lib.rs`.

### Phase 3: Svelte Component Implementation
1. Port static HTML skeleton loaders and layout into `App.svelte` and `TitleBar.svelte`.
2. Implement `ProfileCard.svelte` and `RunesSidebar.svelte`.
3. Implement `RuneCanvas.svelte` with interactive editing, keystone/slot selection, save, cancel, import, and export.

### Phase 4: Cleanup & Deprecation
1. Remove HTMX dependency (`htmx.org`, `tauri-plugin-htmx`) and the custom XHR bridge in `src/main.ts`.
2. Deprecate and remove `src-tauri/src/render/` (`profile.rs`, `rune_tree.rs`, `runes_sidebar.rs`, `shell.rs`).
3. Update project documentation (`CURRENT_STATE.md`, `CONTEXT.md`, `GEMINI.md`).
