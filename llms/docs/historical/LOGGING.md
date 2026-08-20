# Logging & Websocket Events Guide

This document describes the logging infrastructure and real-time websocket event handling for LCU profile changes.

## 1) Logging System

The app uses Python's standard `logging` module with centralized configuration in `api/core/logging.py`.

### Configuration

- **Logger setup**: `configure_logging()` initializes the logging system with:
  - Format: `%(asctime)s %(levelname)s [%(name)s] %(message)s`
  - Log level: controlled by `LCU_COMPANION_LOG_LEVEL` env var (default: `DEBUG`)
  - Example: `LCU_COMPANION_LOG_LEVEL=INFO python -m uvicorn api.main:app`

### Usage

Every module should get its logger at the top level:

```python
from api.core.logging import get_logger

logger = get_logger("api.services.my_module")
logger.debug("Debug message")
logger.info("Info message")
logger.warning("Warning message")
logger.error("Error message")
logger.exception("Error with traceback")
```

### Log Levels

- **DEBUG**: Detailed info for development (function entry/exit, variable values, state changes)
- **INFO**: Normal lifecycle events (process detection, connection established, feature operations)
- **WARNING**: Recoverable issues (missing optional data, retry attempts, degraded functionality)
- **ERROR**: Failures that prevent operation (connection errors, invalid responses, exceptions)

## 2) Websocket Event System

The app connects to the LCU websocket to receive real-time profile change events instead of polling.

### Architecture

**Three-layer design:**

1. **LcuWebsocketListener** (`api/services/lcu_websocket.py`)
   - Low-level websocket client
   - Handles connection, subscription, message parsing
   - Calls registered callback on matching URI events

2. **ProfileEventListener** (`api/services/profile_listener.py`)
   - Business logic layer
   - Registers handlers for profile changes
   - Logs event details (summoner name, icon, level, border type)
   - Distributes events to registered handlers

3. **Integration in FastAPI** (`api/main.py`)
   - Auto-starts on app startup
   - Runs as background task
   - Logs profile changes to output

### Connection Flow

```
1. App startup
   ↓
2. wait_for_client_credentials() → detects LeagueClientUx process
   ↓
3. LcuWebsocketListener.connect() → wss://127.0.0.1:{port}/
   ↓
4. Send [5, "OnJsonApiEvent"] subscription
   ↓
5. Receive events matching /lol-summoner/v1/current-summoner and /lol-regalia/v2/current-summoner/regalia
   ↓
6. Parse URI and event data
   ↓
7. Call registered handlers (e.g., log to console/file)
```

### Event Format

Websocket messages are JSON arrays:

```json
[
  5,
  "OnJsonApiEvent",
  {
    "uri": "/lol-summoner/v1/current-summoner",
    "eventType": "Create",
    "data": {
      "summonerId": 12345,
      "displayName": "YourSummonerName",
      "profileIconId": 42,
      "summonerLevel": 150,
      "gameName": "GameName",
      "tagLine": "NA1",
      ...
    }
  }
]
```

### Tracked URIs

- **`/lol-summoner/v1/current-summoner`**
  - Fires on: summoner icon, level, display name, or game name/tag changes
  - Logged fields: `displayName`, `profileIconId`, `summonerLevel`

- **`/lol-regalia/v2/current-summoner/regalia`**
  - Fires on: ranked tier, border theme, or prestige crest changes
  - Logged fields: `crestType`, `selectedPrestigeCrest`, `highestRankedEntry.tier`

### Example Log Output

```
2026-08-17 21:30:15,123 INFO [api.services.lcu_websocket] Connecting to LCU websocket: wss://127.0.0.1:12345/
2026-08-17 21:30:15,456 INFO [api.services.lcu_websocket] Connected to LCU websocket
2026-08-17 21:30:15,789 DEBUG [api.services.lcu_websocket] Subscribed to OnJsonApiEvent
2026-08-17 21:30:16,012 INFO [api.services.lcu_websocket] Profile event received: uri=/lol-summoner/v1/current-summoner
2026-08-17 21:30:16,034 INFO [api.services.profile_listener] Summoner profile changed: name=YourName icon=42 level=150
2026-08-17 21:30:16,045 INFO [api.main] Profile changed via websocket: uri=/lol-summoner/v1/current-summoner
```

## 3) Using the Websocket Listener

### In the FastAPI App (Automatic)

The profile listener starts automatically when the app starts (see `api/main.py`):

```python
@app.on_event("startup")
async def startup_event() -> None:
    logger.info("Application startup")
    await _start_profile_listener()
```

Logs are printed to the console with timestamps and log levels.

### In Custom Code (Manual)

```python
from api.core.logging import configure_logging, get_logger
from api.services.lcu_api import wait_for_client_credentials
from api.services.profile_listener import ProfileEventListener

logger = get_logger("my_module")

async def my_handler(uri: str, event_data: dict):
    logger.info(f"Profile event: {uri}")

async def main():
    configure_logging()
    credentials = await wait_for_client_credentials()
    listener = ProfileEventListener(credentials)
    listener.register_handler(my_handler)
    await listener.start()

asyncio.run(main())
```

### Using the Demo Script

A standalone demo is provided in `scripts/demo_websocket_listener.py`:

```bash
python -m scripts.demo_websocket_listener
```

This script:
1. Waits for the League Client to start
2. Connects to the LCU websocket
3. Logs all profile changes to the console
4. Runs until interrupted (Ctrl+C)

Example output:
```
2026-08-17 21:30:15,123 INFO [scripts.demo_websocket_listener] Starting LCU Websocket Listener Demo
2026-08-17 21:30:15,456 INFO [scripts.demo_websocket_listener] Waiting for League Client to start...
2026-08-17 21:30:20,789 INFO [scripts.demo_websocket_listener] Client detected: port=12345 pid=9876
2026-08-17 21:30:22,012 INFO [scripts.demo_websocket_listener] LCU API is ready
2026-08-17 21:30:22,345 INFO [scripts.demo_websocket_listener] Connecting to LCU websocket...
2026-08-17 21:30:25,678 INFO [api.services.lcu_websocket] Connecting to LCU websocket: wss://127.0.0.1:12345/
2026-08-17 21:30:25,901 INFO [api.services.lcu_websocket] Connected to LCU websocket
2026-08-17 21:30:26,234 INFO [scripts.demo_websocket_listener] >>> Profile change handler invoked <<<
2026-08-17 21:30:26,456 INFO [scripts.demo_websocket_listener] URI: /lol-summoner/v1/current-summoner
2026-08-17 21:30:26,678 INFO [scripts.demo_websocket_listener] Full event data: {...}
```

## 4) Handler Callbacks

Handlers can be synchronous or asynchronous:

### Async Handler (Recommended)

```python
async def handle_profile_event(uri: str, event_data: dict):
    # Can use await here
    profile = event_data.get("data", {})
    logger.info(f"Icon changed: {profile.get('profileIconId')}")

listener.register_handler(handle_profile_event)
```

### Sync Handler

```python
def handle_profile_event(uri: str, event_data: dict):
    profile = event_data.get("data", {})
    logger.info(f"Icon changed: {profile.get('profileIconId')}")

listener.register_handler(handle_profile_event)
```

Multiple handlers can be registered; they run sequentially and exceptions in one don't affect others.

## 5) Error Handling

Errors in websocket operations are logged but won't crash the app:

- Connection failures: logged as WARNING/ERROR, app can still serve API requests
- Message parsing errors: logged as WARNING, next message is processed
- Handler exceptions: logged as ERROR, other handlers still run
- Websocket closure: logged as INFO, app continues running

Reconnection logic: If the LCU websocket closes, the listener stops. The next app restart will re-establish the connection.

## 6) Future Extensions

This architecture supports adding more event types and handlers:

1. **Add more URIs to monitor** in `LcuWebsocketListener.PROFILE_URIS`:
   ```python
   PROFILE_URIS = {
       "/lol-summoner/v1/current-summoner",
       "/lol-regalia/v2/current-summoner/regalia",
       "/lol-ranked/v1/current-ranked-stats",  # New
   }
   ```

2. **Add business logic in ProfileEventListener**:
   ```python
   elif uri == "/lol-ranked/v1/current-ranked-stats":
       logger.info("Ranked stats changed")
   ```

3. **Push updates to frontend** via SSE or WebSocket API endpoints for live UI updates.

## 7) Performance Notes

- **Polling vs Websocket**: Websocket reduces server load by avoiding repeated HTTP requests
- **Memory**: One active listener connection + queued messages; negligible impact
- **Bandwidth**: Websocket connection is persistent; small message payloads
- **Latency**: Events are delivered within seconds of LCU state changes
