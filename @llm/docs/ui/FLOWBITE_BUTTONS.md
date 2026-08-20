# Flowbite Buttons & Dark Mode Guide

Flowbite provides standardized Tailwind CSS utility classes for button components, states, sizes, and colors in dark mode.

---

## 1. Button Variants & Classes (Dark Theme)

### A. Primary / Brand Button (Blue)
```html
<button
  type="button"
  class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800 cursor-pointer"
>
  Default
</button>
```

### B. Secondary / Alternative Button (Dark / Gray)
```html
<button
  type="button"
  class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700 cursor-pointer"
>
  Alternative
</button>
```

### C. Outline / Pill Button with Icon (e.g. Import / Action)
```html
<button
  type="button"
  class="inline-flex items-center gap-1.5 rounded-xl border border-blue-500/30 bg-blue-500/10 px-3 py-1.5 text-xs font-semibold text-blue-300 transition-all hover:border-blue-500 hover:bg-blue-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-blue-500/40 shadow-sm cursor-pointer"
>
  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
  </svg>
  <span>Import</span>
</button>
```

### D. Danger / Delete Button (Red)
```html
<button
  type="button"
  class="focus:outline-none text-white bg-red-700 hover:bg-red-800 focus:ring-4 focus:ring-red-300 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900 cursor-pointer"
>
  Delete
</button>
```

### E. Small / Compact Action Button (xs / sm)
```html
<button
  type="button"
  class="px-3 py-1.5 text-xs font-medium text-center inline-flex items-center text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 cursor-pointer"
>
  Action
</button>
```

---

## 2. Badges & Indicators

### Pulse Indicator Badge
```html
<span class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-400">
  <span class="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
  Active
</span>
```
