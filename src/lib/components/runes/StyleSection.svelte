<script lang="ts">
  import KeystoneSlot from "./KeystoneSlot.svelte";
  import TierSlot from "./TierSlot.svelte";
  import type { RuneStyleTree } from "../../types/runes";
  import { getRuneTheme } from "../../utils/theme";

  let {
    tree = null,
    title = "Primary Tree",
    isSecondary = false,
    isEditing = false,
    onSelectPerk,
  }: {
    tree: RuneStyleTree | null;
    title: string;
    isSecondary?: boolean;
    isEditing?: boolean;
    onSelectPerk: (perkId: number, slotIndex: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(tree?.id));
</script>

{#if !tree}
  <article class="rounded-2xl border border-gray-800 bg-gray-950/80 p-4 h-full flex flex-col justify-center items-center">
    <h4 class="text-sm font-semibold text-gray-300">{title}</h4>
    <p class="mt-2 text-xs text-gray-500">No data</p>
  </article>
{:else if isSecondary}
  <article
    class="rounded-2xl border {theme.borderSubtle} bg-gray-950/80 p-4 flex flex-col backdrop-blur-sm shadow-md {theme.shadowGlow}"
  >
    <!-- Header Block -->
    <div class="mb-3.5 flex items-center gap-2.5 shrink-0">
      <div
        class="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border {theme.borderSubtle} {theme.bgCard} p-1 shadow-sm"
      >
        {#if tree.icon_url}
          <img src={tree.icon_url} alt={tree.name} class="h-6 w-6 object-contain" />
        {/if}
      </div>
      <div>
        <p class="text-[10px] font-bold uppercase tracking-wider text-gray-400">{title}</p>
        <h4 class="text-sm font-bold {theme.textAccent}">{tree.name}</h4>
      </div>
    </div>

    <!-- Slots -->
    <div class="space-y-3">
      {#each tree.slots as slot, index (index)}
        <TierSlot
          {slot}
          slotIndex={index}
          {theme}
          {isEditing}
          onSelectPerk={(perkId) => onSelectPerk(perkId, index)}
        />
      {/each}
    </div>
  </article>
{:else}
  <article
    class="rounded-2xl border {theme.borderSubtle} bg-gray-950/80 p-4 h-full flex flex-col backdrop-blur-sm shadow-md {theme.shadowGlow}"
  >
    <!-- Header Block -->
    <div class="mb-3.5 flex items-center gap-2.5 shrink-0">
      <div
        class="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border {theme.borderSubtle} {theme.bgCard} p-1 shadow-sm"
      >
        {#if tree.icon_url}
          <img src={tree.icon_url} alt={tree.name} class="h-6 w-6 object-contain" />
        {/if}
      </div>
      <div>
        <p class="text-[10px] font-bold uppercase tracking-wider text-gray-400">{title}</p>
        <h4 class="text-sm font-bold {theme.textAccent}">{tree.name}</h4>
      </div>
    </div>

    <!-- Keystone & Slots -->
    <div class="flex-1 flex flex-col justify-center my-auto">
      {#if tree.slots.length > 0}
        <KeystoneSlot
          slot={tree.slots[0]}
          {theme}
          {isEditing}
          onSelectPerk={(perkId) => onSelectPerk(perkId, 0)}
        />
      {/if}

      <div class="space-y-3">
        {#each tree.slots.slice(1) as slot, index (index + 1)}
          <TierSlot
            {slot}
            slotIndex={index + 1}
            {theme}
            {isEditing}
            onSelectPerk={(perkId) => onSelectPerk(perkId, index + 1)}
          />
        {/each}
      </div>
    </div>
  </article>
{/if}
