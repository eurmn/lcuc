<script lang="ts">
  import type { RuneSlot } from "../../types/runes";

  let {
    statMods = [],
    isEditing = false,
    onSelectPerk,
  }: {
    statMods: RuneSlot[];
    isEditing?: boolean;
    onSelectPerk: (perkId: number, slotIndex: number) => void;
  } = $props();
</script>

<article class="rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-sm">
  <div class="mb-3 flex items-center gap-2 shrink-0">
    <h4 class="text-sm font-semibold text-gray-300">Stat Mods</h4>
  </div>

  <div class="space-y-2">
    {#each statMods as slot, rowIndex (rowIndex)}
      <div class="flex items-center justify-center gap-3 py-1">
        {#each slot.options as option (option.id)}
          {#if isEditing}
            <button
              type="button"
              title={option.name}
              onclick={() => onSelectPerk(option.id, rowIndex)}
              class={option.selected
                ? "group relative inline-flex h-8 w-8 items-center justify-center rounded-full border border-purple-400 bg-purple-950/60 p-1 ring-1 ring-purple-400/80 shadow-sm shadow-purple-950/50 scale-105 transition-all cursor-pointer"
                : "group relative inline-flex h-8 w-8 items-center justify-center rounded-full border border-transparent bg-gray-800/80 p-1 opacity-40 hover:opacity-100 hover:border-purple-400 hover:scale-105 hover:bg-gray-700 transition-all cursor-pointer"}
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
            </button>
          {:else}
            <div
              title={option.name}
              class={option.selected
                ? "inline-flex h-8 w-8 items-center justify-center rounded-full border p-1 transition-all select-none ring-1 ring-purple-400/80 border-purple-400 bg-purple-950/40 shadow-sm"
                : "inline-flex h-8 w-8 items-center justify-center rounded-full border p-1 transition-all select-none border-transparent bg-gray-800/40 opacity-40"}
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
    {/each}
  </div>
</article>
