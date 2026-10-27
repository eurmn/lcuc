<script lang="ts">
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";

  let {
    searchedChampions = [],
    favoriteChampions = [],
    nonFavoriteChampions = [],
    selectedChampion = null,
    searchQuery = "",
    isRuneMismatch = false,
    activeRuneChampionId = null,
    showAllChampions = true,
    pinSelectedFirst = false,
    blocked = false,
    allowedChampionIds = null,
    onSelectChampion,
  }: {
    searchedChampions: ChampionSummary[];
    favoriteChampions: ChampionSummary[];
    nonFavoriteChampions: ChampionSummary[];
    selectedChampion: ChampionSummary | null;
    searchQuery: string;
    isRuneMismatch: boolean;
    activeRuneChampionId: number | null;
    showAllChampions?: boolean;
    pinSelectedFirst?: boolean;
    blocked?: boolean;
    allowedChampionIds?: Set<number> | null;
    onSelectChampion: (champ: ChampionSummary | null) => void;
  } = $props();

  let scrollContainer = $state<HTMLDivElement | null>(null);
  let canScrollLeft = $state<boolean>(false);
  let canScrollRight = $state<boolean>(false);

  export function updateScrollArrows() {
    if (!scrollContainer) return;
    canScrollLeft = scrollContainer.scrollLeft > 4;
    canScrollRight =
      scrollContainer.scrollLeft <
      scrollContainer.scrollWidth - scrollContainer.clientWidth - 4;
  }

  $effect(() => {
    if (favoriteChampions || nonFavoriteChampions || searchedChampions) {
      setTimeout(updateScrollArrows, 50);
    }
  });

  function scrollLeft() {
    scrollContainer?.scrollBy({ left: -200, behavior: "smooth" });
  }

  function scrollRight() {
    scrollContainer?.scrollBy({ left: 200, behavior: "smooth" });
  }

  function getButtonClass(champ: ChampionSummary, isSelected: boolean): string {
    if (isSelected) {
      return "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-purple-500 ring-2 ring-purple-400/80 shadow-[0_0_12px_rgba(168,85,247,0.6)] bg-gray-900 p-0 opacity-100 scale-105 cursor-pointer transition-all z-10";
    }
    if (blocked) {
      const allowed = allowedChampionIds ? allowedChampionIds.has(champ.id) : false;
      if (allowed) {
        return "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-cyan-400 ring-2 ring-cyan-500/50 bg-gray-900 p-0 shadow-[0_0_10px_rgba(34,211,238,0.5)] opacity-100 hover:scale-110 cursor-pointer transition-all z-10";
      }
      return "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border border-gray-800 bg-gray-950 p-0 opacity-25 grayscale cursor-not-allowed transition-all";
    }
    if (champ.is_favorite) {
      return "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-yellow-400 bg-gray-900 p-0 shadow-sm shadow-yellow-950/40 opacity-90 hover:opacity-100 hover:scale-105 cursor-pointer transition-all";
    }
    return "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border border-gray-700/80 bg-gray-900 p-0 opacity-70 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all";
  }

  function getButtonTitle(champ: ChampionSummary, isSelected: boolean): string {
    if (isSelected) {
      return `${champ.name} (Current Pick)`;
    }
    if (blocked) {
      const allowed = allowedChampionIds ? allowedChampionIds.has(champ.id) : false;
      if (allowed) {
        return `${champ.name} (Available in roster — click to swap in client)`;
      }
      return `${champ.name} (Pick can only be changed in League client)`;
    }
    return `${champ.name}${champ.is_favorite ? " (Favorite)" : ""}`;
  }

  function handleChampClick(champ: ChampionSummary, isSelected: boolean) {
    if (blocked) {
      const allowed = allowedChampionIds ? allowedChampionIds.has(champ.id) : false;
      if (!allowed || isSelected) {
        return;
      }
    }
    onSelectChampion(champ);
  }
</script>

<div class="relative flex-1 flex items-center min-w-0 overflow-hidden">
  <!-- Left Arrow Button -->
  {#if canScrollLeft}
    <button
      type="button"
      title="Scroll left"
      onclick={scrollLeft}
      class="absolute left-0 z-10 flex h-6 w-6 items-center justify-center rounded-full border border-gray-700 bg-gray-800/95 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <span class="icon-[heroicons--chevron-left] h-3 w-3"></span>
    </button>
  {/if}

  <!-- Circular Champions Carousel -->
  <div
    bind:this={scrollContainer}
    onscroll={updateScrollArrows}
    class="flex items-center gap-2 overflow-x-auto py-1.5 px-2 scrollbar-none scroll-smooth w-full select-none"
  >
    {#if searchQuery.trim()}
      <!-- Search Results -->
      {#each searchedChampions as champ (champ.id)}
        {@const isSelected = Boolean(selectedChampion && selectedChampion.id === champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={getButtonTitle(champ, isSelected)}
            onclick={() => handleChampClick(champ, isSelected)}
            class={getButtonClass(champ, isSelected)}
          >
            {#if champ.square_icon_url}
              <img
                src={champ.square_icon_url}
                alt={champ.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
              />
            {:else}
              <div class="h-full w-full rounded-full bg-gray-800"></div>
            {/if}
          </button>

          {#if isRuneMismatch && activeRuneChampionId && activeRuneChampionId === champ.id}
            <span
              class="pointer-events-none absolute top-0 right-0 z-20 flex h-2.5 w-2.5 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion"
            ></span>
          {/if}
        </div>
      {/each}
    {:else}
      <!-- Pinned Selected Champion (When pinSelectedFirst is enabled) -->
      {#if pinSelectedFirst && selectedChampion}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={getButtonTitle(selectedChampion, true)}
            onclick={() => handleChampClick(selectedChampion, true)}
            class={getButtonClass(selectedChampion, true)}
          >
            {#if selectedChampion.square_icon_url}
              <img
                src={selectedChampion.square_icon_url}
                alt={selectedChampion.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
              />
            {:else}
              <div class="h-full w-full rounded-full bg-gray-800"></div>
            {/if}
          </button>
        </div>
      {/if}

      <!-- Favorites List -->
      {#each (pinSelectedFirst && selectedChampion ? favoriteChampions.filter((c) => c.id !== selectedChampion.id) : favoriteChampions) as champ (champ.id)}
        {@const isSelected = Boolean(selectedChampion && selectedChampion.id === champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={getButtonTitle(champ, isSelected)}
            onclick={() => handleChampClick(champ, isSelected)}
            class={getButtonClass(champ, isSelected)}
          >
            {#if champ.square_icon_url}
              <img
                src={champ.square_icon_url}
                alt={champ.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
              />
            {:else}
              <div class="h-full w-full rounded-full bg-gray-800"></div>
            {/if}
          </button>

          {#if isRuneMismatch && activeRuneChampionId && activeRuneChampionId === champ.id}
            <span
              class="pointer-events-none absolute top-0 right-0 z-20 flex h-2.5 w-2.5 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion"
            ></span>
          {/if}
        </div>
      {/each}

      <!-- All Champions button -->
      {#if showAllChampions && selectedChampion !== null && !blocked}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title="All Champions (Viewing universal runes)"
            onclick={() => onSelectChampion(null)}
            class="group relative flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-gray-700/80 bg-gray-900 p-0 opacity-70 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all overflow-hidden"
          >
            <img
              src={ALL_CHAMPIONS_ICON_URL}
              alt="All Champions"
              class="opacity-80 w-[70%] h-[70%] object-contain transition-transform group-hover:scale-110"
            />
          </button>

          {#if isRuneMismatch && (!activeRuneChampionId || activeRuneChampionId === 0)}
            <span
              class="pointer-events-none absolute top-0 right-0 z-20 flex h-2.5 w-2.5 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion (All Champions)"
            ></span>
          {/if}
        </div>
      {/if}

      <!-- Non-Favorite Champions List -->
      {#each (pinSelectedFirst && selectedChampion ? nonFavoriteChampions.filter((c) => c.id !== selectedChampion.id) : nonFavoriteChampions) as champ (champ.id)}
        {@const isSelected = Boolean(selectedChampion && selectedChampion.id === champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={getButtonTitle(champ, isSelected)}
            onclick={() => handleChampClick(champ, isSelected)}
            class={getButtonClass(champ, isSelected)}
          >
            {#if champ.square_icon_url}
              <img
                src={champ.square_icon_url}
                alt={champ.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
              />
            {:else}
              <div class="h-full w-full rounded-full bg-gray-800"></div>
            {/if}
          </button>

          {#if isRuneMismatch && activeRuneChampionId && activeRuneChampionId === champ.id}
            <span
              class="pointer-events-none absolute top-0 right-0 z-20 flex h-2.5 w-2.5 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion"
            ></span>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Right Arrow Button -->
  {#if canScrollRight}
    <button
      type="button"
      title="Scroll right"
      onclick={scrollRight}
      class="absolute right-0 z-10 flex h-6 w-6 items-center justify-center rounded-full border border-gray-700 bg-gray-800/95 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <span class="icon-[heroicons--chevron-right] h-3 w-3"></span>
    </button>
  {/if}
</div>
