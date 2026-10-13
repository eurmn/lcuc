<script lang="ts">
  import { Popover } from "flowbite-svelte";
  import { parseRuneDescription } from "../../utils/runeDescription";
  import { createPopoverMiddlewares } from "../../utils/popoverPlacement";
  import { slide } from "svelte/transition";

  let {
    targetId,
    reference,
    name,
    iconUrl = "",
    description = "",
    isEditing = false,
    placement = "top",
    offset = 20,
  }: {
    targetId: string;
    reference?: string;
    name: string;
    iconUrl?: string;
    description?: string;
    isEditing?: boolean;
    placement?: "top" | "bottom" | "left" | "right";
    offset?: number;
  } = $props();

  const formattedDescription = $derived(parseRuneDescription(description));
  const delay = $derived(isEditing ? 1000 : 500);
  const middlewares = $derived(createPopoverMiddlewares(placement, offset, 12));
</script>

{#if description && targetId}
  <Popover
    triggeredBy={`#${targetId}`}
    reference={reference ? reference : undefined}
    trigger="hover"
    triggerDelay={delay}
    transition={slide}
    offset={0}
    {middlewares}
    transitionParams={{ duration: 200 }}
    {placement}
    arrow
    class="z-50 w-72 sm:w-80 rounded-xl border border-gray-700 bg-gray-900/95 backdrop-blur-md p-1 text-gray-200 text-xs font-sans"
    defaultClass=""
  >
    <!-- 1. Header: Rune Icon + Name -->
    <div class="flex items-center gap-2 pb-2 border-b border-gray-800">
      <div
        class="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-full overflow-hidden bg-gray-950 p-0 border border-gray-700/60 shadow-inner"
      >
        {#if iconUrl}
          <img src={iconUrl} alt={name} class="h-full w-full object-cover" />
        {:else}
          <div class="h-full w-full bg-gray-800"></div>
        {/if}
      </div>
      <h3 class="font-bold text-sm text-white tracking-wide truncate">
        {name}
      </h3>
    </div>

    <!-- 2. Body: Formatted Rich Description -->
    <div
      class="text-xs text-gray-300 leading-relaxed rune-description select-text"
    >
      {@html formattedDescription}
    </div>
  </Popover>
{/if}
