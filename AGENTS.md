# LCU Companion Agent Guide

## Mission and architecture

LCU Companion is a Tauri 2 desktop application for League of Legends. Svelte 5
renders the UI, Tauri commands act as adapters, and background Tokio tasks
(in-process agent) own long-running LCU synchronization, collection, persistence, and
expensive statistics calculations.

```text
Svelte presentation -> Tauri IPC facade -> In-process agent dispatcher
                                         -> Rust domain logic
                                         -> SQLite and LCU
```

The migration is incremental. Direct Tauri services remain active for legacy
features (e.g. live rune tree editing, profile card), while all collection,
match history, and statistics aggregation run in the background agent tasks.

## Repository map

```text
src/                         Svelte 5 + TypeScript frontend
src/lib/api/                 Typed Tauri/agent request wrappers (agent.ts, profile.ts, runes.ts)
src/lib/components/          Presentation components (common/, runes/, history/, sidebar/)
src/lib/components/common/   Shared components (AugmentIcon.svelte, GameEntityPopover.svelte)
src/lib/utils/gameDescription.ts
                             Game description & math expression parser with tracker cleanup
src-tauri/src/agent/         Standalone agent process, crawler, projection store, and collector
src-tauri/src/commands/      Tauri command facades
src-tauri/src/services/      Existing LCU, cache, profile, and rune services
src-tauri/src/data/          Static bundled game data (augment_tooltips.json)
src-tauri/src/agent_supervisor.rs
                             Agent lifecycle and authenticated loopback IPC
@llm/PLAN.md                 Architecture migration plan
@llm/docs/context/           Current state, handoffs, and backend architecture
@llm/docs/db/                SQLite ownership, schema, and projection documentation
```

## Agent Execution & In-Process Lifecycle

The agent runs in-process as background Tokio tasks (`collector_loop` and `websocket_loop`)
managed by `AgentSupervisor` inside `lcu-companion`. Running in-process eliminates
dual-binary compilation race conditions, TCP socket overhead, and port reservation on
Windows.

A standalone binary target remains available at `src-tauri/src/bin/lcu-companion-agent.rs`
if external standalone execution is ever needed:

```bash
cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent
```

## Database ownership & storage

The agent owns two databases under `%LOCALAPPDATA%\lcu-companion\`:

```text
%LOCALAPPDATA%\lcu-companion\agent.db        (Personal matches, facts, projections)
%LOCALAPPDATA%\lcu-companion\global-aram.db   (Discovered accounts, global ARAM games, facts, projections)
```

Do not add another writer. Personal facts belong in `agent.db`; global ARAM
Mayhem games, participants, crawler accounts, and configuration belong in `global-aram.db`.

Database access rules:
- Shared connection helper with 5-second busy timeout and WAL mode.
- Short-lived, operation-scoped connections.
- Never hold DB transactions across network calls or JSON parsing.
- Synchronous rusqlite work from async tasks must run inside bounded `spawn_blocking` blocks.

## Collection & Global ARAM Crawler Flow

1. **Personal Collector**: Discovers LCU credentials, resolves current summoner PUUID,
   reconciles recent summaries (20 games), queues missing details, fetches details with
   bounded concurrency, writes facts, and triggers asynchronous projection invalidation.
2. **Global ARAM Crawler**:
   - Seeded from personal matches or current player.
   - Multi-threaded: Spawns configurable concurrent worker agents (`worker_count` in DB config, clamped 1..=16, default 3).
   - Async request pacing: Strictly enforces 0.5–1.0s delay between all LCU HTTP requests per worker.
   - Account deduplication: Atomic transaction claims accounts (`status = 'fetching'`), preventing multiple workers from crawling the same account.
   - Match deduplication: Checks game IDs against `aram_games` with `uncollected_global_games` and atomically locks in-flight games via `in_flight_games: Arc<tokio::sync::Mutex<HashSet<i64>>>` prior to fetching details.
   - Discovered accounts persisted in `discovered_accounts` (`status`: `queued`, `fetching`, `complete`, `failed`).
   - Breadth-first crawl refilled from DB (`WHERE status = 'queued'`).
   - Resilient: Continues on single match history errors; marks accounts as `failed` rather than crashing.

## Statistics & Compare Mode

All statistics calculations belong strictly in Rust:
- Computes WR, WPA (benchmark-relative), KDA ratio, damage, gold, duration, and pick rate.
- Compare mode aggregates personal vs global facts:
  - Personal WR & WPA side-by-side with Global WR & WPA.
  - Pick rate denominators use total matches for the relevant scope.
  - Excludes Poro-Snax IDs `2050` and `2052`.
  - Persists and reads precomputed projections (`statistics_projections`).
- Frontend receives render-ready DTOs and handles presentation/pagination only.

## UI Progress Banner

A top horizontal progress banner in `App.svelte` displays active background
operations (crawler progression, projection builds, database updates) polled
from `agent_operations` and `global_aram_config`.

## Development rules

- Windows paths use backslashes (`\`).
- Use `pnpm`, not `npm`.
- Use lowercase CommunityDragon asset paths.
- Keep Svelte thin; domain calculations belong in Rust.
- Preserve backend cached assets and data URIs.
- Do not run Cargo checks or tests proactively without explicit user instructions.

## Validation

```bash
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent
pnpm svelte-check
git diff --check
```

---

## Complete Project Structure

```text
lcu-companion/
├── @llm/                               # Complete project documentation and context
│   └── docs/
│       ├── context/                    # Architecture, live state, and migration notes
│       ├── db/                         # SQLite schema, queries, and rune domain models
│       ├── historical/                 # Shelved features & logs
│       ├── lcu/                        # LCU REST/WebSocket specs & OpenAPI swagger.json
│       └── ui/                         # Design system, thematic colors, and cheat sheets
├── src/                                # Svelte 5 + TypeScript Frontend Application
│   ├── lib/
│   │   ├── api/                        # Typed Tauri IPC client wrappers (agent.ts, profile.ts, runes.ts)
│   │   ├── components/                 # Reactive UI Components
│   │   │   ├── history/                # Match history & statistics components (BuildStatisticsView, ProfileStatisticsView)
│   │   │   ├── runes/                  # Center canvas rune tree components
│   │   │   ├── sidebar/                # Dual list sidebar components
│   │   │   ├── ProfileCard.svelte      # Summoner profile card & challenge badge
│   │   │   └── TitleBar.svelte         # Window title bar with drag region & controls
│   │   ├── types/                      # Frontend TypeScript interface definitions
│   │   └── utils/                      # Utilities & Theme definitions
│   ├── App.svelte                      # Root application component & progress banner
│   ├── main.ts                         # Application entrypoint mounting Svelte App
│   ├── styles.css                      # Tailwind CSS root imports and custom scrollbar
│   └── vite-env.d.ts                   # Vite environment typings
├── src-tauri/                          # Rust Backend Core
│   ├── src/
│   │   ├── agent/                      # Standalone agent process, crawler, projections, stats
│   │   ├── commands/                   # Typed Tauri IPC command endpoints
│   │   ├── models/                     # Strongly typed Rust domain models
│   │   ├── services/                   # Background services & business logic
│   │   ├── agent_supervisor.rs         # Agent process lifecycle and IPC auth
│   │   ├── lib.rs                      # Tauri setup, command registration, and entrypoint
│   │   └── main.rs                     # Windows native entry point
│   ├── build.rs                        # Tauri build script
│   ├── Cargo.toml                      # Rust dependencies & optimization profiles
│   └── tauri.conf.json                 # Tauri configuration, window dimensions, and permissions
├── package.json                        # Node dependencies (Svelte 5, Vite, Tailwind CSS, TypeScript)
└── svelte.config.js                    # Svelte compiler configuration
```

---

## 4. Subsystem Deep-Dives

### A. LCU Auto-Discovery & Real-Time Synchronization
1. **Lockfile Discovery**: [`services/lcu_client.rs`](/src-tauri/src/services/lcu_client.rs) inspects the Windows process list (`sysinfo`) for `LeagueClientUx.exe`, extracts `--app-port` and `--remoting-auth-token`, and reads the active `lockfile`.
2. **WebSocket Watcher**: [`services/lcu_websocket.rs`](/src-tauri/src/services/lcu_websocket.rs) establishes a background WSS connection to `127.0.0.1:<port>`, subscribes to `OnJsonApiEvent`, and actively triggers updates when the player switches rune pages or enters Champion Select.
3. **Companion Slot (`LCUC`)**: Exported runes are marked with the prefix `[LCUC]` and automatically synced to the active client slot.

### B. Persistent In-Disk Asset Caching
To ensure instant loading and offline capability:
- [`services/asset_cache.rs`](/src-tauri/src/services/asset_cache.rs) caches remote CDragon assets to disk under `%APPDATA%/com.eurmn.lcu-companion/cache/`.
- Assets include:
  - Rune Perk Icons (`/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/...`)
  - Summoner Profile Icons (`/latest/plugins/rcp-be-lol-game-data/global/default/v1/profile-icons/...`)
  - Ranked Borders (`/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-emblems/...`)
  - Challenge Rank Tokens (`/latest/plugins/rcp-fe-lol-static-assets/global/default/challenges-shared/icon-token-(rank).png`)
- The backend serves local cached assets directly as local file URIs or base64 streams to the Svelte components.

### C. Thematic Rune UI System
The visual styling conforms to official League of Legends style paths via [`src/lib/utils/theme.ts`](/src/lib/utils/theme.ts):
- **Precision (ID: 8000)**: Amber / Gold (`border-yellow-500`, `text-yellow-400`, amber glow)
- **Domination (ID: 8100)**: Crimson / Red (`border-red-500`, `text-red-400`, crimson glow)
- **Sorcery (ID: 8200)**: Brand Purple (`border-purple-500`, `text-purple-400`, purple glow)
- **Inspiration (ID: 8300)**: Turquoise / Cyan (`border-cyan-400`, `text-cyan-400`, cyan glow)
- **Resolve (ID: 8400)**: Emerald / Green (`border-emerald-500`, `text-emerald-400`, emerald glow)

### D. SQLite Rune Library
Custom runesets are persisted in local SQLite via [`services/rune_store.rs`](/src-tauri/src/services/rune_store.rs):
- Table `runesets`: `id`, `name`, `primary_style_id`, `sub_style_id`, `selected_perk_ids` (JSON), `created_at`, `updated_at`.
- Table `favorite_champions`: `champion_id`, `created_at`.
- Provides instant importing from the live League client into the saved database and exporting back to the client.

### E. Flowbite Component System
The application's design language and UI primitives (buttons, modals, input groups, badges, tabs, alerts) are based on the **Flowbite** component system. Full component reference documentation is available under [`@llm/docs/ui/flowbite/`](/@llm/docs/ui/flowbite/):
- Most of the application is built with raw html sourced from flowbite. But from now on the changes should be made using [`flowbite-svelte`](/@llm/docs/ui/flowbite/FLOWBITE_SVELTE.md).

### F. Heroicons Icon System
All icons across frontend Svelte components must use `@iconify-json/heroicons` (or `@iconify-json/flowbite` where specified) icon classes (e.g. `<span class="icon-[heroicons--...]"></span>` or `<span class="icon-[flowbite--...]"></span>`) rather than raw inline `<svg>` elements.

### G. ARAM Mayhem Augments & Build Statistics Subsystem
1. **Tier Identification & Official Color Palette**:
   - **Silver**: `#a3b9bd`
   - **Gold**: `#e3c49f`
   - **Prismatic**: `#f2c2ff`
   - **Prismatic Border Gradient**: `linear-gradient(180deg, #e78fff, #70008f)`
2. **Dedicated Augment Icon (`AugmentIcon.svelte`)**:
   - Preserves original image asset presentation (`object-cover`).
   - Applies the official tier borders (Silver `#a3b9bd`, Gold `#e3c49f`, Prismatic `linear-gradient(180deg, #e78fff, #70008f)`).
   - Graceful fallback with initial letter glyph when an icon asset is missing.
3. **Rich Entity Popovers (`GameEntityPopover.svelte`)**:
   - Universal hovercard popovers across items, augments, and champions.
   - Dynamic tier glow/border, gold pricing, item classifications, and rich HTML descriptions.
4. **Horizontal Inspector & "Best Matches" Synergy**:
   - Split top horizontal inspector in [`BuildStatisticsView.svelte`](/src/lib/components/history/BuildStatisticsView.svelte):
     - Selected Build Details card (title, tier/price, native in-game description, performance metrics bar).
     - Unified 3-column "Best Matches" card (Champions, Items, Augments) with `((WR))` and `((WPA))` micro-badges.
   - Overlapping card deck hand preview for top 3 synergizing champions in the main table with WPA-based sorting (`champWpa`).
5. **CommunityDragon Description & Tooltip Parser (`gameDescription.ts`)**:
   - Formula evaluators resolving `mDisplayAsPercent` ratio calculations.
   - Dynamic math expression evaluator handling custom scaling (e.g. `@Var*20@%`, `@Var*1000@`, `@Var/2@`).
   - Live match tracker scrubbing removing scoreboard metrics (`Damage Dealt:`, `Bonus Damage: @f1.0@`, `<rules>` containers) to keep tooltips clean.
   - Float32 precision sanitization preventing IEEE-754 decimal artifacts (e.g. `0.0799999982` -> `0.08`).

---

## 5. Comprehensive Documentation Index (`@llm/docs/`)

For detailed specifications and domain knowledge, consult the specialized documents in [`@llm/docs/`](/@llm/docs/):

* **Context & Live State**:
  * [`20261012_BUILD_STATISTICS_TOOLTIPS_AND_BEST_MATCHES.md`](/@llm/docs/context/conversation/20261012_BUILD_STATISTICS_TOOLTIPS_AND_BEST_MATCHES.md) — Augment tooltips, build statistics inspector, Best Matches, math parser, and tier palette handoff.
  * [`CONTEXT.md`](/@llm/docs/context/CONTEXT.md) — System overview, data flows, and module responsibilities.
  * [`CURRENT_STATE.md`](/@llm/docs/context/CURRENT_STATE.md) — Live snapshot of registered commands, services, and test status.
  * [`HTMX_TO_SVELTE.md`](/@llm/docs/context/HTMX_TO_SVELTE.md) — Svelte 5 + TypeScript migration plan and architecture reference.
  * [`MIGRATION.md`](/@llm/docs/context/MIGRATION.md) — Architecture migration history.
  * [`DOCS_LINKS.md`](/@llm/docs/context/DOCS_LINKS.md) — Manifest of all documentation files.

* **League Client Integration**:
  * [`LCU.md`](/@llm/docs/lcu/LCU.md) — LCU protocol, auth headers, and WebSocket event handling.
  * [`swagger.json`](/@llm/docs/lcu/swagger.json) — Full OpenAPI specification for LCU endpoints.
  * [`getting-started-with-the-lcu-api`](/@llm/docs/lcu/getting-started-with-the-lcu-api) — LCU REST integration fundamentals.
  * [`getting-started-with-the-lcu-websocket`](/@llm/docs/lcu/getting-started-with-the-lcu-websocket) — LCU WebSocket event subscription guide.

* **Database & Domain Models**:
  * [`DATABASE.md`](/@llm/docs/db/DATABASE.md) — SQLite schema, indexes, and queries.
  * [`RUNES.md`](/@llm/docs/db/RUNES.md) — Rune perk IDs, style trees, and serialization formats.

* **UI, Animation & Theming**:
  * [`UI.md`](/@llm/docs/ui/UI.md) — UI architecture, Svelte 5 component hierarchy, Flowbite patterns, and Svelte Motion design.
  * [`flowbite/`](/@llm/docs/ui/flowbite/) — Flowbite component guides (buttons, forms, modals, alerts, badges, tables, etc.).
  * [`motion_svelte.llm.txt`](/@llm/docs/ui/motion_svelte.llm.txt) — Svelte Motion (`@humanspeak/svelte-motion`) declarative animation guide and cheat sheet.
  * [`THEME.md`](/@llm/docs/ui/THEME.md) — Thematic color palette rules for rune paths and Flowbite button styling tokens.
  * [`RUNE_TREE_PLAN.md`](/@llm/docs/ui/RUNE_TREE_PLAN.md) — Center canvas layout plan and sizing.
  * [`tauri.llms.txt`](/@llm/docs/ui/tauri.llms.txt) — Tauri 2.0 API reference cheat sheet.

* **Historical Archive**:
  * [`walkthrough/`](/@llm/docs/historical/walkthrough/) — Permanent archival walkthroughs for completed features and plans.
  * [`TOOLS_FEATURE_ARCHIVE.md`](/@llm/docs/historical/TOOLS_FEATURE_ARCHIVE.md) — Archival notes on the mass friend removal tool and top navigation header.

---

## 6. Development & Validation Workflow

### Running Locally
```cmd
# Build the standalone agent, then start frontend and Tauri in development mode.
# Tauri's debug supervisor also invokes Cargo for the agent on each restart.
pnpm tauri dev
```

### Planning & Walkthrough Persistence
- **Walkthrough Archival**: Whenever completing work in Planning Mode, in addition to generating the session walkthrough artifact, **ALWAYS save a permanent copy of the walkthrough document inside `@llm/docs/historical/walkthrough/<feature_name>_walkthrough.md`** so historical implementation context is permanently tracked across sessions. Change the file name of the markdown file if the conversation gets too long/changes subject. If the conversation includes multiple very different changes that need their own documentation, create different files for different features. After creating a walkthrough and finishing your task, ask the user for feedback and, when approved, commit your changes.
- Verify the last 2 walkthroughs to have been created when you are in a new conversation. This will give you context over the current development environment the application is currently at.

### Command Execution Guidelines
- **No Unsolicited Checks or Tests**: **DO NOT** run check, build, or test commands (such as `cargo check`, `cargo test`, `svelte-check`, `vite build`, etc.) without the user's direct and explicit request.

## Package Manager & Tooling
- Always use `pnpm` instead of `npm` for installing dependencies, running scripts, and managing packages in this repository.
- Do not run `pnpm svelte-check` automatically or proactively without the user explicitly asking for it.
- When the user explicitly requests type-checking Svelte components and TypeScript in this project, use:
  ```bash
  pnpm svelte-check
  ```
- Do not run `cargo check` or `cargo build` proactively before finishing code changes. Only run check/test commands after all code modifications are complete.

