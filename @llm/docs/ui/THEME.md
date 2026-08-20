# Design System & Color Palette Guide

This document establishes the official color palette and component styling standards for **LCU Companion**.

---

## 1. Brand Color Palette: Purple / Violet Accent

The primary brand accent color is **Purple** (`purple-600` / `#9333ea`):

| Token | Utility Class | Hex Code | Usage |
| :--- | :--- | :--- | :--- |
| **Brand 50** | `bg-purple-50` / `text-purple-50` | `#faf5ff` | Subtle light tints |
| **Brand 100** | `bg-purple-100` / `text-purple-100` | `#f3e8ff` | Light badge highlights |
| **Brand 400** | `text-purple-400` / `border-purple-400` | `#c084fc` | Active borders, indicators, text highlights |
| **Brand 500** | `bg-purple-500` / `ring-purple-500` | `#a855f7` | Hover states, glowing rings |
| **Brand 600** | `bg-purple-600` / `border-purple-600` | `#9333ea` | **Primary action buttons (Import, Edit, Save)** |
| **Brand 800** | `border-purple-800` / `ring-purple-800` | `#6b21a8` | Button focus rings |
| **Brand 950** | `bg-purple-950/40` | `#3b0764` | Selected item card backgrounds, icon boxes |

---

## 2. Standard Flowbite Component Buttons

### A. Solid Action Button (Import / Edit / Save)
```html
<button
  type="button"
  class="inline-flex shrink-0 items-center gap-2 rounded-xl bg-purple-600 px-4 py-2 text-sm font-semibold text-white shadow-md shadow-purple-950/40 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
>
  <span>Action</span>
</button>
```

### B. Secondary / Cancel Button
```html
<button
  type="button"
  class="inline-flex shrink-0 items-center gap-2 rounded-xl border border-gray-700 bg-gray-800 px-4 py-2 text-sm font-semibold text-gray-200 shadow-sm transition-all hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-4 focus:ring-gray-700 active:scale-95 cursor-pointer"
>
  <span>Cancel</span>
</button>
```

### C. Compact Sidebar Button (Import)
```html
<button
  type="button"
  class="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-purple-500/30 bg-purple-500/10 px-2.5 py-1.5 text-xs font-semibold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-purple-500/40 shadow-sm cursor-pointer"
>
  <span>Import</span>
</button>
```

---

## 3. Dark Theme Card Conventions

- **Main Card Canvas**: `bg-gray-900 border border-gray-800 rounded-2xl p-5`
- **Sub-Cards (Primary Tree, Secondary Tree, Stat Mods)**: `bg-gray-950/80 border border-gray-800 rounded-2xl p-4`
- **Perk Slot Rows**: `bg-gray-900/90 border border-gray-800/80 rounded-xl p-3`
- **Selected Perk**: `ring-2 ring-purple-400 border-2 border-purple-400 bg-purple-950/60 shadow-md`
