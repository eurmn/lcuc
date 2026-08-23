<script lang="ts">
  import type { RuneSlot } from "../../types/runes";
  import type { RuneTheme } from "../../utils/theme";

  let {
    slot,
    slotIndex = 0,
    theme,
    isEditing = false,
    onSelectPerk,
  }: {
    slot: RuneSlot;
    slotIndex?: number;
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectPerk: (perkId: number) => void;
  } = $props();

  const selectedOpt = $derived(slot.options.find((o) => o.selected));
  const slotLabel = $derived(
    slot.label && !slot.label.toLowerCase().includes("slot")
      ? slot.label
      : `Slot ${slotIndex + 1}`
  );
</script>

<div class="rounded-xl border border-gray-800/80 bg-gray-900/90 p-3">
  <div class="mb-2 flex items-center justify-between">
    <p class="text-[11px] font-medium uppercase tracking-wide text-gray-400">
      {slotLabel}
    </p>
    {#if selectedOpt}
      <span class="text-xs font-bold {theme.textAccent}">
        {selectedOpt.name}
      </span>
    {/if}
  </div>

  <div class="flex flex-nowrap items-center justify-center gap-3">
    {#each slot.options as option (option.id)}
      {#if isEditing}
        <button
          type="button"
          title={option.name}
          onclick={() => onSelectPerk(option.id)}
          class={option.selected
            ? `group relative inline-flex h-11 w-11 items-center justify-center rounded-full border ${theme.borderPrimary} ${theme.bgCard} p-1 ring-1 ${theme.ringActive} shadow-sm ${theme.shadowGlow} scale-105 transition-all cursor-pointer`
            : `group relative inline-flex h-11 w-11 items-center justify-center rounded-full border border-gray-700/60 bg-gray-800/60 p-1 opacity-40 hover:opacity-100 ${theme.borderHover} hover:scale-105 hover:bg-gray-700 transition-all cursor-pointer`}
        >
          {#if option.icon_url}
            <img
              src={option.icon_url}
              alt={option.name}
              class="h-full w-full rounded-full object-cover"
            />
          {:else}
            <div class="h-full w-full rounded-full bg-gray-800"></div>
          {/if}

          {#if option.selected}
            <span
              class="absolute -top-1 -right-1 flex h-3.5 w-3.5 items-center justify-center rounded-full {theme.bgBadge} text-white shadow-sm"
            >
              <span class="icon-[heroicons--check] h-2.5 w-2.5"></span>
            </span>
          {/if}
        </button>
      {:else}
        <div
          title={option.name}
          class={option.selected
            ? `group inline-flex h-11 w-11 items-center justify-center rounded-full border p-1 transition-all select-none ring-1 ${theme.ringActive} border ${theme.borderPrimary} ${theme.bgCard} shadow-sm ${theme.shadowGlow}`
            : "group inline-flex h-11 w-11 items-center justify-center rounded-full border p-1 transition-all select-none border-gray-700/60 bg-gray-800/60 opacity-40"}
        >
          {#if option.icon_url}
            <img
              src={option.icon_url}
              alt={option.name}
              class="h-full w-full rounded-full object-cover"
            />
          {:else}
            <div class="h-full w-full rounded-full bg-gray-800"></div>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
</div>
