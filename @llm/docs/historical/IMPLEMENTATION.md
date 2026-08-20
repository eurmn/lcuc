# Implementation Summary: Logging Module & Websocket Listener

## Overview

Implemented a **real-time websocket event listener** for League Client Update (LCU) profile changes with comprehensive logging integration. The system monitors summoner icon, level, and border changes via websocket instead of polling.

## Files Created

### 1. `api/services/lcu_websocket.py` (165 lines)
**Low-level websocket client for LCU event streaming**

- **LcuWebsocketListener**: Core websocket connection class
  - Connects to `wss://127.0.0.1:{port}/` with Basic auth
  - Subscribes to `OnJsonApiEvent` messages
  - Filters events by URI (profile and regalia endpoints)
  - Parses JSON messages and dispatches to callbacks
  - Full error handling and logging at each step

- **Key features**:
  - Async/await support with context manager
  - Automatic TLS verification disable for self-signed LCU certs
  - Message validation and type checking
  - Graceful connection closure and cleanup

### 2. `api/services/profile_listener.py` (98 lines)
**Business logic layer for profile change events**

- **ProfileEventListener**: Higher-level event handler
  - Manages websocket connection lifecycle
  - Registers multiple async/sync handlers
  - Extracts and logs profile details:
    - Summoner name, icon ID, level
    - Crest type, prestige crest, ranked tier
  - Distributes events to all registered handlers
  - Robust error handling per handler

- **Key features**:
  - Handler registration/callback system
  - Automatic async/sync handler detection
  - Detailed logging of profile changes
  - Independent error isolation between handlers

### 3. `api/main.py` (Updated)
**FastAPI integration for automatic listener startup**

- **Startup event handler**: Auto-starts profile listener on app startup
- **Shutdown event handler**: Graceful cleanup and task cancellation
- **Background task**: Runs listener in separate asyncio task
- **Callback integration**: Logs all profile change events

- **Key additions**:
  - `_start_profile_listener()` - Creates background listener task
  - `_on_profile_change()` - Event callback for logging
  - `@app.on_event("startup")` - Initialize on server start
  - `@app.on_event("shutdown")` - Cleanup on server stop

### 4. `scripts/demo_websocket_listener.py` (71 lines)
**Standalone demonstration script**

- Waits for League Client detection
- Connects to LCU websocket
- Registers logging handler
- Continuously logs profile changes
- Exit via Ctrl+C

- **Usage**: `python -m scripts.demo_websocket_listener`

### 5. `@docs/LOGGING.md` (271 lines)
**Comprehensive logging & websocket documentation**

- Logging system overview and configuration
- Websocket architecture and event flow diagram
- Connection lifecycle and event format examples
- Handler registration patterns
- Error handling and recovery strategies
- Future extension guidelines

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    FastAPI App (main.py)                │
│  ┌─────────────────────────────────────────────────────┐ │
│  │         ProfileEventListener (Background Task)      │ │
│  │  • Manages websocket lifecycle                      │ │
│  │  • Registers handlers                               │ │
│  │  • Logs profile changes                             │ │
│  │  ┌───────────────────────────────────────────────┐  │ │
│  │  │    LcuWebsocketListener                       │  │ │
│  │  │  • Connect to wss://127.0.0.1:{port}/        │  │ │
│  │  │  • Subscribe to OnJsonApiEvent                │  │ │
│  │  │  • Parse & dispatch events                    │  │ │
│  │  │  └─────────────────────────────────────────┐  │  │ │
│  │  │          LCU WebSocket Connection          │  │  │ │
│  │  │     (Real-time profile/regalia events)      │  │  │ │
│  │  └─────────────────────────────────────────┘  │  │ │
│  │                                                  │  │ │
│  │  Handlers:                                       │  │ │
│  │  • _on_profile_change() → logs event            │  │ │
│  │  • Custom handlers via register_handler()       │  │ │
│  └───────────────────────────────────────────────┘  │ │
│                                                      │ │
│  API Endpoints (unchanged):                         │ │
│  • GET  /api/summoner-profile                       │ │
│  • POST /api/remove-friends                         │ │
│  • GET  /api/health                                 │ │
└─────────────────────────────────────────────────────┘
```

## Event Flow Example

**Scenario**: User changes summoner icon in League Client

```
1. League Client updates /lol-summoner/v1/current-summoner
   ↓
2. LCU sends websocket event with new profileIconId
   ↓
3. LcuWebsocketListener receives JSON:
   [5, "OnJsonApiEvent", {"uri": "/lol-summoner/v1/current-summoner", "data": {...}}]
   ↓
4. Parses URI, matches against PROFILE_URIS
   ↓
5. Calls ProfileEventListener._on_profile_event()
   ↓
6. Logs extracted data:
   "Summoner profile changed: name=YourName icon=42 level=150"
   ↓
7. Invokes registered handlers (_on_profile_change)
   ↓
8. Handler logs final event:
   "Profile changed via websocket: uri=/lol-summoner/v1/current-summoner"
```

## Logging Output Example

```
2026-08-17 21:30:15,123 INFO [api.main] Application startup
2026-08-17 21:30:15,234 DEBUG [api.main] Profile listener task created
2026-08-17 21:30:15,456 INFO [api.services.profile_listener] Profile event listener started
2026-08-17 21:30:15,567 INFO [api.services.lcu_websocket] Connecting to LCU websocket: wss://127.0.0.1:12345/
2026-08-17 21:30:15,789 INFO [api.services.lcu_websocket] Connected to LCU websocket
2026-08-17 21:30:15,901 DEBUG [api.services.lcu_websocket] Subscribed to OnJsonApiEvent
2026-08-17 21:30:16,012 INFO [api.services.lcu_websocket] Profile event received: uri=/lol-summoner/v1/current-summoner
2026-08-17 21:30:16,134 INFO [api.services.profile_listener] Summoner profile changed: name=YourName icon=42 level=150
2026-08-17 21:30:16,245 INFO [api.main] Profile changed via websocket: uri=/lol-summoner/v1/current-summoner
```

## Key Features

✅ **Real-time monitoring** - No polling, immediate event delivery via websocket
✅ **Structured logging** - Consistent format with timestamps and log levels  
✅ **Error resilience** - Connection failures don't crash the app
✅ **Extensible design** - Easy to add more URIs, handlers, or features
✅ **Automatic startup** - Profile listener starts when app starts
✅ **Background operation** - Runs independently from HTTP request handling
✅ **Clean shutdown** - Graceful cleanup on app termination
✅ **Demo script** - Standalone tool for testing without full app

## Configuration

**Log level** via environment variable:
```bash
LCU_COMPANION_LOG_LEVEL=DEBUG python -m uvicorn api.main:app
LCU_COMPANION_LOG_LEVEL=INFO python -m uvicorn api.main:app
```

**Default**: DEBUG (shows all messages)

## Testing

**Validate syntax**:
```bash
python -m py_compile api/services/lcu_websocket.py
python -m py_compile api/services/profile_listener.py
python -m py_compile api/main.py
```

**Run demo script**:
```bash
python -m scripts.demo_websocket_listener
```

**Run full app** (websocket listener starts automatically):
```bash
python -m uvicorn api.main:app --reload
```

## Future Enhancements

1. **Push to frontend**: Send events to frontend via SSE or WebSocket API for live UI updates
2. **Event persistence**: Store profile change history in database
3. **Additional URIs**: Monitor ranked stats, rune pages, champion select
4. **Event filtering**: Allow configurable URI filtering per handler
5. **Metrics**: Add prometheus/datadog metrics for event frequency
6. **Reconnection**: Auto-reconnect websocket on LCU restart

## Files Modified

- `api/main.py` - Added startup/shutdown handlers and background task management

## Files Created

- `api/services/lcu_websocket.py` - Low-level websocket client
- `api/services/profile_listener.py` - Business logic event handler
- `scripts/demo_websocket_listener.py` - Standalone demo script
- `scripts/__init__.py` - Module marker
- `@docs/LOGGING.md` - Comprehensive documentation
