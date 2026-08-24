<script lang="ts">
  import TelemetryStats from "./TelemetryStats.svelte";
  import type { EnrichedRuneOption } from "../../types/rune_builder";
  import type { RuneTheme } from "../../utils/theme";
  import { formatOccurrence, formatPickRate, getWpaText } from "../../utils/wpa";

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
  const wpaText = $derived(getWpaText(option.wpa, hasData));
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
      ? `relative flex flex-col flex-1 min-w-[110px] gap-1.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm transition-all cursor-pointer overflow-hidden`
      : `relative flex flex-col flex-1 min-w-[110px] gap-1.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm overflow-hidden select-none cursor-default`)
    : (isEditing
      ? "relative flex flex-col flex-1 min-w-[110px] gap-1.5 rounded-xl border-2 border-gray-700 bg-gray-900/80 p-2 sm:p-2.5 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer overflow-hidden"
      : "relative flex flex-col flex-1 min-w-[110px] gap-1.5 rounded-xl border-2 border-gray-800/60 bg-gray-900/40 p-2 sm:p-2.5 text-left opacity-35 overflow-hidden select-none cursor-default")}
>
  <!-- 1. Top Header: Perk Name -->
  <div class="flex items-center justify-between w-full min-w-0">
    <span class="text-xs font-semibold text-white truncate leading-tight" title={option.name}>
      {option.name}
    </span>
  </div>

  <!-- 2. Main Row: Perk Icon + Telemetry -->
  <div class="flex items-center gap-2 sm:gap-2.5 w-full min-w-0">
    <!-- Borderless Circular Perk Icon -->
    <div class="relative flex h-8 w-8 sm:h-9 sm:w-9 shrink-0 items-center justify-center rounded-full overflow-hidden p-0 bg-transparent">
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

    <!-- Telemetry Column: Telemetry Pick Rate & WPA -->
    <TelemetryStats
      occurrence={option.occurrence}
      pickRatePct={option.pick_rate_pct}
      wpa={option.wpa}
    />
  </div>
</button>
