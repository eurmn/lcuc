# PLAN - Next version

## 1) Startup behavior (wait for client + login)

Goal: the app must work even if it starts before League Client.

Plan:

1. Keep waiting for `LeagueClientUx` process (already implemented).
2. Read `--app-port` and `--remoting-auth-token` from process args (already implemented).
3. Connect to LCU with HTTPS verification disabled and Basic auth `riot:<token>` (already implemented).
4. Add a **login-ready wait** step before profile/runes features:
   - Poll `GET /lol-summoner/v1/current-summoner`
   - Continue only when response is `200` and summoner data is valid.

Example (Python, simplified):

```python
async def wait_until_logged_in(lcu):
    while True:
        try:
            data = await lcu.request_json("GET", "/lol-summoner/v1/current-summoner")
            if isinstance(data, dict) and data.get("summonerId"):
                return data
        except Exception:
            pass
        await asyncio.sleep(1.0)
```

## 2) Real-time updates with LCU websocket (icon/level/border)

Goal: update UI immediately when profile changes.

Use websocket events instead of polling:

1. Connect to `wss://127.0.0.1:{port}/` with Basic auth and TLS verify disabled.
2. Subscribe to `OnJsonApiEvent`.
3. Handle only these URIs:
   - `/lol-summoner/v1/current-summoner` (name, icon, level)
   - `/lol-regalia/v2/current-summoner/regalia` (ranked/theme border)
4. On matching event:
   - refresh cached assets (CDragon module)
   - push updated profile state to frontend (HTMX endpoint or SSE later)

Example (Python, simplified):

```python
import base64
import json
import aiohttp

async def listen_profile_events(port: int, token: str):
    auth = base64.b64encode(f"riot:{token}".encode()).decode()
    headers = {"Authorization": f"Basic {auth}"}
    async with aiohttp.ClientSession(headers=headers) as session:
        async with session.ws_connect(f"wss://127.0.0.1:{port}/", ssl=False) as ws:
            await ws.send_json([5, "OnJsonApiEvent"])
            async for msg in ws:
                if msg.type != aiohttp.WSMsgType.TEXT:
                    continue
                payload = json.loads(msg.data)
                if len(payload) < 3:
                    continue
                event = payload[2]
                uri = event.get("uri")
                if uri in {
                    "/lol-summoner/v1/current-summoner",
                    "/lol-regalia/v2/current-summoner/regalia",
                }:
                    # Re-fetch profile + regalia and update state/UI
                    pass
```

## 3) Logging improvements (safe scope)

Keep logs focused on app-owned operations only.

Plan:

1. Structured logs (JSON-like fields) for:
   - process detection,
   - auth/connect,
   - API call start/end/status,
   - asset cache hit/miss/download,
   - websocket event handled.
2. Add request IDs to correlate one frontend action with backend steps.
3. Add log levels:
   - `DEBUG` for dev detail,
   - `INFO` for normal lifecycle,
   - `WARNING/ERROR` for failures.
4. Do **not** log or inspect unrelated Riot Client traffic.

## 4) Delivery steps

1. Implement `wait_until_logged_in`.
2. Add websocket listener service for profile/regalia events.
3. On event, refresh profile payload + border/icon cache.
4. Update header UI live from refreshed state.
5. Validate with manual test:
   - client closed -> app waits
   - login -> profile appears
   - icon/level/border change -> UI updates without reload.
