<script lang="ts">
  import { Popover } from "flowbite-svelte";
  import { parseGameDescription } from "../../utils/gameDescription";
  import { createPopoverMiddlewares } from "../../utils/popoverPlacement";
  import { slide } from "svelte/transition";
  import AugmentIcon from "./AugmentIcon.svelte";

  let {
    targetId,
    reference,
    name,
    iconUrl = "",
    description = "",
    rarity = "",
    itemClass = "",
    priceTotal = 0,
    categories = [],
    placement = "top",
    delay = 350,
    offset = 16,
  }: {
    targetId: string;
    reference?: string;
    name: string;
    iconUrl?: string;
    description?: string;
    rarity?: string;
    itemClass?: string;
    priceTotal?: number;
    categories?: string[];
    placement?: "top" | "bottom" | "left" | "right";
    delay?: number;
    offset?: number;
  } = $props();

  const formattedDescription = $derived(parseGameDescription(description));
  const normRarity = $derived(rarity.toLowerCase());
  const middlewares = $derived(createPopoverMiddlewares(placement, offset, 12));
</script>

{#if (description || name) && targetId}
  <Popover
    triggeredBy={`#${targetId}`}
    reference={reference ? reference : undefined}
    trigger="hover"
    triggerDelay={delay}
    transition={slide}
    offset={0}
    {middlewares}
    transitionParams={{ duration: 150 }}
    {placement}
    arrow
    class="z-50 w-80 max-w-[90vw] rounded-xl border {normRarity === 'prismatic' ? 'border-[#e78fff]/60 shadow-[0_0_16px_rgba(231,143,255,0.25)]' : normRarity === 'gold' ? 'border-[#e3c49f]/60 shadow-[0_0_16px_rgba(227,196,159,0.2)]' : normRarity === 'silver' ? 'border-[#a3b9bd]/60 shadow-[0_0_16px_rgba(163,185,189,0.15)]' : 'border-gray-700 shadow-2xl'} bg-gray-900/95 backdrop-blur-md p-3 text-gray-200 text-xs font-sans pointer-events-none"
    defaultClass=""
  >
    <!-- Header: Icon, Title, Badge, Gold -->
    <div class="flex items-center gap-2.5 pb-2.5 border-b border-gray-800">
      {#if iconUrl}
        {#if rarity}
          <AugmentIcon src={iconUrl} {name} {rarity} class="h-10 w-10" roundedClass="rounded-lg" />
        {:else}
          <div class="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-lg overflow-hidden bg-gray-950 border border-gray-700">
            <img src={iconUrl} alt={name} class="h-full w-full object-cover" />
          </div>
        {/if}
      {/if}
      <div class="min-w-0 flex-1">
        <div class="flex items-center justify-between gap-1.5">
          <h3 class="font-bold text-sm tracking-wide truncate {normRarity === 'prismatic' ? 'text-[#f2c2ff]' : normRarity === 'gold' ? 'text-[#e3c49f]' : normRarity === 'silver' ? 'text-[#a3b9bd]' : 'text-white'}">
            {name}
          </h3>
          {#if rarity}
            <span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {normRarity === 'silver' ? 'bg-[#a3b9bd]/15 text-[#a3b9bd] border border-[#a3b9bd]/40' : normRarity === 'gold' ? 'bg-[#e3c49f]/15 text-[#e3c49f] border border-[#e3c49f]/40' : 'bg-[#70008f]/30 text-[#f2c2ff] border border-[#e78fff]/50'}">
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
