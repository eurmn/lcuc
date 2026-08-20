# Documentation Links & Resource Manifest

This document catalogs every documentation file available in `llms/docs/`, organized by category, explaining what each resource contains and when to consult it in future sessions.

---

## 1. Context & Architecture (`llms/docs/context/`)

- [`CONTEXT.md`](tauri/lcu-companion/llms/docs/context/CONTEXT.md)
  - **Purpose**: Master introduction and onboarding document for LCU Companion.
  - **Usage**: Consult first when starting any new LLM session for system overview, architecture diagram, and directory map.

- [`CURRENT_STATE.md`](tauri/lcu-companion/llms/docs/context/CURRENT_STATE.md)
  - **Purpose**: Live snapshot of application features, Tauri commands, domain models, services, renderers, and verified live test results.
  - **Usage**: Read before adding or modifying any feature to see what is already implemented and registered.

- [`MIGRATION.md`](tauri/lcu-companion/llms/docs/context/MIGRATION.md)
  - **Purpose**: Chronicle of the FastAPI $\rightarrow$ Tauri 2.0 migration, key technical insights (Rust raw string literals with HTML attributes, HTMX bridge, LCU lockfile discovery), and LLM prompt tips.
  - **Usage**: Reference when encountering Rust string parsing issues or modifying the SSR HTMX bridge.

---

## 2. League Client Update (LCU) Integration (`llms/docs/lcu/`)

- [`LCU.md`](tauri/lcu-companion/llms/docs/lcu/LCU.md)
  - **Purpose**: Deep dive into LCU protocol, authentication (`riot:<token>`), lockfile format, readiness checks, and websocket event subscriptions (`OnJsonApiEvent`).
  - **Usage**: Consult when adding new LCU endpoints or implementing websocket events.

- [`swagger.json`](tauri/lcu-companion/llms/docs/lcu/swagger.json)
  - **Purpose**: Full OpenAPI 2.0/3.0 schema of the League Client Update endpoints, schemas, and parameters.
  - **Usage**: Search for specific endpoint paths and data models.

- [`getting-started-with-the-lcu-api`](tauri/lcu-companion/llms/docs/lcu/getting-started-with-the-lcu-api) & [`getting-started-with-the-lcu-websocket`](tauri/lcu-companion/llms/docs/lcu/getting-started-with-the-lcu-websocket)
  - **Purpose**: Foundational guides on REST and WebSocket interactions with the Riot client.

- [`current_rune_page_sample.json`](tauri/lcu-companion/llms/docs/lcu/current_rune_page_sample.json)
  - **Purpose**: Sample payload returned by `/lol-perks/v1/currentpage`.

---

## 3. Database & Persistence (`llms/docs/db/`)

- [`DATABASE.md`](tauri/lcu-companion/llms/docs/db/DATABASE.md)
  - **Purpose**: SQLite schema definitions, indexing, and query patterns for `runesets`.
  - **Usage**: Consult when adding new database tables or extending rune metadata.

- [`RUNES.md`](tauri/lcu-companion/llms/docs/db/RUNES.md)
  - **Purpose**: Rune domain model specifications, perk IDs, and serialization schemas.

---

## 4. UI, Styling & Frontend (`llms/docs/ui/`)

- [`UI.md`](tauri/lcu-companion/llms/docs/ui/UI.md)
  - **Purpose**: Design system principles, Flowbite dark theme specs, color palettes, and component structures.

- [`RUNE_TREE_PLAN.md`](tauri/lcu-companion/llms/docs/ui/RUNE_TREE_PLAN.md)
  - **Purpose**: Visual design specifications for the center rune canvas, keystone sizes, and stat mod rows.

- [`htmx.txt`](tauri/lcu-companion/llms/docs/ui/htmx.txt) & [`tauri.llms.txt`](tauri/lcu-companion/llms/docs/ui/tauri.llms.txt)
  - **Purpose**: LLM-optimized quick reference cheat sheets for HTMX 2.0 and Tauri 2.0 APIs.

---

## 5. Historical & Legacy Reference (`llms/docs/historical/`)

- Contains early planning documents ([`PLAN.md`](tauri/lcu-companion/llms/docs/historical/PLAN.md), [`INDEX.md`](tauri/lcu-companion/llms/docs/historical/INDEX.md), [`IMPLEMENTATION.md`](tauri/lcu-companion/llms/docs/historical/IMPLEMENTATION.md), [`LOGGING.md`](tauri/lcu-companion/llms/docs/historical/LOGGING.md)) from the initial prototyping phase.
