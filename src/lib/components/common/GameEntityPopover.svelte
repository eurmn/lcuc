<script lang="ts">
  import { Popover } from "flowbite-svelte";
  import { parseGameDescription } from "../../utils/gameDescription";
  import { slide } from "svelte/transition";

  let {
    targetId,
    name,
    iconUrl = "",
    description = "",
    rarity = "",
    itemClass = "",
    priceTotal = 0,
    categories = [],
    placement = "top",
    delay = 350,
  }: {
    targetId: string;
    name: string;
    iconUrl?: string;
    description?: string;
    rarity?: string;
    itemClass?: string;
    priceTotal?: number;
    categories?: string[];
    placement?: "top" | "bottom" | "left" | "right";
    delay?: number;
  } = $props();

  const formattedDescription = $derived(parseGameDescription(description));
  const normRarity = $derived(rarity.toLowerCase());
</script>

{#if (description || name) && targetId}
  <Popover
    triggeredBy={`#${targetId}`}
    trigger="hover"
    triggerDelay={delay}
    transition={slide}
    offset={16}
    transitionParams={{ duration: 150 }}
    {placement}
    arrow
    class="z-50 w-80 max-w-[90vw] rounded-xl border {normRarity === 'prismatic' ? 'border-fuchsia-500/60 shadow-[0_0_16px_rgba(217,70,239,0.25)]' : normRarity === 'gold' ? 'border-yellow-500/60 shadow-[0_0_16px_rgba(234,179,8,0.2)]' : normRarity === 'silver' ? 'border-slate-500/60 shadow-[0_0_16px_rgba(148,163,184,0.15)]' : 'border-gray-700 shadow-2xl'} bg-gray-900/95 backdrop-blur-md p-3 text-gray-200 text-xs font-sans pointer-events-none"
    defaultClass=""
  >
    <!-- Header: Icon, Title, Badge, Gold -->
    <div class="flex items-center gap-2.5 pb-2.5 border-b border-gray-800">
      {#if iconUrl}
        <div class="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-lg overflow-hidden bg-gray-950 border {normRarity === 'prismatic' ? 'border-fuchsia-500' : normRarity === 'gold' ? 'border-yellow-500' : normRarity === 'silver' ? 'border-slate-400' : 'border-gray-700'}">
          <img src={iconUrl} alt={name} class="h-full w-full object-cover" />
        </div>
      {/if}
      <div class="min-w-0 flex-1">
        <div class="flex items-center justify-between gap-1.5">
          <h3 class="font-bold text-sm text-white tracking-wide truncate">
            {name}
          </h3>
          {#if rarity}
            <span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {normRarity === 'silver' ? 'bg-slate-700/80 text-slate-200 border border-slate-600/60' : normRarity === 'gold' ? 'bg-yellow-950/80 text-yellow-300 border border-yellow-600/60' : 'bg-fuchsia-950/80 text-fuchsia-300 border border-fuchsia-500/60'}">
              {rarity}
            </span>
          {/if}
        </div>
        <div class="flex flex-wrap items-center gap-2 mt-0.5 text-[11px] text-gray-400">
          {#if priceTotal > 0}
            <span class="inline-flex items-center gap-1 font-semibold text-yellow-400">
              <img src="https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/gameplay/goldcoins.png" alt="gold" class="h-3 w-3 object-contain inline" />
              {priceTotal.toLocaleString()}
            </span>
          {/if}
          {#if itemClass}
            <span class="capitalize text-gray-400">{itemClass}</span>
          {/if}
          {#if categories.length > 0}
            <span class="text-gray-500 truncate">{categories.slice(0, 3).join(", ")}</span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Body: Formatted Rich Description -->
    {#if description}
      <div class="mt-2 text-xs text-gray-300 leading-relaxed select-text space-y-1">
        {@html formattedDescription}
      </div>
    {:else}
      <p class="mt-2 text-xs text-gray-500 italic">No description available.</p>
    {/if}
  </Popover>
{/if}
