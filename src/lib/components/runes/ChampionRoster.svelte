<script lang="ts">
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_OPTION } from "../../types/champions";

  let {
    allChampions = [],
    savedRoster = [],
    selectedChampionId = 0,
    onSelectChampion,
  }: {
    allChampions: ChampionSummary[];
    savedRoster: ChampionSummary[];
    selectedChampionId: number;
    onSelectChampion: (champion: ChampionSummary) => void;
  } = $props();

  let searchQuery = $state("");
  let scrollContainer = $state<HTMLDivElement | null>(null);

  // When search query is entered or saved roster is empty, enter search/all mode
  const isSearchMode = $derived(searchQuery.trim().length > 0 || savedRoster.length === 0);

  const displayedChampions = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (q) {
      // Filter all champions matching query, prioritizing saved ones first
      const matches = allChampions.filter(
        (c) => c.name.toLowerCase().includes(q) || c.alias.toLowerCase().includes(q)
      );
      matches.sort((a, b) => {
        if (a.has_saved_runes && !b.has_saved_runes) return -1;
        if (!a.has_saved_runes && b.has_saved_runes) return 1;
        return a.name.localeCompare(b.name);
      });
      return matches;
    }

    if (savedRoster.length > 0) {
      return savedRoster;
    }

    return allChampions;
  });

  function scrollLeft() {
    scrollContainer?.scrollBy({ left: -220, behavior: "smooth" });
  }

  function scrollRight() {
    scrollContainer?.scrollBy({ left: 220, behavior: "smooth" });
  }

  function clearSearch() {
    searchQuery = "";
  }
</script>

<div class="flex flex-col gap-2 rounded-2xl border border-gray-800 bg-gray-950/70 p-3 shadow-md">
  <!-- Search Bar & Mode Header -->
  <div class="flex items-center justify-between gap-3 px-1">
    <div class="relative flex-1 max-w-xs">
      <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
        </svg>
      </div>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search champion..."
        class="w-full rounded-xl border border-gray-700 bg-gray-900/90 py-1.5 pl-9 pr-8 text-xs font-semibold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
      />
      {#if searchQuery}
        <button
          type="button"
          title="Clear search"
          aria-label="Clear search"
          onclick={clearSearch}
          class="absolute inset-y-0 right-0 flex items-center pr-2.5 text-gray-400 hover:text-white cursor-pointer"
        >
          <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
          </svg>
        </button>
      {/if}
    </div>

    <!-- Mode Badge -->
    <div class="flex items-center gap-2">
      {#if isSearchMode}
        <span class="inline-flex items-center gap-1.5 rounded-full border border-purple-500/30 bg-purple-500/10 px-2 py-0.5 text-[10px] font-semibold text-purple-300">
          <span class="h-1.5 w-1.5 rounded-full bg-purple-400 animate-pulse"></span>
          Browsing All ({displayedChampions.length})
        </span>
      {:else}
        <span class="inline-flex items-center gap-1.5 rounded-full border border-blue-500/30 bg-blue-500/10 px-2 py-0.5 text-[10px] font-semibold text-blue-300">
          Saved Roster ({savedRoster.length})
        </span>
      {/if}
    </div>
  </div>

  <!-- Horizontal Champion Roster with Arrow Controls -->
  <div class="relative flex items-center">
    <!-- Left Scroll Button -->
    <button
      type="button"
      title="Scroll left"
      onclick={scrollLeft}
      class="absolute left-0 z-10 flex h-8 w-8 -translate-x-1 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md backdrop-blur-sm hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
      </svg>
    </button>

    <!-- Scrollable Champion List -->
    <div
      bind:this={scrollContainer}
      class="flex items-center gap-2 overflow-x-auto py-1 px-8 scrollbar-none scroll-smooth w-full select-none"
    >
      <!-- "All Champions" Option -->
      <button
        type="button"
        onclick={() => onSelectChampion(ALL_CHAMPIONS_OPTION)}
        class={selectedChampionId === 0
          ? "flex shrink-0 items-center gap-2 rounded-xl border-2 border-purple-500 bg-purple-950/60 px-3 py-1.5 shadow-md shadow-purple-950/40 ring-1 ring-purple-400 cursor-pointer"
          : "flex shrink-0 items-center gap-2 rounded-xl border border-gray-800 bg-gray-900/80 px-3 py-1.5 opacity-60 hover:opacity-100 hover:border-gray-700 hover:bg-gray-800 transition-all cursor-pointer"}
      >
        <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-gray-800 text-purple-400 border border-purple-500/40">
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>
          </svg>
        </div>
        <span class="text-xs font-bold text-gray-200">All Champions</span>
      </button>

      <!-- Individual Champion Cards -->
      {#each displayedChampions as champ (champ.id)}
        <button
          type="button"
          title={champ.name}
          onclick={() => onSelectChampion(champ)}
          class={selectedChampionId === champ.id
            ? "group relative flex shrink-0 items-center gap-2 rounded-xl border-2 border-purple-500 bg-purple-950/60 p-1.5 pr-3 shadow-md shadow-purple-950/50 ring-1 ring-purple-400 cursor-pointer"
            : "group relative flex shrink-0 items-center gap-2 rounded-xl border border-gray-800 bg-gray-900/80 p-1.5 pr-3 opacity-60 hover:opacity-100 hover:border-gray-700 hover:bg-gray-800 transition-all cursor-pointer"}
        >
          <div class="relative h-7 w-7 shrink-0 overflow-hidden rounded-lg border border-gray-700/60 bg-gray-800">
            {#if champ.square_icon_url}
              <img src={champ.square_icon_url} alt={champ.name} class="h-full w-full object-cover" />
            {:else}
              <div class="h-full w-full bg-gray-700"></div>
            {/if}
          </div>

          <span class="text-xs font-bold text-gray-200 truncate max-w-[100px]">{champ.name}</span>

          {#if champ.has_saved_runes}
            <span
              title="Saved rune pages exist"
              class="flex h-2 w-2 rounded-full bg-emerald-400 shadow-sm shadow-emerald-400 shrink-0"
            ></span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Right Scroll Button -->
    <button
      type="button"
      title="Scroll right"
      onclick={scrollRight}
      class="absolute right-0 z-10 flex h-8 w-8 translate-x-1 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md backdrop-blur-sm hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
      </svg>
    </button>
  </div>
</div>
