/**
 * Utilities for formatting Win Percentage Added (WPA) metrics and dynamic color scales.
 */

export interface WpaBarMetrics {
  isPositive: boolean;
  barWidthPct: number;
  colorClass: string;
  textColorClass: string;
}

/**
 * Returns bar metrics for the bi-directional center-origin WPA bar.
 * Baseline is 50%. Maximum clamp is -3.0% and +3.0%.
 */
export function getWpaBarMetrics(wpa: number, hasData: boolean = true): WpaBarMetrics {
  if (!hasData) {
    return {
      isPositive: true,
      barWidthPct: 0,
      colorClass: "bg-gray-700",
      textColorClass: "text-gray-400",
    };
  }

  const clamped = Math.max(-3.0, Math.min(3.0, wpa));
  const isPositive = clamped >= 0;
  const barWidthPct = (Math.abs(clamped) / 3.0) * 50;

  let colorClass = "bg-gray-500";
  let textColorClass = "text-gray-300";

  if (wpa > 0.05) {
    colorClass = "bg-emerald-400";
    textColorClass = "text-emerald-400";
  } else if (wpa < -0.05) {
    colorClass = "bg-red-500";
    textColorClass = "text-red-400";
  }

  return {
    isPositive,
    barWidthPct,
    colorClass,
    textColorClass,
  };
}

/**
 * Calculates white text & bar opacity scaled by pick rate percentage.
 */
export function getPickRateOpacity(pickRate: number): number {
  if (!pickRate || pickRate <= 0) return 0.35;
  return Math.max(0.4, Math.min(1.0, 0.4 + (pickRate / 100) * 0.6));
}

/**
 * Formats WPA text as clean regular text showing only +/- X with two decimal places (e.g. "+1.42", "-0.85", "No Data").
 */
export function getWpaText(wpa: number, hasData: boolean = true): string {
  if (!hasData) return "No Data";
  const sign = wpa > 0 ? "+" : "";
  return `${sign}${wpa.toFixed(2)}`;
}

/**
 * Formats full games count string as a full number (e.g. "142,857" or "142857").
 */
export function formatGameCount(occurrence: number, hasData: boolean = true): string {
  if (!hasData || occurrence <= 0) return "No Data";
  return occurrence.toLocaleString();
}

/**
 * Formats a raw occurrence count into a human-readable abbreviation (e.g. 324.7k).
 */
export function formatOccurrence(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return count.toString();
}

/**
 * Formats pick rate percentage.
 */
export function formatPickRate(rate: number): string {
  return `${rate.toFixed(1)}%`;
}
