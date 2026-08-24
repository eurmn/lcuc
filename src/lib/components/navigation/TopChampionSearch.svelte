<script lang="ts">
  import { LayoutGroup, MotionDiv, type MotionTransition } from "@humanspeak/svelte-motion";
  import SelectedChampionSlot from "../sidebar/SelectedChampionSlot.svelte";
  import ChampionCarousel from "../sidebar/ChampionCarousel.svelte";
  import type { ChampionSummary } from "../../types/champions";

  let {
    selectedChampion = null,
    allChampions = [],
    activeRuneChampionId = null,
    championSearchQuery = $bindable(""),
    onSelectChampion,
    onToggleFavorite,
  }: {
    selectedChampion: ChampionSummary | null;
    allChampions: ChampionSummary[];
    activeRuneChampionId?: number | null;
    championSearchQuery?: string;
    onSelectChampion: (champ: ChampionSummary | null) => void;
    onToggleFavorite?: (champId: number) => void;
  } = $props();

  let carouselRef = $state<ChampionCarousel | null>(null);

  export function updateScrollArrows() {
    carouselRef?.updateScrollArrows();
  }

  // 1. Favorites list (sorted alphabetically)
  const favoriteChampions = $derived.by(() => {
    return allChampions
      .filter(
        (c) =>
          c.is_favorite &&
          c.id > 0 &&
          !c.id.toString().startsWith("600") &&
          (!selectedChampion || c.id !== selectedChampion.id),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  // 2. Non-favorite champions list (sorted alphabetically)
  const nonFavoriteChampions = $derived.by(() => {
    return allChampions
      .filter(
        (c) =>
          !c.is_favorite &&
          c.id > 0 &&
          !c.id.toString().startsWith("600") &&
          (!selectedChampion || c.id !== selectedChampion.id),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  // 3. Search results (when user types in search bar)
  const searchedChampions = $derived.by(() => {
    const q = championSearchQuery.trim().toLowerCase();
    if (!q) return [];
    return allChampions
      .filter(
        (c) =>
          !c.id.toString().startsWith("600") &&
          c.id > 0 &&
          (!selectedChampion || c.id !== selectedChampion.id) &&
          (c.name.toLowerCase().includes(q) ||
            c.alias.toLowerCase().includes(q)),
      )
      .sort((a, b) => {
        const aFav = a.is_favorite ? 1 : 0;
        const bFav = b.is_favorite ? 1 : 0;
        if (aFav !== bFav) return bFav - aFav;
        return a.name.localeCompare(b.name);
      });
  });

  const isRuneMismatch = $derived.by(() => {
    return (
      (activeRuneChampionId || 0) !==
      (selectedChampion ? selectedChampion.id : 0)
    );
  });
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let isSearchOpen = $state(false);

  const searchTransition: MotionTransition = {
    width: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    opacity: { duration: 0.2, ease: "easeInOut" },
  };

  function toggleSearch() {
    isSearchOpen = !isSearchOpen;
    if (isSearchOpen) {
      setTimeout(() => {
        searchInputRef?.focus();
        carouselRef?.updateScrollArrows();
      }, 50);
    } else {
      championSearchQuery = "";
      setTimeout(() => {
        carouselRef?.updateScrollArrows();
      }, 350);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isSearchOpen) {
      isSearchOpen = false;
      championSearchQuery = "";
      setTimeout(() => {
        carouselRef?.updateScrollArrows();
      }, 350);
    }
  }
</script>

<div
  class="flex flex-1 min-w-0 items-center gap-2.5 px-1 h-[3.25rem] overflow-hidden select-none"
  onkeydown={handleKeydown}
  role="region"
  aria-label="Champion selection and search"
>
  <!-- Dedicated Selected Champion Component (Pinned) -->
  <div class="shrink-0 flex items-center justify-center">
    <SelectedChampionSlot
      {selectedChampion}
      onDeselect={() => onSelectChampion(null)}
      {onToggleFavorite}
    />
  </div>

  <!-- Vertical Divider -->
  <div class="h-6 w-px bg-gray-800 shrink-0"></div>

  <!-- Animated Collapsible / Expandable Search Bar -->
  <MotionDiv
    key="champion-search-container"
    animate={{ width: isSearchOpen ? 256 : 48 }}
    transition={searchTransition}
    class="relative h-11 shrink-0 flex items-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-900/90"
  >
    <!-- Permanent Leading Search Icon (Clickable trigger when closed) -->
    <button
      type="button"
      title={isSearchOpen ? "Search champion" : "Click to search champion"}
      aria-label="Search champion"
      onclick={() => {
        if (!isSearchOpen) toggleSearch();
      }}
      class="absolute inset-y-0 left-0 w-12 flex items-center justify-center text-gray-400 z-20 transition-colors {isSearchOpen
        ? 'pointer-events-none'
        : 'cursor-pointer hover:text-white hover:bg-gray-800'}"
    >
      <span class="icon-[heroicons--magnifying-glass] h-5 w-5 shrink-0"></span>
    </button>

    <!-- Expanding Inner Content (Width matches expanded 256px container, delayed opacity on expand) -->
    <div
      class="absolute inset-y-0 left-12 right-8 flex items-center transition-opacity duration-200 {isSearchOpen
        ? 'opacity-100 delay-150'
        : 'opacity-0 delay-0 pointer-events-none'}"
    >
      <input
        bind:this={searchInputRef}
        type="text"
        tabindex={isSearchOpen ? 0 : -1}
        bind:value={championSearchQuery}
        placeholder="Search champion..."
        class="w-full h-full bg-transparent text-xs sm:text-sm font-semibold text-white placeholder:text-gray-500 focus:outline-none"
      />
    </div>

    <!-- Trailing Close (X) Button -->
    {#if isSearchOpen}
      <button
        type="button"
        title="Close search"
        aria-label="Close search"
        onclick={toggleSearch}
        class="absolute inset-y-0 right-0 w-8 flex items-center justify-center text-gray-400 hover:text-white cursor-pointer z-20 transition-colors"
      >
        <span class="icon-[heroicons--x-mark] h-4 w-4 shrink-0"></span>
      </button>
    {/if}
  </MotionDiv>

  <!-- Horizontally Scrollable Results Carousel with Left/Right Navigation Arrows -->
  <ChampionCarousel
    bind:this={carouselRef}
    {searchedChampions}
    {favoriteChampions}
    {nonFavoriteChampions}
    {selectedChampion}
    searchQuery={championSearchQuery}
    {isRuneMismatch}
    {activeRuneChampionId}
    {onSelectChampion}
  />
</div>
