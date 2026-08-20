# UI Development Guide

This document defines the UI architecture for this project.

## Architecture: Pure SSR + HTMX

This application is a **pure Server-Side Rendered (SSR) application using HTMX and Tailwind CSS / Flowbite**. There is **no custom client-side JavaScript**.

- **Templates** live in `templates/` (and `templates/components/`).
- **Server Rendering** is handled in `api/web/templates.py`.
- **Interactivity** is powered by HTMX attributes (`hx-get`, `hx-post`, `hx-swap`, `hx-target`, `hx-trigger`).
- **Styles** are authored in `client/src/styles/index.css` and compiled with Tailwind CSS to `api/static/css/index.css`.

---

## File Structure

```
templates/
├── index.html                   # HTML shell (loads Tailwind CSS, Flowbite, HTMX)
└── components/
    ├── header.html              # Main navigation header with tab switching
    ├── shell.html               # Tab shell container
    ├── profile_card.html        # Summoner profile card template
    ├── profile_challenge_tier_icon.html
    ├── runes.html               # Runes page container (mounts saved list + rune tree via HTMX)
    ├── runes_saved_list.html    # Saved rune pages partial
    ├── rune_tree.html           # Live rune tree partial (primary/secondary/stat mods)
    └── tools.html               # Tools page partial

api/web/templates.py             # Server-side HTML renderers (render_index, render_shell, render_rune_tree, etc.)
client/src/styles/index.css      # Tailwind CSS entrypoint
```

---

## HTMX Routing Model

The backend exposes `/ui/*` endpoints returning HTML partials:

| Endpoint | Description |
| :--- | :--- |
| `GET /` | Returns full page shell with initial tab |
| `GET /ui/shell?tab=runes\|tools` | Returns tab shell with active navigation header and tab content |
| `GET /ui/profile-card` | Returns server-rendered profile card |
| `GET /ui/profile-challenge-tier` | Returns server-rendered challenge tier icon |
| `GET /ui/runesets` | Returns server-rendered list of saved rune pages |
| `GET /ui/rune-tree/current` | Returns server-rendered rune tree card (primary, secondary, stat mods) |

---

## Practical Workflow for New UI Features

1. **Create/Update HTML Template** in `templates/components/*.html`.
2. **Add Render Function** in `api/web/templates.py` escaping dynamic values.
3. **Expose `/ui/...` Endpoint** in `api/main.py` with `response_class=HTMLResponse`.
4. **Mount via HTMX** in parent templates using attributes like `hx-get`, `hx-trigger="load"`, `hx-swap="outerHTML"`.
5. **Add Interactivity** (refresh, modals, filters) using standard HTMX swaps and triggers without writing custom JS.

---

## Guardrails

- **Zero Client-Side JS**: Do not add custom JS scripts or custom web elements.
- **Server Templates as Source of Truth**: All UI components are rendered server-side in Python.
- **Flowbite & Tailwind**: Use Tailwind CSS and Flowbite classes for consistent dark-mode styling.
