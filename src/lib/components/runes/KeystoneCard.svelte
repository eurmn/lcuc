<script lang="ts">
  import type { EnrichedKeystoneOption } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";
  import {
    formatGameCount,
    formatOccurrence,
    formatPickRate,
    getPickRateOpacity,
    getWpaBarMetrics,
    getWpaText,
  } from "../../utils/wpa";

  let {
    keystone,
    isEditing = true,
    onSelect,
  }: {
    keystone: EnrichedKeystoneOption;
    isEditing?: boolean;
    onSelect: (keystoneId: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(keystone.style_id));
  const hasData = $derived(keystone.occurrence > 0 || keystone.pick_rate_pct > 0);
  const opacityVal = $derived(getPickRateOpacity(keystone.pick_rate_pct));
  const wpaMetrics = $derived(getWpaBarMetrics(keystone.wpa, hasData));
  const wpaText = $derived(getWpaText(keystone.wpa, hasData));
  const gameCountText = $derived(formatGameCount(keystone.occurrence, hasData));
</script>

<button
  type="button"
  disabled={!isEditing}
  title={hasData
    ? `${keystone.name} (${keystone.style_name}) — ${formatPickRate(keystone.pick_rate_pct)} pick rate (${formatOccurrence(keystone.occurrence)} games), ${wpaText}`
    : `${keystone.name} — No statistical information available`}
  onclick={() => isEditing && onSelect(keystone.id)}
  class={keystone.selected
    ? (isEditing
      ? `relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm transition-all cursor-pointer overflow-hidden`
      : `relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm overflow-hidden select-none cursor-default`)
    : (isEditing
      ? "relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 border-gray-700 bg-gray-900/80 p-2 sm:p-2.5 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer overflow-hidden"
      : "relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 border-gray-800/60 bg-gray-900/40 p-2 sm:p-2.5 text-left opacity-35 overflow-hidden select-none cursor-default")}
>
  <!-- 1. Top Header: Keystone Name -->
  <div class="flex items-center justify-between w-full min-w-0">
    <span class="text-xs font-semibold text-white truncate leading-tight">{keystone.name}</span>
  </div>

  <!-- 2. Main Row: Keystone Icon on left, Stats & Details on right -->
  <div class="flex items-center gap-2 sm:gap-2.5 w-full min-w-0">
    <!-- Borderless Circular Keystone Icon -->
    <div
      class="relative flex h-10 w-10 sm:h-11 sm:w-11 shrink-0 items-center justify-center rounded-full overflow-hidden p-0 bg-transparent"
    >
      {#if keystone.icon_url}
        <img src={keystone.icon_url} alt={keystone.name} class="h-full w-full object-cover" />
      {:else}
        <div class="h-full w-full rounded-full bg-gray-800"></div>
      {/if}

      {#if keystone.selected}
        <span
          class="absolute -top-1 -right-1 flex h-3.5 w-3.5 items-center justify-center rounded-full {theme.bgBadge} text-white shadow-sm"
        >
          <svg class="h-2.5 w-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
          </svg>
        </span>
      {/if}
    </div>

    <!-- Details Column: Game Count + White Bar, Divider, WPA Text + Bi-Directional Bar -->
    <div class="flex flex-col min-w-0 flex-1 overflow-hidden">
      {#if hasData}
        <!-- 1. Games Count & White Pick Rate Bar -->
        <div class="flex flex-col gap-1 min-w-0">
          <div class="flex items-center justify-between">
            <span class="text-[10px] sm:text-[11px] font-normal text-white leading-none truncate" style="opacity: {opacityVal}">
              {gameCountText}
            </span>
          </div>
          <div class="h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
            <div
              class="h-full rounded-full bg-white transition-all"
              style="width: {Math.min(100, Math.max(4, keystone.pick_rate_pct))}%; opacity: {opacityVal}"
            ></div>
          </div>
        </div>

        <!-- Divider between Pick Rate and WPA -->
        <div class="h-[1px] w-full bg-gray-800/80 my-1"></div>

        <!-- 2. WPA Text & Bi-Directional Center-Origin Bar -->
        <div class="flex flex-col gap-1 min-w-0">
          <div class="flex items-center justify-between">
            <span class="text-[9px] sm:text-[10px] font-normal {wpaMetrics.textColorClass} leading-none truncate">
              {wpaText}
            </span>
          </div>
          <div class="relative h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
            <!-- Center divider (50% baseline) -->
            <div class="absolute left-1/2 top-0 bottom-0 w-[1px] -translate-x-1/2 bg-gray-700 z-10"></div>
            {#if keystone.wpa > 0}
              <div
                class="absolute left-1/2 top-0 bottom-0 bg-emerald-400 rounded-r-full"
                style="width: {wpaMetrics.barWidthPct}%"
              ></div>
            {:else if keystone.wpa < 0}
              <div
                class="absolute right-1/2 top-0 bottom-0 bg-red-500 rounded-l-full"
                style="width: {wpaMetrics.barWidthPct}%"
              ></div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="flex flex-col gap-1 py-1">
          <span class="text-[10px] font-normal text-gray-400">No Data</span>
          <div class="h-1 w-full rounded-full bg-gray-950"></div>
        </div>
      {/if}
    </div>
  </div>
</button>
