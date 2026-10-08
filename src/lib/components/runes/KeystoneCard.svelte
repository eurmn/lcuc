<script lang="ts">
  import TelemetryStats from "./TelemetryStats.svelte";
  import RunePopover from "./RunePopover.svelte";
  import type { EnrichedKeystoneOption } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";
    import clsx from "clsx";

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
  const cardId = $derived(`keystone-card-${keystone.id}`);
</script>

<div class="relative h-full w-full">
  <button
    id={cardId}
    type="button"
    disabled={!isEditing}
    onclick={() => isEditing && onSelect(keystone.id)}
    class={clsx("h-full w-full",
      keystone.selected
        ? isEditing
          ? `relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm transition-all cursor-pointer overflow-hidden`
          : `relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 ${theme.borderPrimary} bg-gray-900 p-2 sm:p-2.5 text-left shadow-sm overflow-hidden select-none cursor-default`
        : isEditing
          ? "relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 border-gray-700 bg-gray-900/80 p-2 sm:p-2.5 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 transition-all cursor-pointer overflow-hidden"
        : "relative flex flex-col min-w-0 w-full flex-1 gap-1.5 rounded-2xl border-2 border-gray-800/60 bg-gray-900/40 p-2 sm:p-2.5 text-left opacity-35 overflow-hidden select-none cursor-default"
    )}>
    <!-- 1. Top Header: Keystone Name -->
    <div class="flex items-center justify-between w-full min-w-0">
      <span class="text-xs font-semibold text-white truncate leading-tight"
        >{keystone.name}</span
      >
    </div>

    <!-- 2. Main Row: Keystone Icon on left, Stats & Details on right -->
    <div
      class="flex items-center justify-center gap-2 sm:gap-2.5 w-full min-w-0"
    >
      <!-- Borderless Circular Keystone Icon -->
      <div
        class="relative flex h-12 w-12 sm:h-16 sm:w-16 shrink-0 items-center justify-center rounded-full overflow-hidden p-0 bg-transparent"
      >
        {#if keystone.icon_url}
          <img
            src={keystone.icon_url}
            alt={keystone.name}
            class="h-full w-full object-cover"
          />
        {:else}
          <div class="h-full w-full rounded-full bg-gray-800"></div>
        {/if}
      </div>
    </div>

    <!-- 3. Details Column: Telemetry Pick Rate & WPA -->
    <TelemetryStats
      occurrence={keystone.occurrence}
      pickRatePct={keystone.pick_rate_pct}
      wpa={keystone.wpa}
      compact={true}
    />
  </button>

  <RunePopover
    targetId={cardId}
    name={keystone.name}
    iconUrl={keystone.icon_url}
    description={keystone.long_desc}
    {isEditing}
  />
</div>
