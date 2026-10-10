# Application Logging

LCU Companion uses one operational logging contract across Svelte, Tauri, and
the standalone agent.

## Format

Each record is emitted as:

```text
[2026-10-10T20:00:00.123Z] [INFO] [frontend.history] Loaded 20 matches
```

The fields are UTC timestamp, severity, source, message, and optional context.
Sources identify the process and component, for example:

- `frontend.ipc`
- `frontend.history`
- `tauri.history`
- `tauri.lcu.websocket`
- `agent.collector`
- `agent.websocket`

## Destinations and retention

- Operational logs are written to `%LOCALAPPDATA%\\lcu-companion\\logs\\application.log`.
- Logs are also sent to the developer console.
- Files rotate at 10 MB and retain up to 7 days of rotated log files.
- Raw match-history response archives remain separate under the existing
  match-history response log directory.

Normal builds persist `INFO`, `WARN`, and `ERROR`; debug records are available
in development console output and are not persisted by the release Rust logger.

## Usage

Frontend code imports `logger` from `src/lib/utils/logger.ts`:

```ts
logger.info("frontend.history", "Loaded match history", { count: matches.length });
logger.error("frontend.ipc", "IPC command failed", error);
```

Rust code imports the shared logger module:

```rust
crate::logger::warn("agent.collector", "Detail unavailable");
```

Frontend records are forwarded through the authenticated Tauri application
boundary with `api_log`. Logging failures are intentionally non-fatal.

## Redaction

Never include LCU lockfile tokens, authorization headers, passwords, or full
raw response payloads in operational logs. The Rust sink redacts common token
and authorization forms as a final safeguard. Detailed response archives must
remain separate and should only be written by the existing diagnostic paths.
