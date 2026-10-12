# Walkthrough: In-Process Agent Migration & Build Pipeline Streamlining

## Summary

Migrated the background data collector, LCU WebSocket listener, and statistics engine from an external child process with TCP loopback IPC into **in-process background Tokio tasks** inside `lcu-companion`. This resolves the compilation order conflicts when running `pnpm tauri dev`, removes the need to build a secondary binary upfront, and eliminates TCP networking latency and port collisions on Windows.

Additionally, updated [`AGENT.md`](AGENT.md) to remove the `pwsh -NoProfile -Command` shell guideline and update the architecture documentation.

---

## Changes

### 1. Tauri Configuration
#### [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json)
* Updated `"beforeDevCommand"` from `"pnpm build:agent && pnpm dev"` to `"pnpm dev"`.
* `pnpm tauri dev` now compiles only the main `lcu-companion` binary in a single step without Cargo lock contention.

### 2. Rust In-Process Agent Dispatcher
#### [`src-tauri/src/agent/mod.rs`](src-tauri/src/agent/mod.rs)
* Made `RuntimeState`, `ProgressOperation`, `database_path`, `global_aram_database_path`, and `initialize_database` public.
* Extracted `pub async fn handle_command` and added `pub async fn dispatch_command` for direct in-process command execution without TCP network serialization.
* Exposed `pub async fn collector_loop` and `pub async fn websocket_loop` to allow in-process task spawning.
* Retained `run_process` and `run_process_from_main` so the standalone binary `src-tauri/src/bin/lcu-companion-agent.rs` remains functional if ever needed.

### 3. Agent Supervisor Refactored to In-Process
#### [`src-tauri/src/agent_supervisor.rs`](src-tauri/src/agent_supervisor.rs)
* Replaced child OS process management (`Command`, `Child`), ephemeral port reservation, and TCP stream read/write loops with direct in-process task management.
* On `AgentSupervisor::start()`:
  - Initializes `agent.db` and `global-aram.db`.
  - Instantiates `Arc<RwLock<RuntimeState>>`.
  - Spawns `collector_loop` and `websocket_loop` as background Tokio tasks.
* Preserved the exact same public API:
  - `status(&self) -> AgentStatus`
  - `request(&self, command: &str) -> Result<Value, String>`
  - `request_with_args(&self, command: &str, args: Value) -> Result<Value, String>`
* All Tauri commands in [`src-tauri/src/commands/agent.rs`](src-tauri/src/commands/agent.rs) and the frontend Svelte API remain 100% compatible with zero code changes required.

### 4. Agent Guidelines & Documentation
#### [`AGENT.md`](AGENT.md)
* Removed the `pwsh -NoProfile -Command` guideline as requested.
* Updated the Mission and Architecture sections to document the in-process Tokio task model.

---

## Verification Results

### Build & Dev Pipeline
* `pnpm tauri dev` only needs to trigger `pnpm dev` before running Tauri.
* There is no secondary Cargo compilation or lock contention on startup.
* The frontend receives identical typed responses from `api_agent_status`, `api_statistics`, `api_build_statistics`, and `api_global_aram_config`.
