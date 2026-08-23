# UI Development Guide & Architecture

This document defines the frontend UI architecture, component hierarchy, design system, Flowbite patterns, and motion design system for **LCU Companion**.

---

## 1. UI Architecture Overview

The application uses a **Client-Rendered Reactive Architecture** built with **Svelte 5 (Runes)**, **TypeScript**, and **Tailwind CSS 4.0** inside a native **Tauri 2.0** desktop container. All backend interaction happens asynchronously through strongly typed Tauri IPC commands (`invoke("api_*")`), completely eliminating legacy Server-Side Rendered (SSR) HTML strings.

### Key Architectural Principles
- **Modern Svelte 5 Reactivity**: State and effects are managed natively with runes (`$state`, `$derived`, `$props`, `$effect`).
- **Flowbite Design System**: UI components adhere to [Flowbite](/@llm/docs/ui/flowbite/) design patterns, token conventions, and component styles.
- **Declarative Motion Design**: Fluid layout animations, FLIP transitions, and smooth expand/collapse states are powered by [`@humanspeak/svelte-motion`](/@llm/docs/ui/motion_svelte.llm.txt).
- **Thematic Rune Color Palettes**: Colors dynamically align with League of Legends rune paths (Precision amber, Domination red, Sorcery purple, Inspiration cyan, Resolve emerald) defined in [`src/lib/utils/theme.ts`](/src/lib/utils/theme.ts).

---

## 2. Component Hierarchy & File Structure

```text
src/
├── lib/
│   ├── api/                            # Strongly typed Tauri IPC invoke clients
│   │   ├── client.ts                   # Base invoke wrapper with error handling & logging
│   │   ├── profile.ts                  # Summoner profile and challenge tier API endpoints
│   │   ├── runes.ts                    # Live overview, saved runesets, import/export APIs
│   │   ├── rune_builder.ts             # Telemetry-backed rune tree & stats APIs
│   │   └── champions.ts                # Champion list and favorites toggle APIs
│   ├── components/                     # Svelte 5 Reactive UI Components
│   │   ├── TitleBar.svelte             # Custom desktop title bar with drag region & window controls
│   │   ├── ProfileCard.svelte          # Summoner avatar, Riot ID, level, and ranked challenge badge
│   │   ├── ChampionSelectModal.svelte  # Flowbite-styled modal overlay for picking rune page champion
│   │   ├── sidebar/                    # Dual-list collapsible sidebar
│   │   │   ├── RunesSidebar.svelte     # Main sidebar container with motion transition wrapper
│   │   │   ├── SelectedChampionSlot.svelte # Active champion filter chip with favorites scroll carousel
│   │   │   ├── SavedRunesList.svelte   # Local SQLite custom runesets with delete/assign actions
│   │   │   └── ClientRunesList.svelte  # Live League client rune pages with import action
│   │   └── runes/                      # Main rune tree canvas & interactive builder
│   │       ├── RuneCanvas.svelte       # Top action bar, role/tier filters, and layout grid
│   │       ├── PrimaryPathCard.svelte  # Primary path selector & keystone / tier perk matrix
│   │       ├── SecondaryPathCard.svelte# Secondary path selector & 2-perk cross-tier selection
│   │       ├── StatModsSection.svelte  # Offense, flex, and defense stat shards
│   │       ├── KeystoneCard.svelte     # Large keystone card with winrate / pickrate telemetry
│   │       ├── RuneCard.svelte         # Perk slot card with telemetry badge & selection ring
│   │       └── StatShardsCards.svelte  # Stat shard pill options
│   ├── types/                          # TypeScript DTO models mirroring Rust backend
│   │   ├── profile.ts
│   │   ├── runes.ts
│   │   ├── rune_builder.ts
│   │   └── champions.ts
│   └── utils/
│       └── theme.ts                    # Thematic rune path colors and badge mappings
├── App.svelte                          # Root layout, top-level state coordination & modals
├── main.ts                             # Application entrypoint mounting Svelte root
└── styles.css                          # Tailwind CSS 4 imports, theme tokens & custom scrollbars
```

---

## 3. Flowbite Component System Usage

Flowbite provides the standardized visual language and component ergonomics across the entire app. Reference documentation is mirrored under [`@llm/docs/ui/flowbite/`](/@llm/docs/ui/flowbite/):

### A. Buttons & Actions ([`@llm/docs/ui/flowbite/components/buttons.md`](/@llm/docs/ui/flowbite/components/buttons.md))
- **Primary Brand / Action**: Solid purple brand buttons (`bg-purple-600 hover:bg-purple-500 text-white shadow-md focus:ring-4 focus:ring-purple-800 rounded-xl`) for primary operations like "Save changes" or active filters.
- **Solid Dark / Secondary**: Borderless dark buttons (`bg-gray-800 hover:bg-gray-700 text-white focus:ring-4 focus:ring-gray-700 rounded-xl`) for "Edit", "Cancel", and secondary modal dismiss actions.
- **Gradient Duotone**: Gradient mesh buttons (`bg-gradient-to-br from-purple-600 to-blue-500 hover:bg-gradient-to-bl text-white shadow-md focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-800 rounded-xl`) for standout operations like "Send to Client".
- **Ghost Buttons**: Transparent lightweight triggers (`bg-transparent hover:bg-gray-800 text-gray-400 hover:text-white rounded-lg`) for window controls, icon buttons, and sub-actions.
- **Fixed Action Sizes**: Standard action button widths (e.g. `w-20` for 5rem Edit/Cancel, `w-32` for 8rem Save/Send) ensure layout stability between edit/view mode toggles.

### B. Modals & Dialogs ([`@llm/docs/ui/flowbite/components/modal.md`](/@llm/docs/ui/flowbite/components/modal.md))
- Used in [`ChampionSelectModal.svelte`](/src/lib/components/ChampionSelectModal.svelte) with a backdrop overlay (`bg-black/75 backdrop-blur-sm`), header search input with leading heroicon, card grid with scroll area, and standard footer actions.

### C. Badges & Pills ([`@llm/docs/ui/flowbite/components/badge.md`](/@llm/docs/ui/flowbite/components/badge.md))
- Count indicators in sidebar headers (`bg-gray-800 text-gray-400 border border-gray-700/60 rounded-full px-2 py-0.5 text-[11px]`).
- Winrate / pickrate metrics on rune cards (`bg-purple-950/60 text-purple-300 border border-purple-500/30 rounded-md text-[10px]`).

### D. Input Groups & Search ([`@llm/docs/ui/flowbite/components/forms.md`](/@llm/docs/ui/flowbite/components/forms.md))
- Flowbite text inputs with dark theme styles (`bg-gray-900 border-gray-700 text-white placeholder-gray-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 rounded-xl`).

---

## 4. Motion Design System (`@humanspeak/svelte-motion`)

The application integrates **Svelte Motion** ([`@llm/docs/ui/motion_svelte.llm.txt`](/@llm/docs/ui/motion_svelte.llm.txt) / [motion.svelte.page](https://motion.svelte.page)) to deliver physics-based, 60fps layout transitions.

### General Motion Design Scope
Motion design in the UI is applied purposefully to enhance spatial awareness, avoid abrupt layout jumps, and provide natural feedback during UI state changes:
1. **Layout Transitions (FLIP)**: Components smoothly animate position and dimensions (`layout="position"` or `layout={true}`) when neighboring elements resize or collapse.
2. **Spring Physics & Custom Easings**: Transitions use cubic-bezier curves (e.g. `[0.16, 1, 0.3, 1]`) to maintain a snappy, native desktop feel.
3. **Shared Layout Groups**: `<LayoutGroup>` coordinates synchronized animations across multiple sibling or nested components without layout thrashing.

### Sidebar Motion Implementation Example
The collapsible dual-list sidebar ([`RunesSidebar.svelte`](/src/lib/components/sidebar/RunesSidebar.svelte), [`ClientRunesList.svelte`](/src/lib/components/sidebar/ClientRunesList.svelte), [`SavedRunesList.svelte`](/src/lib/components/sidebar/SavedRunesList.svelte)) serves as the primary implementation example:
- **Collapsible Aside (`MotionAside`)**:
  - Animates width between expanded (`17rem` / `w-[272px]`) and collapsed (`4.5rem` / `w-[72px]`) states smoothly over 350ms.
- **Morphing Text & Headers (`MotionSpan`)**:
  - Category titles ("League Client Pages" ↔ "LEAGUE", "Companion Saved Runes" ↔ "SAVED") transition smoothly between full labels and condensed badges.
- **Reordering & Item Transitions (`MotionLi`, `MotionDiv`)**:
  - Rune page list items animate layout positions smoothly when items are added, removed, or filtered by champion.

```svelte
<!-- Example from sidebar components -->
<LayoutGroup>
  <MotionAside
    layout
    transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
    class="relative flex flex-col h-full bg-gray-950 border-r border-gray-800/80 select-none {isOpen ? 'w-[272px]' : 'w-[72px]'}"
  >
    <!-- Content smoothly reflows -->
  </MotionAside>
</LayoutGroup>
```

---

## 5. Iconography Standard

All visual icons across Svelte components use the Iconify utility class system with `@iconify-json/heroicons` and `@iconify-json/flowbite`:
- **Syntax**: `<span class="icon-[<collection>--<icon-name>] h-4.5 w-4.5"></span>`
- **Heroicons**: Navigation, edit triggers, status badges (`icon-[heroicons--pencil-square-16-solid]`, `icon-[heroicons--bolt-16-solid]`, `icon-[heroicons--star]`).
- **Flowbite Icons**: Specific functional icons (`icon-[flowbite--floppy-disk-solid]`).
- **Never Use Raw SVG Strings**: Raw inline SVGs are disallowed in favor of consistent Iconify classes.

