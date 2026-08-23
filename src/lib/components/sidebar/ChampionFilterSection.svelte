<script lang="ts">
  import { MotionDiv, type MotionTransition } from "@humanspeak/svelte-motion";
  import SelectedChampionSlot from "./SelectedChampionSlot.svelte";
  import ChampionCarousel from "./ChampionCarousel.svelte";
  import type { ChampionSummary } from "../../types/champions";

  let {
    isOpen = false,
    selectedChampion = null,
    allChampions = [],
    activeRuneChampionId = null,
    championSearchQuery = $bindable(""),
    layoutTransition,
    onSelectChampion,
    onToggleFavorite,
  }: {
    isOpen: boolean;
    selectedChampion: ChampionSummary | null;
    allChampions: ChampionSummary[];
    activeRuneChampionId?: number | null;
    championSearchQuery: string;
    layoutTransition: MotionTransition;
    onSelectChampion: (champ: ChampionSummary | null) => void;
    onToggleFavorite: (champId: number) => void;
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
</script>

<MotionDiv
  layout="position"
  transition={layoutTransition}
  class="flex flex-col gap-2 rounded-2xl border border-gray-800 bg-gray-950/80 p-2 shadow-md overflow-hidden w-full min-w-0"
>
  <!-- Search Input (Smoothly collapses to height 0 and opacity 0 without scale distortion) -->
  <MotionDiv
    animate={{ opacity: isOpen ? 1 : 0, height: isOpen ? "auto" : 0 }}
    transition={layoutTransition}
    class="relative w-full overflow-hidden"
  >
    <div
      class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400"
    >
      <span class="icon-[heroicons--magnifying-glass] h-4 w-4"></span>
    </div>
    <input
      type="text"
      bind:value={championSearchQuery}
      placeholder="Search champion..."
      class="w-full rounded-xl border border-gray-700 bg-gray-900/90 py-1.5 pl-9 pr-8 text-xs font-semibold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
    />
    {#if championSearchQuery}
      <button
        type="button"
        title="Clear search"
        aria-label="Clear search"
        onclick={() => (championSearchQuery = "")}
        class="absolute inset-y-0 right-0 flex items-center pr-2.5 text-gray-400 hover:text-white cursor-pointer"
      >
        <span class="icon-[heroicons--x-mark] h-3.5 w-3.5"></span>
      </button>
    {/if}
  </MotionDiv>

  <!-- Champion Selector Row: Dedicated Selected Champion Slot (Pinned First) + Scrollable Results -->
  <div
    class="flex items-center {isOpen
      ? 'justify-start'
      : 'justify-center'} gap-2 w-full min-w-0 overflow-hidden"
  >
    <!-- 1. Dedicated Selected Champion Component (Pinned First) -->
    <div class="shrink-0 flex items-center justify-center">
      <SelectedChampionSlot
        {selectedChampion}
        onDeselect={() => onSelectChampion(null)}
        {onToggleFavorite}
      />
    </div>

    <!-- 2. Divider & Carousel (Progressively eaten by overflow as sidebar shrinks) -->
    <MotionDiv
      animate={{ opacity: isOpen ? 1 : 0, width: isOpen ? "auto" : 0 }}
      transition={layoutTransition}
      class="flex items-center gap-2 min-w-0 flex-1 overflow-hidden"
    >
      <!-- Vertical Divider -->
      <div class="h-8 w-px bg-gray-800 shrink-0"></div>

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
    </MotionDiv>
  </div>
</MotionDiv>
