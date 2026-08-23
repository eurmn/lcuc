<script lang="ts">
  import { onMount } from "svelte";
  import { LayoutGroup, MotionAside, MotionDiv } from "@humanspeak/svelte-motion";
  import ProfileCard from "../ProfileCard.svelte";
  import SelectedChampionSlot from "./SelectedChampionSlot.svelte";
  import SavedRunesList from "./SavedRunesList.svelte";
  import ClientRunesList from "./ClientRunesList.svelte";
  import ChampionSelectModal from "../ChampionSelectModal.svelte";
  import type { ClientRunePageSummary, RuneSetSummary } from "../../types/runes";
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getChampions, toggleFavoriteChampion } from "../../api/champions";

  let {
    isOpen = $bindable(false),
    clientPages = [],
    savedRunesets = [],
    activeCompanionName = null,
    selectedKey = "",
    selectedChampion = null,
    activeRuneChampionId = null,
    loading = false,
    onSelectPage,
    onImportPage,
    onDeletePage,
    onSelectChampionFilter,
    onChangeRunesetChampion,
  }: {
    isOpen?: boolean;
    clientPages: ClientRunePageSummary[];
    savedRunesets: RuneSetSummary[];
    activeCompanionName: string | null;
    selectedKey: string;
    selectedChampion?: ChampionSummary | null;
    activeRuneChampionId?: number | null;
    loading: boolean;
    onSelectPage: (key: string) => void;
    onImportPage: (pageId: number) => void;
    onDeletePage: (runesetId: number) => void;
    onSelectChampionFilter?: (champion: ChampionSummary | null) => void;
    onChangeRunesetChampion?: (runesetId: number, championId: number) => void;
  } = $props();

  let profileCardRef = $state<ProfileCard | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let championSearchQuery = $state("");
  let scrollContainer = $state<HTMLDivElement | null>(null);
  let canScrollLeft = $state<boolean>(false);
  let canScrollRight = $state<boolean>(false);

  // Modal state for editing saved runeset champion
  let editingRunesetId = $state<number | null>(null);
  let editingRunesetCurrentChampId = $state<number | null>(null);
  let isRunesetChampionModalOpen = $state<boolean>(false);

  export function refreshProfile() {
    profileCardRef?.refreshProfile();
  }

  export async function refreshChampions() {
    try {
      allChampions = await getChampions();
      setTimeout(updateScrollArrows, 50);
    } catch (err) {
      console.error("[Sidebar Champions Refresh Error]", err);
    }
  }

  function updateScrollArrows() {
    if (!scrollContainer) return;
    canScrollLeft = scrollContainer.scrollLeft > 4;
    canScrollRight =
      scrollContainer.scrollLeft <
      scrollContainer.scrollWidth - scrollContainer.clientWidth - 4;
  }

  onMount(async () => {
    await refreshChampions();
  });

  // 1. Favorites list (sorted alphabetically)
  const favoriteChampions = $derived.by(() => {
    return allChampions
      .filter(
        (c) =>
          c.is_favorite &&
          c.id > 0 &&
          !c.id.toString().startsWith("600") &&
          (!selectedChampion || c.id !== selectedChampion.id)
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
          (!selectedChampion || c.id !== selectedChampion.id)
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
          (c.name.toLowerCase().includes(q) || c.alias.toLowerCase().includes(q))
      )
      .sort((a, b) => {
        const aFav = a.is_favorite ? 1 : 0;
        const bFav = b.is_favorite ? 1 : 0;
        if (aFav !== bFav) return bFav - aFav;
        return a.name.localeCompare(b.name);
      });
  });

  const isRuneMismatch = $derived.by(() => {
    return (activeRuneChampionId || 0) !== (selectedChampion ? selectedChampion.id : 0);
  });

  $effect(() => {
    if (favoriteChampions || nonFavoriteChampions || searchedChampions) {
      setTimeout(updateScrollArrows, 50);
    }
  });

  // Filter saved runes:
  // - Shows saved runes for selectedChampion (or universal runes if selectedChampion is null)
  // - If the currently selected rune page (selectedKey) is from a different champion, prepend it before the current champion's saved runes!
  const filteredSavedRunes = $derived.by(() => {
    let list: RuneSetSummary[];
    if (selectedChampion && selectedChampion.id > 0) {
      list = savedRunesets.filter(
        (r) => r.champion_id === selectedChampion.id || !r.champion_id || r.champion_id === 0
      );
    } else {
      list = savedRunesets.filter(
        (r) => !r.champion_id || r.champion_id === 0
      );
    }

    // If the active/selected page is a saved runeset from a different champion, prepend it before the current champion's saved runes
    if (selectedKey.startsWith("saved:")) {
      const activeId = parseInt(selectedKey.slice("saved:".length), 10);
      const activeRuneset = savedRunesets.find((r) => r.id === activeId);
      if (activeRuneset && !list.some((r) => r.id === activeId)) {
        list = [activeRuneset, ...list];
      }
    }

    const q = championSearchQuery.trim().toLowerCase();
    if (!q) return list;
    return list.filter((r) => r.rune_page_name.toLowerCase().includes(q));
  });

  function selectChampion(champ: ChampionSummary | null) {
    onSelectChampionFilter?.(champ);
  }

  async function handleToggleFavorite(championId: number) {
    try {
      const updated = await toggleFavoriteChampion(championId);
      allChampions = allChampions.map((c) =>
        c.id === championId ? { ...c, is_favorite: updated } : c
      );
    } catch (err) {
      console.error("[Toggle Favorite Error]", err);
    }
  }


  function handleOpenChangeChampionModal(runesetId: number, currentChampId: number) {
    editingRunesetId = runesetId;
    editingRunesetCurrentChampId = currentChampId;
    isRunesetChampionModalOpen = true;
  }

  function handleSelectRunesetChampion(champ: ChampionSummary | null) {
    if (editingRunesetId !== null) {
      const champId = champ ? champ.id : 0;
      onChangeRunesetChampion?.(editingRunesetId, champId);
    }
  }

  function scrollLeft() {
    scrollContainer?.scrollBy({ left: -160, behavior: "smooth" });
  }

  function scrollRight() {
    scrollContainer?.scrollBy({ left: 160, behavior: "smooth" });
  }

  let layoutTransition = $derived({
    x: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    width: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    y: { duration: 0.35, ease: "linear" },
    height: { duration: 0.35, ease: "linear" },
    opacity: { duration: 0.35, ease: "linear" },
  });
</script>

<LayoutGroup id="sidebar-rail">
  <MotionAside
    key="runes-sidebar"
    animate={{ width: isOpen ? 384 : 130 }}
    transition={layoutTransition}
    class="h-full shrink-0 border-r border-gray-800 bg-gray-900/60 overflow-hidden select-none z-20"
  >
    <div class="h-full w-full p-2.5 flex flex-col gap-3 select-none overflow-y-auto overflow-x-hidden min-w-0">
      <!-- Profile Card Container with right-sticked collapse button -->
      <ProfileCard
        bind:this={profileCardRef}
        {isOpen}
        onToggleCollapse={() => (isOpen = !isOpen)}
      />

      <!-- Champion Selector & Search Container -->
      <MotionDiv layout="position" transition={layoutTransition} class="flex flex-col gap-2 rounded-2xl border border-gray-800 bg-gray-950/80 p-2 shadow-md overflow-hidden w-full min-w-0">
        <!-- Search Input (Smoothly collapses to height 0 and opacity 0 without scale distortion) -->
        <MotionDiv
          animate={{ opacity: isOpen ? 1 : 0, height: isOpen ? "auto" : 0 }}
          transition={layoutTransition}
          class="relative w-full overflow-hidden"
        >
          <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
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
        <div class="flex items-center {isOpen ? 'justify-start' : 'justify-center'} gap-2 w-full min-w-0 overflow-hidden">
          <!-- 1. Dedicated Selected Champion Component (Pinned First) -->
          <div class="shrink-0 flex items-center justify-center">
            <SelectedChampionSlot
              {selectedChampion}
              onDeselect={() => selectChampion(null)}
              onToggleFavorite={handleToggleFavorite}
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
                {#if championSearchQuery.trim()}
                  <!-- Search Results -->
                  {#each searchedChampions as champ (champ.id)}
                    <div class="relative flex shrink-0 items-center justify-center">
                      <button
                        type="button"
                        title={`${champ.name}${champ.is_favorite ? ' (Favorite)' : ''}`}
                        onclick={() => selectChampion(champ)}
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
                        onclick={() => selectChampion(champ)}
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
                        onclick={() => selectChampion(null)}
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
                        onclick={() => selectChampion(champ)}
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
          </MotionDiv>
        </div>
      </MotionDiv>

      <!-- Separator -->
      <hr class="border-gray-800" />

      {#if loading}
        <!-- Skeleton Loaders -->
        <div class="space-y-4 animate-pulse w-full">
          <div class="h-3 w-16 mx-auto rounded-full bg-gray-800"></div>
          <div class="h-12 w-full rounded-2xl bg-gray-800/40"></div>
          <div class="h-12 w-full rounded-2xl bg-gray-800/40"></div>
        </div>
      {:else}
        <div id="saved-runes-container" class="space-y-4 w-full min-w-0">
          <!-- Saved Companion Runes -->
          <SavedRunesList
            items={filteredSavedRunes}
            {selectedKey}
            {activeCompanionName}
            {allChampions}
            {isOpen}
            onSelect={(id) => onSelectPage(`saved:${id}`)}
            onDelete={onDeletePage}
            onChangeChampion={handleOpenChangeChampionModal}
          />

          <!-- Client Pages -->
          <ClientRunesList
            items={clientPages}
            {selectedKey}
            {isOpen}
            onSelect={(id) => onSelectPage(`client:${id}`)}
            onImport={onImportPage}
          />
        </div>
      {/if}
    </div>
  </MotionAside>
</LayoutGroup>

<!-- Flowbite Champion Select Modal for changing saved runeset champion -->
<ChampionSelectModal
  isOpen={isRunesetChampionModalOpen}
  title="Change Rune Page Champion"
  currentChampionId={editingRunesetCurrentChampId}
  onSelect={handleSelectRunesetChampion}
  onClose={() => (isRunesetChampionModalOpen = false)}
/>
