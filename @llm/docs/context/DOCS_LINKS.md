# Documentation Links & Resource Manifest

This document catalogs every documentation file available in `@llm/docs/`, organized by category, explaining what each resource contains and when to consult it in future sessions.

---

## 1. Context & Architecture (`@llm/docs/context/`)

- [`BACKEND_ARCHITECTURE.md`](./BACKEND_ARCHITECTURE.md)
  - **Purpose**: Current-to-target process split, database ownership, synchronization model, migration status, and implementation checklist.
  - **Usage**: Consult before changing Tauri startup, LCU services, persistence, or background collection.

- [`CONTEXT.md`](/@llm/docs/context/CONTEXT.md)
  - **Purpose**: Master introduction and onboarding document for LCU Companion.
  - **Usage**: Consult first when starting any new LLM session for system overview, architecture diagram, and directory map.

- [`CURRENT_STATE.md`](/@llm/docs/context/CURRENT_STATE.md)
  - **Purpose**: Live snapshot of application features, Tauri commands, domain models, services, renderers, and verified live test results.
  - **Usage**: Read before adding or modifying any feature to see what is already implemented and registered.

- [`SESSION_HANDOFF_2026-10-11.md`](./SESSION_HANDOFF_2026-10-11.md)
  - **Purpose**: Session handoff for the standalone agent, collection pipeline, server-owned statistics, database contention, and current reconciliation bottleneck.
  - **Usage**: Read first when continuing agent, collector, persistence, or statistics work.

- [`LOGGING.md`](./LOGGING.md)
  - **Purpose**: Shared log format, destinations, retention, redaction rules, and frontend/Rust usage.
  - **Usage**: Consult before adding diagnostics or replacing direct console/print calls.

- [`MIGRATION.md`](/@llm/docs/context/MIGRATION.md)
  - **Purpose**: Chronicle of the FastAPI $\rightarrow$ Tauri 2.0 migration, key technical insights (Rust raw string literals with HTML attributes, HTMX bridge, LCU lockfile discovery), and LLM prompt tips.
  - **Usage**: Reference when encountering Rust string parsing issues or modifying the SSR HTMX bridge.

- [`HTMX_TO_SVELTE.md`](/@llm/docs/context/HTMX_TO_SVELTE.md)
  - **Purpose**: Migration outline and guide for transitioning from Rust SSR HTML + HTMX to Svelte 5 and typed JSON Tauri commands.
  - **Usage**: Master reference and blueprint for the upcoming frontend migration session.

---

## 2. League Client Update (LCU) Integration (`@llm/docs/lcu/`)

- [`LCU.md`](/@llm/docs/lcu/LCU.md)
  - **Purpose**: Deep dive into LCU protocol, authentication (`riot:<token>`), lockfile format, readiness checks, and websocket event subscriptions (`OnJsonApiEvent`).
  - **Usage**: Consult when adding new LCU endpoints or implementing websocket events.

- [`swagger.json`](/@llm/docs/lcu/swagger.json)
  - **Purpose**: Full OpenAPI 2.0/3.0 schema of the League Client Update endpoints, schemas, and parameters.
  - **Usage**: Search for specific endpoint paths and data models.

- [`getting-started-with-the-lcu-api`](/@llm/docs/lcu/getting-started-with-the-lcu-api) & [`getting-started-with-the-lcu-websocket`](/@llm/docs/lcu/getting-started-with-the-lcu-websocket)
  - **Purpose**: Foundational guides on REST and WebSocket interactions with the Riot client.

- [`current_rune_page_sample.json`](/@llm/docs/lcu/current_rune_page_sample.json)
  - **Purpose**: Sample payload returned by `/lol-perks/v1/currentpage`.

---

## 3. Database & Persistence (`@llm/docs/db/`)

- [`DATABASE.md`](/@llm/docs/db/DATABASE.md)
  - **Purpose**: SQLite schema definitions, indexing, and query patterns for `runesets`.
  - **Usage**: Consult when adding new database tables or extending rune metadata.

- [`RUNES.md`](/@llm/docs/db/RUNES.md)
  - **Purpose**: Rune domain model specifications, perk IDs, and serialization schemas.

## 6. External Integrations (`@llm/docs/integrations/`)

- [`PENGU_LOADER.md`](../integrations/PENGU_LOADER.md)
  - **Purpose**: Research findings, compliance caveat, and the proposed optional integration boundary for Pengu Loader.

---

## 4. UI, Styling & Frontend (`@llm/docs/ui/`)

- [`UI.md`](/@llm/docs/ui/UI.md)
  - **Purpose**: Design system principles, Flowbite dark theme specs, color palettes, and component structures.

- [`THEME.md`](/@llm/docs/ui/THEME.md)
  - **Purpose**: Thematic color tokens for rune trees (Precision Yellow, Domination Red, Sorcery Purple, Inspiration Cyan, Resolve Emerald).

- [`FLOWBITE_BUTTONS.md`](/@llm/docs/ui/FLOWBITE_BUTTONS.md)
  - **Purpose**: Flowbite button design patterns and micro-interaction states.

- [`RUNE_TREE_PLAN.md`](/@llm/docs/ui/RUNE_TREE_PLAN.md)
  - **Purpose**: Visual design specifications for the center rune canvas, keystone sizes, and stat mod rows.

- [`htmx.txt`](/@llm/docs/ui/htmx.txt) & [`tauri.llms.txt`](/@llm/docs/ui/tauri.llms.txt)
  - **Purpose**: LLM-optimized quick reference cheat sheets for HTMX 2.0 and Tauri 2.0 APIs.

---

## 5. Historical & Legacy Reference (`@llm/docs/historical/`)

- [`TOOLS_FEATURE_ARCHIVE.md`](/@llm/docs/historical/TOOLS_FEATURE_ARCHIVE.md)
  - **Purpose**: Archive documentation regarding the shelved header tools tab and friend removal utility.

- Contains early planning documents ([`PLAN.md`](/@llm/docs/historical/PLAN.md), [`INDEX.md`](/@llm/docs/historical/INDEX.md), [`IMPLEMENTATION.md`](/@llm/docs/historical/IMPLEMENTATION.md), [`LOGGING.md`](/@llm/docs/historical/LOGGING.md)) from the initial prototyping phase.
