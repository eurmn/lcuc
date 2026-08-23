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
      ? `relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 text-left shadow-sm transition-all cursor-pointer`
      : `relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 text-left shadow-sm select-none cursor-default`)
    : (isEditing
      ? "relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 border-gray-700 bg-gray-900/80 p-2 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer"
      : "relative flex flex-1 min-w-[110px] items-center gap-2.5 rounded-xl border-2 border-gray-800/60 bg-gray-900/40 p-2 text-left opacity-35 select-none cursor-default")}
>
  <!-- Borderless Circular Perk Icon -->
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

  <!-- Telemetry Column: Telemetry Pick Rate & WPA -->
  <TelemetryStats
    occurrence={option.occurrence}
    pickRatePct={option.pick_rate_pct}
    wpa={option.wpa}
  />
</button>
