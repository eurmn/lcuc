<script lang="ts">
  import clsx from "clsx";
  import {
    formatGameCount,
    getPickRateOpacity,
    getWpaBarMetrics,
    getWpaText,
  } from "../../utils/wpa";

  let {
    occurrence = 0,
    pickRatePct = 0,
    wpa = 0,
    compact = false,
  }: {
    occurrence: number;
    pickRatePct: number;
    wpa: number;
    compact?: boolean;
  } = $props();

  const hasData = $derived(occurrence > 0 || pickRatePct > 0);
  const opacityVal = $derived(getPickRateOpacity(pickRatePct));
  const wpaMetrics = $derived(getWpaBarMetrics(wpa, hasData));
  const wpaText = $derived(getWpaText(wpa, hasData));
  const gameCountText = $derived(formatGameCount(occurrence, hasData));
</script>

<div class="flex flex-col min-w-0 flex-1 overflow-hidden">
  {#if hasData}
    <!-- 1. Games Count & White Pick Rate Bar -->
    <div class="flex flex-col gap-1 min-w-0">
      <div class="flex items-center justify-between">
        <span
          class="{compact
            ? 'text-[10px] sm:text-[11px]'
            : 'text-[11px]'} font-normal text-white leading-none truncate"
          style="opacity: {opacityVal}"
        >
          {gameCountText}
        </span>
      </div>
      <div
        class="h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner"
      >
        <div
          class="h-full rounded-full bg-white transition-all duration-250 ease-in-out"
          style="width: {Math.min(
            100,
            Math.max(4, pickRatePct),
          )}%; opacity: {opacityVal}"
        ></div>
      </div>
    </div>

    <!-- Divider between Pick Rate and WPA -->
    <div class="h-[1px] w-full bg-gray-800/80 my-1"></div>

    <!-- 2. WPA Text & Bi-Directional Center-Origin Bar -->
    <div class="flex flex-col gap-1 min-w-0">
      <div class="flex items-center justify-between">
        <span
          class="{compact
            ? 'text-[9px] sm:text-[10px]'
            : 'text-[10px]'} font-normal {wpaMetrics.textColorClass} leading-none truncate"
        >
          {wpaText}
        </span>
      </div>
      <div
        class="relative h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner"
      >
        <!-- Center divider (50% baseline) -->
        <div
          class="absolute left-1/2 top-0 bottom-0 w-[1px] -translate-x-1/2 bg-gray-700 z-10"
        ></div>
        <div
          class={clsx(
            "absolute top-0 bottom-0 transition-all duration-250 ease-in-out",
            wpa > 0
              ? "bg-green-500 rounded-r-full left-1/2"
              : "bg-red-500 rounded-l-full right-1/2",
          )}
          style="width: {wpaMetrics.barWidthPct}%"
        ></div>
      </div>
    </div>
  {:else}
    <div class="flex flex-col gap-1 py-1">
      <span class="text-[10px] font-normal text-gray-400">No Data</span>
      <div class="h-1 w-full rounded-full bg-gray-950"></div>
    </div>
  {/if}
</div>
