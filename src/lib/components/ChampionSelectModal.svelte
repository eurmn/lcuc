<script lang="ts">
  import { onMount } from "svelte";
  import type { ChampionSummary } from "../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../types/champions";
  import { getChampions } from "../api/champions";
  import { logger } from "../utils/logger";

  let {
    isOpen = false,
    title = "Select Champion",
    currentChampionId = null,
    onSelect,
    onClose,
  }: {
    isOpen: boolean;
    title?: string;
    currentChampionId?: number | null;
    onSelect: (champion: ChampionSummary | null) => void;
    onClose: () => void;
  } = $props();

  let allChampions = $state<ChampionSummary[]>([]);
  let searchQuery = $state("");
  let searchInputRef = $state<HTMLInputElement | null>(null);

  onMount(async () => {
    try {
      allChampions = await getChampions();
    } catch (err) {
      logger.error("frontend.champion-select", "Failed to load champions", err);
    }
  });

  $effect(() => {
    if (isOpen) {
      searchQuery = "";
      setTimeout(() => searchInputRef?.focus(), 50);
    }
  });

  const filteredChampions = $derived.by(() => {
    const valid = allChampions.filter(
      (c) => !c.id.toString().startsWith("600") && c.id > 0
    );
    const q = searchQuery.trim().toLowerCase();
    if (!q) {
      return [...valid].sort((a, b) => {
        const aFav = a.is_favorite ? 1 : 0;
        const bFav = b.is_favorite ? 1 : 0;
        if (aFav !== bFav) return bFav - aFav;

        const aSaved = a.has_saved_runes ? 1 : 0;
        const bSaved = b.has_saved_runes ? 1 : 0;
        if (aSaved !== bSaved) return bSaved - aSaved;

        return a.name.localeCompare(b.name);
      });
    }
    return valid.filter(
      (c) =>
        c.name.toLowerCase().includes(q) || c.alias.toLowerCase().includes(q)
    );
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- Flowbite Modal Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 animate-fade-in"
    onclick={onClose}
    role="dialog"
    aria-modal="true"
  >
    <!-- Modal Container -->
    <div
      class="relative w-full max-w-lg max-h-[85vh] rounded-2xl bg-gray-900 border border-gray-700 shadow-2xl shadow-black/80 flex flex-col overflow-hidden animate-scale-up"
      role="document"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Flowbite Modal Header -->
      <div class="flex items-center justify-between p-4 border-b border-gray-800">
        <div class="flex items-center gap-2">
          <span class="flex h-2.5 w-2.5 rounded-full bg-purple-500 shadow-sm shadow-purple-500"></span>
          <h3 class="text-sm font-bold uppercase tracking-wider text-white">
            {title}
          </h3>
        </div>
        <button
          type="button"
          title="Close modal"
          aria-label="Close modal"
          onclick={onClose}
          class="text-gray-400 bg-transparent hover:bg-gray-800 hover:text-white rounded-xl text-sm w-8 h-8 inline-flex justify-center items-center cursor-pointer transition-colors"
        >
          <span class="icon-[heroicons--x-mark] h-4 w-4"></span>
        </button>
      </div>

      <!-- Flowbite Modal Search & Quick Actions -->
      <div class="p-4 border-b border-gray-800/80 flex flex-col gap-3 bg-gray-950/40">
        <!-- Search Input -->
        <div class="relative">
          <div class="absolute inset-y-0 start-0 flex items-center ps-3 pointer-events-none text-gray-400">
            <span class="icon-[heroicons--magnifying-glass] h-4 w-4"></span>
          </div>
          <input
            bind:this={searchInputRef}
            type="text"
            bind:value={searchQuery}
            placeholder="Search champion..."
            class="block w-full p-2 ps-9 pe-8 text-xs font-semibold text-white border border-gray-700 rounded-xl bg-gray-900 placeholder:text-gray-500 focus:ring-1 focus:ring-purple-500 focus:border-purple-500 focus:outline-none"
          />
          {#if searchQuery}
            <button
              type="button"
              title="Clear search"
              aria-label="Clear search"
              onclick={() => (searchQuery = "")}
              class="absolute inset-y-0 end-0 flex items-center pe-3 text-gray-400 hover:text-white cursor-pointer"
            >
              <span class="icon-[heroicons--x-mark] h-3.5 w-3.5"></span>
            </button>
          {/if}
        </div>

        <!-- "All Champions" Option Button -->
        <button
          type="button"
          onclick={() => {
            onSelect(null);
            onClose();
          }}
          class={currentChampionId === null || currentChampionId === 0
            ? "flex items-center gap-3 p-2 rounded-xl border-2 border-purple-500 bg-purple-950/30 shadow-md cursor-pointer transition-all"
            : "flex items-center gap-3 p-2 rounded-xl border border-gray-800 bg-gray-900/80 hover:border-gray-600 hover:bg-gray-800 cursor-pointer transition-all"}
        >
          <div class="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-full border border-gray-700 bg-gray-900 overflow-hidden">
            <img
              src={ALL_CHAMPIONS_ICON_URL}
              alt="All Champions"
              class="opacity-80 w-[70%] h-[70%] object-contain"
            />
          </div>
          <div class="flex-1 text-left min-w-0">
            <p class="text-xs font-bold text-white">All Champions (Universal)</p>
            <p class="text-[10px] text-gray-400">Available across all champions</p>
          </div>
          {#if currentChampionId === null || currentChampionId === 0}
            <span class="text-[10px] font-bold text-purple-400 bg-purple-500/20 px-2 py-0.5 rounded-full border border-purple-500/40">
              Selected
            </span>
          {/if}
        </button>
      </div>

      <!-- Flowbite Modal Body: Champion Grid -->
      <div class="p-4 overflow-y-auto flex-1 max-h-80 select-none">
        <div class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-2.5">
          {#each filteredChampions as champ (champ.id)}
            {@const isSelected = currentChampionId === champ.id}
            <button
              type="button"
              onclick={() => {
                onSelect(champ);
                onClose();
              }}
              class={isSelected
                ? "flex flex-col items-center gap-1.5 p-2 rounded-xl border-2 border-purple-500 bg-purple-950/40 shadow-md scale-105 cursor-pointer transition-all"
                : champ.is_favorite
                ? "flex flex-col items-center gap-1.5 p-2 rounded-xl border-2 border-yellow-400 bg-gray-950/60 hover:border-yellow-300 hover:bg-gray-800/80 hover:scale-105 hover:shadow-md cursor-pointer transition-all group"
                : "flex flex-col items-center gap-1.5 p-2 rounded-xl border border-gray-800/90 bg-gray-950/60 hover:border-purple-500 hover:bg-gray-800/80 hover:scale-105 hover:shadow-md cursor-pointer transition-all group"}
            >
              <div class="relative flex h-10 w-10 shrink-0 overflow-hidden rounded-full border border-gray-700/80 bg-gray-900 p-0 shadow-inner">
                {#if champ.square_icon_url}
                  <img
                    src={champ.square_icon_url}
                    alt={champ.name}
                    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
                  />
                {:else}
                  <div class="h-full w-full rounded-full bg-gray-800"></div>
                {/if}
              </div>

              <span class="text-[11px] font-semibold text-gray-300 group-hover:text-white truncate max-w-full text-center">
                {champ.name}
              </span>
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}
