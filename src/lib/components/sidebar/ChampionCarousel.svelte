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
    onSelectChampion,
  }: {
    searchedChampions: ChampionSummary[];
    favoriteChampions: ChampionSummary[];
    nonFavoriteChampions: ChampionSummary[];
    selectedChampion: ChampionSummary | null;
    searchQuery: string;
    isRuneMismatch: boolean;
    activeRuneChampionId: number | null;
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
    scrollContainer?.scrollBy({ left: -160, behavior: "smooth" });
  }

  function scrollRight() {
    scrollContainer?.scrollBy({ left: 160, behavior: "smooth" });
  }
</script>

<div class="relative flex-1 flex items-center min-w-0 overflow-hidden">
  <!-- Left Arrow Button -->
  {#if canScrollLeft}
    <button
      type="button"
      title="Scroll left"
      onclick={scrollLeft}
      class="absolute left-0 z-10 flex h-7 w-7 -translate-x-1.5 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <span class="icon-[heroicons--chevron-left] h-3.5 w-3.5"></span>
    </button>
  {/if}

  <!-- Circular Champions Carousel -->
  <div
    bind:this={scrollContainer}
    onscroll={updateScrollArrows}
    class="flex items-center gap-2 overflow-x-auto py-1 px-1 scrollbar-none scroll-smooth w-full select-none"
  >
    {#if searchQuery.trim()}
      <!-- Search Results -->
      {#each searchedChampions as champ (champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={`${champ.name}${champ.is_favorite ? " (Favorite)" : ""}`}
            onclick={() => onSelectChampion(champ)}
            class={champ.is_favorite
              ? "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-yellow-400 bg-gray-950 p-0 shadow-sm shadow-yellow-950/40 opacity-90 hover:opacity-100 hover:scale-105 cursor-pointer transition-all"
              : "relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 opacity-70 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all"}
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
              class="pointer-events-none absolute top-[2px] right-[2px] z-20 flex h-3 w-3 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion"
            ></span>
          {/if}
        </div>
      {/each}
    {:else}
      <!-- Favorites List -->
      {#each favoriteChampions as champ (champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={`${champ.name} (Favorite)`}
            onclick={() => onSelectChampion(champ)}
            class="relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-yellow-400 bg-gray-950 p-0 shadow-sm shadow-yellow-950/40 opacity-90 hover:opacity-100 hover:scale-105 cursor-pointer transition-all"
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
              class="pointer-events-none absolute top-[2px] right-[2px] z-20 flex h-3 w-3 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion"
            ></span>
          {/if}
        </div>
      {/each}

      <!-- All Champions button -->
      {#if selectedChampion !== null}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title="All Champions (Viewing universal runes)"
            onclick={() => onSelectChampion(null)}
            class="group relative flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-gray-700/80 bg-gray-950 p-0 opacity-70 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all overflow-hidden"
          >
            <img
              src={ALL_CHAMPIONS_ICON_URL}
              alt="All Champions"
              class="opacity-80 w-[70%] h-[70%] object-contain transition-transform group-hover:scale-110"
            />
          </button>

          {#if isRuneMismatch && (!activeRuneChampionId || activeRuneChampionId === 0)}
            <span
              class="pointer-events-none absolute top-[2px] right-[2px] z-20 flex h-3 w-3 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
              title="Active rune page champion (All Champions)"
            ></span>
          {/if}
        </div>
      {/if}

      <!-- Non-Favorite Champions List -->
      {#each nonFavoriteChampions as champ (champ.id)}
        <div class="relative flex shrink-0 items-center justify-center">
          <button
            type="button"
            title={champ.name}
            onclick={() => onSelectChampion(champ)}
            class="relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 opacity-70 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all"
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
              class="pointer-events-none absolute top-[2px] right-[2px] z-20 flex h-3 w-3 rounded-full bg-blue-500 shadow-[0_1px_3px_rgba(29,78,216,0.85)]"
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
      class="absolute right-0 z-10 flex h-7 w-7 translate-x-1.5 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
    >
      <span class="icon-[heroicons--chevron-right] h-3.5 w-3.5"></span>
    </button>
  {/if}
</div>
