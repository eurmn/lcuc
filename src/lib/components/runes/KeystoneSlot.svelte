<script lang="ts">
  import type { RuneSlot } from "../../types/runes";
  import type { RuneTheme } from "../../utils/theme";

  let {
    slot,
    theme,
    isEditing = false,
    onSelectPerk,
  }: {
    slot: RuneSlot;
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectPerk: (perkId: number) => void;
  } = $props();

  const selectedOpt = $derived(slot.options.find((o) => o.selected));
</script>

<div class="mb-6">
  <div class="mb-2 flex items-center justify-between px-1">
    <p class="text-[11px] font-medium uppercase tracking-wide text-gray-400">Keystone</p>
    {#if selectedOpt}
      <span class="text-xs font-bold {theme.textAccent}">
        {selectedOpt.name}
      </span>
    {/if}
  </div>

  <div class="flex flex-nowrap items-center justify-center gap-2.5 sm:gap-3.5 md:gap-4 py-2 pb-1 w-full min-w-0">
    {#each slot.options as option (option.id)}
      {#if isEditing}
        <button
          type="button"
          title={option.name}
          onclick={() => onSelectPerk(option.id)}
          class={option.selected
            ? `group relative inline-flex h-16 w-16 sm:h-18 sm:w-18 md:h-20 md:w-20 shrink-0 items-center justify-center rounded-full border-2 ${theme.borderPrimary} ${theme.bgCard} p-2 ring-4 ${theme.ringFocus} shadow-xl ${theme.shadowGlow} scale-105 transition-all cursor-pointer`
            : `group relative inline-flex h-16 w-16 sm:h-18 sm:w-18 md:h-20 md:w-20 shrink-0 items-center justify-center rounded-full border border-gray-700/60 bg-gray-800/60 p-2 opacity-40 hover:opacity-100 ${theme.borderHover} hover:scale-105 hover:bg-gray-700 transition-all cursor-pointer`}
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
              class="absolute -top-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full {theme.bgBadge} text-white shadow-sm"
            >
              <span class="icon-[heroicons--check] h-3 w-3"></span>
            </span>
          {/if}
        </button>
      {:else}
        <div
          title={option.name}
          class={option.selected
            ? `group inline-flex h-16 w-16 sm:h-18 sm:w-18 md:h-20 md:w-20 shrink-0 items-center justify-center rounded-full border p-2 transition-all select-none ring-4 ${theme.ringFocus} border-2 ${theme.borderPrimary} ${theme.bgCard} shadow-xl ${theme.shadowGlow} scale-105`
            : "group inline-flex h-16 w-16 sm:h-18 sm:w-18 md:h-20 md:w-20 shrink-0 items-center justify-center rounded-full border p-2 transition-all select-none border-gray-700/60 bg-gray-800/40 opacity-40"}
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
