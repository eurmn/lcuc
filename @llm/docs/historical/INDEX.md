# Documentation Index

## Overview

This directory contains comprehensive documentation for the LCU Companion project, with a focus on the newly implemented websocket event listener and logging system.

## Quick Links

### For First-Time Users
Start here → **[QUICKSTART.md](QUICKSTART.md)**
- Quick reference guide
- How to run the app/demo
- Example logs
- Troubleshooting

### For Developers
→ **[LOGGING.md](LOGGING.md)**
- Logging system architecture
- Websocket event handling details
- Handler registration patterns
- Error handling strategies
- Future extensions

### For Architects
→ **[IMPLEMENTATION.md](IMPLEMENTATION.md)**
- Technical architecture overview
- File descriptions
- Event flow diagrams
- Configuration options
- Testing instructions

### Rune Builder & Telemetry Integration
→ **[RUNE_BUILDER_INTEGRATION.md](RUNE_BUILDER_INTEGRATION.md)**
- Modular `RuneStatsProvider` trait & Coachless.gg API specs
- Champion selection roster & name detection
- Individual card-based rune layout & Flowbite dark styling
- Dynamic WPA color indicator scale (-1.0% red -> white -> +1.0% green)

### Executive Summary
→ **[README_IMPLEMENTATION.md](README_IMPLEMENTATION.md)**
- What was built
- Key features
- Files created/modified
- Verification results


## Document Descriptions

### [QUICKSTART.md](QUICKSTART.md) - Quick Reference
**Best for**: Getting started quickly, running demos
- TL;DR section
- 3 usage options (full app, demo script, custom code)
- Configuration
- Troubleshooting tips
- Module overview table

### [LOGGING.md](LOGGING.md) - Complete Technical Guide
**Best for**: Deep understanding of logging and websocket systems
- Logging system configuration and usage
- Websocket architecture and connection flow
- Event format specifications with JSON examples
- Handler callback patterns (async and sync)
- Error handling and recovery
- Performance notes
- Future extension guidelines

### [IMPLEMENTATION.md](IMPLEMENTATION.md) - Technical Details
**Best for**: Understanding the implementation and architecture
- Architecture overview with diagrams
- File descriptions (LcuWebsocketListener, ProfileEventListener, etc.)
- Event flow examples with scenarios
- Logging output examples
- Configuration options
- Feature checklist
- Testing procedures
- Future enhancements

### [README_IMPLEMENTATION.md](README_IMPLEMENTATION.md) - Executive Summary
**Best for**: Project overview and status
- Implementation summary
- What was implemented
- Files created/modified with line counts
- Example log output
- Architecture diagram
- Feature list with checkmarks
- Verification results

### [LCU.md](LCU.md) - LCU Integration Reference
**Best for**: Understanding the LCU API and current modules
- LCU model and authentication flow
- Current module map (lcu_api.py, lcu.py, cdragon.py)
- Current profile pipeline
- Friend removal pipeline
- Websocket usage recommendations
- Endpoints used today
- Data contracts and extension strategy
- Quick extension checklist

### [PLAN.md](PLAN.md) - Project Roadmap
**Best for**: Understanding project goals and next phases
- Startup behavior (wait for client + login)
- Real-time updates with websocket
- Logging improvements
- Delivery steps

### [flowbite_links.txt](flowbite_links.txt)
- Reference links for UI components

### [swagger.json](swagger.json)
- API specification

### [getting-started-with-the-lcu-api/](getting-started-with-the-lcu-api/)
- LCU API getting started guide

### [getting-started-with-the-lcu-websocket/](getting-started-with-the-lcu-websocket/)
- LCU websocket getting started guide

### [examples.html](examples.html)
- HTML examples and reference

## Reading Paths

### Path 1: New to the Project
1. [QUICKSTART.md](QUICKSTART.md) - Get running quickly
2. [LOGGING.md](LOGGING.md) - Understand the logging system
3. [LCU.md](LCU.md) - Learn about LCU integration
4. [PLAN.md](PLAN.md) - Understand future roadmap

### Path 2: Developer Implementation
1. [README_IMPLEMENTATION.md](README_IMPLEMENTATION.md) - Overview
2. [IMPLEMENTATION.md](IMPLEMENTATION.md) - Architecture
3. [LOGGING.md](LOGGING.md) - Detailed logging/websocket guide
4. Code in `api/services/lcu_websocket.py` and `api/services/profile_listener.py`

### Path 3: Architecture & Design
1. [IMPLEMENTATION.md](IMPLEMENTATION.md) - Architecture diagrams
2. [LOGGING.md](LOGGING.md) - Connection flows
3. [LCU.md](LCU.md) - Module map
4. [PLAN.md](PLAN.md) - Feature roadmap

### Path 4: Operations & Troubleshooting
1. [QUICKSTART.md](QUICKSTART.md) - Troubleshooting section
2. [LOGGING.md](LOGGING.md) - Error handling section
3. [IMPLEMENTATION.md](IMPLEMENTATION.md) - Testing procedures

## Key Concepts

### Logging Levels
- **DEBUG**: Development details (default)
- **INFO**: Normal lifecycle events
- **WARNING**: Recoverable issues
- **ERROR**: Failures preventing operation

### Event System
- **WebSocket**: Real-time event delivery from LCU
- **URIs Monitored**: 
  - `/lol-summoner/v1/current-summoner` (profile)
  - `/lol-regalia/v2/current-summoner/regalia` (border)
- **Handlers**: Registered callbacks on profile changes

### Modules
- **LcuWebsocketListener**: Low-level websocket client
- **ProfileEventListener**: Business logic event handler
- **FastAPI Integration**: Auto-startup and cleanup

## Quick Commands

**Run full app with logging**:
```bash
python -m uvicorn api.main:app --reload
```

**Run demo script**:
```bash
python -m scripts.demo_websocket_listener
```

**Change log level**:
```bash
LCU_COMPANION_LOG_LEVEL=INFO python -m uvicorn api.main:app
```

**Verify syntax**:
```bash
python -m py_compile api/services/lcu_websocket.py
python -m py_compile api/services/profile_listener.py
```

## File Locations

```
@docs/
├── INDEX.md (you are here)
├── QUICKSTART.md
├── LOGGING.md
├── IMPLEMENTATION.md
├── README_IMPLEMENTATION.md
├── LCU.md
├── PLAN.md
├── flowbite_links.txt
├── swagger.json
├── examples.html
├── lcu/
│   ├── examples.html
│   ├── getting-started-with-the-lcu-api/
│   └── getting-started-with-the-lcu-websocket/
```

## Questions?

**Q: How do I get started?**
A: Read [QUICKSTART.md](QUICKSTART.md)

**Q: How does the websocket work?**
A: Read [LOGGING.md](LOGGING.md) section 2

**Q: How is it architected?**
A: Read [IMPLEMENTATION.md](IMPLEMENTATION.md)

**Q: What was implemented?**
A: Read [README_IMPLEMENTATION.md](README_IMPLEMENTATION.md)

**Q: What's the plan for the future?**
A: Read [PLAN.md](PLAN.md)

**Q: What LCU endpoints are available?**
A: Read [LCU.md](LCU.md)
