# Quick Start: Websocket Event Listener

## TL;DR

The app now automatically listens to LCU profile changes via websocket and logs them.

### What Changed?

1. **New modules** for websocket listening and profile event handling
2. **Automatic startup** - Profile listener starts when FastAPI app starts
3. **Logging to console** - All profile changes are logged with timestamps

### How to Use

#### Option 1: Run the Full App (Auto-starts Listener)

```bash
python -m uvicorn api.main:app --reload
```

Open http://localhost:8000 and watch the console for profile change logs.

#### Option 2: Run Demo Script Only

```bash
python -m scripts.demo_websocket_listener
```

This script:
- Waits for League Client to start
- Connects to LCU websocket
- Logs all profile changes
- Exits with Ctrl+C

#### Option 3: Use in Custom Code

```python
import asyncio
from api.core.logging import configure_logging, get_logger
from api.services.lcu_api import wait_for_client_credentials
from api.services.profile_listener import ProfileEventListener

logger = get_logger("my_app")

async def handle_event(uri: str, event_data: dict):
    logger.info(f"Profile changed: {uri}")

async def main():
    configure_logging()
    credentials = await wait_for_client_credentials()
    listener = ProfileEventListener(credentials)
    listener.register_handler(handle_event)
    await listener.start()

if __name__ == "__main__":
    asyncio.run(main())
```

## What Gets Logged?

When you change your summoner icon, level, or border in League Client:

```
2026-08-17 21:30:16,012 INFO [api.services.lcu_websocket] Profile event received: uri=/lol-summoner/v1/current-summoner
2026-08-17 21:30:16,134 INFO [api.services.profile_listener] Summoner profile changed: name=YourName icon=42 level=150
2026-08-17 21:30:16,245 INFO [api.main] Profile changed via websocket: uri=/lol-summoner/v1/current-summoner
```

## Watched Events

These URI changes are monitored:

- `/lol-summoner/v1/current-summoner` - Summoner icon, level, name
- `/lol-regalia/v2/current-summoner/regalia` - Ranked border, prestige crest

## Architecture

```
LcuWebsocketListener (Low-level)
    ↓
ProfileEventListener (Business logic)
    ↓
Handlers (Custom code)
    ├─→ Log to console
    ├─→ Update database
    └─→ Push to frontend
```

## Modules

| Module | Purpose |
|--------|---------|
| `api/services/lcu_websocket.py` | Connect to LCU websocket, parse events |
| `api/services/profile_listener.py` | Handle profile changes, call handlers |
| `api/main.py` | Auto-start listener on app startup |
| `scripts/demo_websocket_listener.py` | Standalone demo |

## Configuration

**Change log level** (default: DEBUG):
```bash
LCU_COMPANION_LOG_LEVEL=INFO python -m uvicorn api.main:app
LCU_COMPANION_LOG_LEVEL=WARNING python -m uvicorn api.main:app
LCU_COMPANION_LOG_LEVEL=ERROR python -m uvicorn api.main:app
```

## Troubleshooting

**No logs appearing?**
- Make sure League Client is running before app starts
- Check log level: `LCU_COMPANION_LOG_LEVEL=DEBUG`
- Try demo script: `python -m scripts.demo_websocket_listener`

**Connection failed?**
- Ensure League Client is running
- Check if port/token are correct (app logs them)
- Check firewall isn't blocking localhost

**Handler not being called?**
- Check that handler is registered: `listener.register_handler(my_handler)`
- Handler must accept `uri: str` and `event_data: dict` params
- Check logs for errors in handler

## Next Steps

1. **Run the demo**: `python -m scripts.demo_websocket_listener`
2. **Start League Client**: Open the game launcher and log in
3. **Change your icon**: Try changing summoner icon in client
4. **Watch logs**: You should see profile change events logged

For detailed info, see:
- `@docs/LOGGING.md` - Full documentation
- `@docs/IMPLEMENTATION.md` - Architecture and design
- `@docs/LCU.md` - LCU API details
- `@docs/PLAN.md` - Project roadmap
