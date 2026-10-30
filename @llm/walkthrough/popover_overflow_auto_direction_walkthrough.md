# Walkthrough — Popover Auto Direction Switching & Viewport Overflow Prevention

Fixed popovers across the application (`GameEntityPopover` and `RunePopover`) from overflowing and clipping off-screen boundaries by implementing dynamic multi-axis direction switching, optimal middleware ordering, and viewport collision constraints via Floating UI.

---

## 1. Problem Description

When hovering build items (such as Malignance in Build Statistics), rune cards, or match history items, popups could spawn off the left or right edges of the screen, clipping content off-screen:
- **Default 2-way flip limitation**: The default Flowbite-Svelte `flip()` middleware only checked the direct opposite placement (e.g. `right` -> `left`). If an element was wide or positioned such that neither side had enough room, Floating UI reverted to the overflowing side.
- **Middleware ordering**: Flowbite-Svelte's default Popper appended `offset` *after* `flip` and `shift`. This meant `flip()` evaluated boundaries with zero offset, and the subsequent offset pushed the popover over the viewport boundary.
- **Full-width table row anchors**: In `BuildStatisticsView`, the popover was triggered by and positioned relative to the full-width `<tr>` element (spanning the whole table width), which forced `placement="right"` to push past the window's right edge and flip to a negative left coordinate.

---

## 2. Changes Made

### A. Popover Placement & Middleware Engine
- Created [`src/lib/utils/popoverPlacement.ts`](src/lib/utils/popoverPlacement.ts):
  - **`getFallbackPlacements`**: Returns a prioritized fallback list for any placement. For example:
    - `"top"` -> `["bottom", "right", "left"]`
    - `"bottom"` -> `["top", "right", "left"]`
    - `"right"` -> `["left", "bottom", "top"]`
    - `"left"` -> `["right", "bottom", "top"]`
  - **`createPopoverMiddlewares`**:
    1. `dom.offset(offsetDistance)`: Executed first so subsequent collision calculations include the offset spacing.
    2. `dom.flip({ fallbackPlacements, fallbackStrategy: "bestFit", crossAxis: true, padding })`: Automatically changes spawn direction if the original direction overflows, picking the best fitting non-overflowing direction.
    3. `dom.shift({ padding, limiter: dom.limitShift() })`: Slides the popover along the axis with safe viewport padding margins (12px) to prevent edge clipping.

### B. Common Game Entity Popover
- Updated [`src/lib/components/common/GameEntityPopover.svelte`](src/lib/components/common/GameEntityPopover.svelte):
  - Integrated `createPopoverMiddlewares`.
  - Added optional `reference` anchor prop (enabling hover trigger on a container while anchoring coordinates to a specific inner element).
  - Configured `offset={0}` on the Popover component so that Popper's internal offset append is a no-op, preserving the pre-calculated `offset -> flip -> shift` execution order.

### C. Rune Card Popover
- Updated [`src/lib/components/runes/RunePopover.svelte`](src/lib/components/runes/RunePopover.svelte):
  - Integrated `createPopoverMiddlewares`.
  - Removed obsolete `left-0` CSS class that could conflict with inline Floating UI positioning.
  - Added support for customizable `placement`, `offset`, and `reference`.

### D. Build Statistics Table Anchor Refinement
- Updated [`src/lib/components/history/BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte):
  - Added `id={`${domId}-cell`}` to the build name/icon `<td>` element.
  - Passed `reference={`#${domId}-cell`}` to `GameEntityPopover`.
  - Hovering anywhere on the row triggers the popover, while positioning anchors directly to the build name cell on the left, giving plenty of horizontal room to spawn to the right, or automatically flipping to `bottom`/`top` if space is constrained.

---

## 3. Verification

- Verified Floating UI middleware resolution with `@floating-ui/dom 1.8.0`.
- Verified that all 4 placement directions (`top`, `bottom`, `left`, `right`) properly execute multi-axis fallbacks and clamp inside viewport bounds with 12px padding.

