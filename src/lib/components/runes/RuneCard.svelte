<script lang="ts">
  import type { EnrichedRuneOption } from "../../types/rune_builder";
  import type { RuneTheme } from "../../utils/theme";
  import {
    formatGameCount,
    formatOccurrence,
    formatPickRate,
    getPickRateOpacity,
    getWpaBarMetrics,
    getWpaText,
  } from "../../utils/wpa";

  let {
    option,
    theme,
    isEditing = true,
    onSelect,
  }: {
    option: EnrichedRuneOption;
    theme: RuneTheme;
    isEditing?: boolean;
    onSelect: (perkId: number) => void;
  } = $props();

  const hasData = $derived(option.occurrence > 0 || option.pick_rate_pct > 0);
  const opacityVal = $derived(getPickRateOpacity(option.pick_rate_pct));
  const wpaMetrics = $derived(getWpaBarMetrics(option.wpa, hasData));
  const wpaText = $derived(getWpaText(option.wpa, hasData));
  const gameCountText = $derived(formatGameCount(option.occurrence, hasData));
</script>

<button
  type="button"
  disabled={!isEditing}
  title={hasData
    ? `${option.name} — ${formatPickRate(option.pick_rate_pct)} pick rate (${formatOccurrence(option.occurrence)} games), ${wpaText}`
    : `${option.name} — No statistical information available`}
  onclick={() => isEditing && onSelect(option.id)}
  class={option.selected
    ? (isEditing
      ? `relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 text-left shadow-sm transition-all cursor-pointer`
      : `relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 text-left shadow-sm select-none cursor-default`)
    : (isEditing
      ? "relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 border-gray-700 bg-gray-900/80 p-2 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer"
      : "relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 border-gray-800/60 bg-gray-900/40 p-2 text-left opacity-35 select-none cursor-default")}
>
  <!-- Borderless Circular Perk Icon (Image fills it all, no background/padding) -->
  <div class="relative flex h-9 w-9 shrink-0 items-center justify-center rounded-full overflow-hidden p-0 bg-transparent">
    {#if option.icon_url}
      <img src={option.icon_url} alt={option.name} class="h-full w-full object-cover" />
    {:else}
      <div class="h-full w-full rounded-full bg-gray-800"></div>
    {/if}

    {#if option.selected}
      <span
        class="absolute -top-1 -right-1 flex h-3.5 w-3.5 items-center justify-center rounded-full {theme.bgBadge} text-white shadow-sm"
      >
        <svg class="h-2 w-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
        </svg>
      </span>
    {/if}
  </div>

  <!-- Telemetry Column: Games Count + White Bar, Divider, WPA Text + Bi-Directional Bar -->
  <div class="flex flex-col min-w-0 flex-1">
    {#if hasData}
      <!-- 1. Games Count & White Pick Rate Bar -->
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-normal text-white leading-none" style="opacity: {opacityVal}">
            {gameCountText}
          </span>
        </div>
        <div class="h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
          <div
            class="h-full rounded-full bg-white transition-all"
            style="width: {Math.min(100, Math.max(4, option.pick_rate_pct))}%; opacity: {opacityVal}"
          ></div>
        </div>
      </div>

      <!-- Divider between Pick Rate and WPA -->
      <div class="h-[1px] w-full bg-gray-800/80 my-1"></div>

      <!-- 2. WPA Text & Bi-Directional Center-Origin Bar -->
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between">
          <span class="text-[10px] font-normal {wpaMetrics.textColorClass} leading-none">
            {wpaText}
          </span>
        </div>
        <div class="relative h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
          <!-- Center divider (50% baseline) -->
          <div class="absolute left-1/2 top-0 bottom-0 w-[1px] -translate-x-1/2 bg-gray-700 z-10"></div>
          {#if option.wpa > 0}
            <div
              class="absolute left-1/2 top-0 bottom-0 bg-emerald-400 rounded-r-full"
              style="width: {wpaMetrics.barWidthPct}%"
            ></div>
          {:else if option.wpa < 0}
            <div
              class="absolute right-1/2 top-0 bottom-0 bg-red-500 rounded-l-full"
              style="width: {wpaMetrics.barWidthPct}%"
            ></div>
          {/if}
        </div>
      </div>
    {:else}
      <!-- No Data Fallback State -->
      <div class="flex flex-col gap-1 py-1">
        <span class="text-[10px] font-normal text-gray-400">No Data</span>
        <div class="h-1 w-full rounded-full bg-gray-950"></div>
      </div>
    {/if}
  </div>
</button>
