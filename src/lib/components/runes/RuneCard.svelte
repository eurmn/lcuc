<script lang="ts">
  import TelemetryStats from "./TelemetryStats.svelte";
  import RunePopover from "./RunePopover.svelte";
  import type { EnrichedRuneOption } from "../../types/rune_builder";
  import type { RuneTheme } from "../../utils/theme";
    import clsx from "clsx";

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

  const cardId = $derived(`rune-card-${option.id}`);
</script>

<div class="relative h-full w-full">
  <button
    id={cardId}
    type="button"
    disabled={!isEditing}
    onclick={() => isEditing && onSelect(option.id)}
    class={clsx(
      "h-full w-full relative flex flex-col flex-1 min-w-27.5 gap-1.5 rounded-xl border-2",
      option.selected
        ? isEditing
          ? `${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm transition-all cursor-pointer overflow-hidden`
          : `${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm overflow-hidden select-none cursor-default`
        : isEditing
          ? "border-gray-700 bg-gray-900/80 p-2 sm:p-2.5 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer overflow-hidden"
          : "border-gray-800/60 bg-gray-900/40 p-2 sm:p-2.5 text-left opacity-35 overflow-hidden select-none cursor-default",
    )}
  >
    <!-- 1. Top Header: Perk Name -->
    <div class="flex items-center justify-between w-full min-w-0">
      <span
        class="text-xs font-semibold text-white truncate leading-tight"
        title={option.name}
      >
        {option.name}
      </span>
    </div>

    <!-- 2. Main Row: Perk Icon + Telemetry -->
    <div class="flex items-center gap-2 sm:gap-2.5 w-full min-w-0">
      <!-- Borderless Circular Perk Icon -->
      <div
        class="relative flex h-8 w-8 sm:h-9 sm:w-9 shrink-0 items-center justify-center rounded-full overflow-hidden p-0 bg-transparent"
      >
        {#if option.icon_url}
          <img
            src={option.icon_url}
            alt={option.name}
            class="h-full w-full object-cover"
          />
        {:else}
          <div class="h-full w-full rounded-full bg-gray-800"></div>
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

  <RunePopover
    targetId={cardId}
    name={option.name}
    iconUrl={option.icon_url}
    description={option.long_desc}
    {isEditing}
  />
</div>
