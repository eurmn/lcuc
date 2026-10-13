import * as dom from "@floating-ui/dom";
import type { Middleware, Placement } from "@floating-ui/dom";

export type PopoverPlacement = "top" | "bottom" | "left" | "right";

/**
 * Returns an ordered array of fallback placements when the preferred placement overflows.
 * Evaluates the opposite side first, followed by perpendicular directions.
 */
export function getFallbackPlacements(placement: Placement = "top"): Placement[] {
  switch (placement) {
    case "top":
      return ["bottom", "right", "left"];
    case "bottom":
      return ["top", "right", "left"];
    case "right":
      return ["left", "bottom", "top"];
    case "left":
      return ["right", "bottom", "top"];
    case "top-start":
      return ["bottom-start", "right-start", "left-start"];
    case "top-end":
      return ["bottom-end", "right-end", "left-end"];
    case "bottom-start":
      return ["top-start", "right-start", "left-start"];
    case "bottom-end":
      return ["top-end", "right-end", "left-end"];
    case "right-start":
      return ["left-start", "bottom-start", "top-start"];
    case "right-end":
      return ["left-end", "bottom-end", "top-end"];
    case "left-start":
      return ["right-start", "bottom-start", "top-start"];
    case "left-end":
      return ["right-end", "bottom-end", "top-end"];
    default:
      return ["bottom", "top", "right", "left"];
  }
}

/**
 * Creates Floating UI middlewares for popovers and tooltips.
 * - offset(offsetDistance): applies distance between reference and floating element first
 *   so flip and shift account for spacing.
 * - flip(...): dynamically tests fallback placements and changes spawn direction
 *   if the original direction would overflow the viewport boundary.
 * - shift(...): ensures the floating element never clips outside viewport padding margins.
 */
export function createPopoverMiddlewares(
  placement: Placement = "top",
  offsetDistance = 16,
  padding = 12
): Middleware[] {
  return [
    dom.offset(offsetDistance),
    dom.flip({
      fallbackPlacements: getFallbackPlacements(placement),
      fallbackStrategy: "bestFit",
      crossAxis: true,
      padding,
    }),
    dom.shift({
      padding,
      limiter: dom.limitShift(),
    }),
  ];
}

