<script lang="ts">
  import { onMount } from "svelte";
  import ProfileCard from "../ProfileCard.svelte";
  import SavedRunesList from "./SavedRunesList.svelte";
  import ClientRunesList from "./ClientRunesList.svelte";
  import type { ClientRunePageSummary, RuneSetSummary } from "../../types/runes";
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_OPTION, ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getChampions } from "../../api/champions";

  let {
    clientPages = [],
    savedRunesets = [],
    activeCompanionName = null,
    selectedKey = "",
    loading = false,
    onSelectPage,
    onImportPage,
    onDeletePage,
    onSelectChampionFilter,
  }: {
    clientPages: ClientRunePageSummary[];
    savedRunesets: RuneSetSummary[];
    activeCompanionName: string | null;
    selectedKey: string;
    loading: boolean;
    onSelectPage: (key: string) => void;
    onImportPage: (pageId: number) => void;
    onDeletePage: (runesetId: number) => void;
    onSelectChampionFilter?: (champion: ChampionSummary) => void;
  } = $props();

  let profileCardRef = $state<ProfileCard | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let championSearchQuery = $state("");
  let selectedChampionId = $state<number>(0);
  let scrollContainer = $state<HTMLDivElement | null>(null);
  let canScrollLeft = $state<boolean>(false);
  let canScrollRight = $state<boolean>(false);

  export function refreshProfile() {
    profileCardRef?.refreshProfile();
  }

  function updateScrollArrows() {
    if (!scrollContainer) return;
    canScrollLeft = scrollContainer.scrollLeft > 4;
    canScrollRight =
      scrollContainer.scrollLeft <
      scrollContainer.scrollWidth - scrollContainer.clientWidth - 4;
  }

  onMount(async () => {
    try {
      allChampions = await getChampions();
      setTimeout(updateScrollArrows, 50);
    } catch (err) {
      console.error("[Sidebar Champions Load Error]", err);
    }
  });

  const displayedChampions = $derived.by(() => {
    const validChamps = allChampions.filter(
      (c) => !c.id.toString().startsWith("600")
    );
    const q = championSearchQuery.trim().toLowerCase();
    if (!q) return validChamps;
    return validChamps.filter(
      (c) => c.name.toLowerCase().includes(q) || c.alias.toLowerCase().includes(q)
    );
  });

  $effect(() => {
    if (displayedChampions) {
      setTimeout(updateScrollArrows, 50);
    }
  });

  // Filter saved runes if a champion is searched / filtered
  const filteredSavedRunes = $derived.by(() => {
    if (selectedChampionId > 0) {
      return savedRunesets.filter((r) => r.champion_id === selectedChampionId);
    }
    const q = championSearchQuery.trim().toLowerCase();
    if (!q) return savedRunesets;
    return savedRunesets.filter((r) => r.rune_page_name.toLowerCase().includes(q));
  });

  function selectChampion(champ: ChampionSummary) {
    selectedChampionId = champ.id;
    onSelectChampionFilter?.(champ);
  }

  function scrollLeft() {
    scrollContainer?.scrollBy({ left: -160, behavior: "smooth" });
  }

  function scrollRight() {
    scrollContainer?.scrollBy({ left: 160, behavior: "smooth" });
  }
</script>

<aside class="w-full max-w-sm shrink-0 border-r border-gray-800 bg-gray-900/60 overflow-y-auto h-full select-none">
  <div class="h-full p-4 flex flex-col gap-4">
    <!-- Profile Card Container -->
    <ProfileCard bind:this={profileCardRef} />

    <!-- Champion Selector & Search Container (Horizontally Scrollable with Arrows) -->
    <div class="flex flex-col gap-2.5 rounded-2xl border border-gray-800 bg-gray-950/80 p-3 shadow-md">
      <!-- Search Input on Top -->
      <div class="relative">
        <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
          </svg>
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
            <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
            </svg>
          </button>
        {/if}
      </div>

      <!-- Horizontal Scrollable Champion Row with Arrows -->
      <div class="relative flex items-center">
        <!-- Left Arrow Button (Only if can scroll left) -->
        {#if canScrollLeft}
          <button
            type="button"
            title="Scroll left"
            onclick={scrollLeft}
            class="absolute left-0 z-10 flex h-7 w-7 -translate-x-1.5 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
          >
            <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
            </svg>
          </button>
        {/if}

        <!-- Horizontal Circular Champions Container -->
        <div
          bind:this={scrollContainer}
          onscroll={updateScrollArrows}
          class="flex items-center gap-2 overflow-x-auto py-1 px-1 scrollbar-none scroll-smooth w-full select-none"
        >
          <!-- "All Champions" Circular Button (Clash Autofill Icon) -->
          <button
            type="button"
            title="All Champions"
            onclick={() => selectChampion(ALL_CHAMPIONS_OPTION)}
            class={selectedChampionId === 0
              ? "relative flex h-12 w-12 shrink-0 items-center justify-center rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 scale-105 cursor-pointer transition-all overflow-hidden"
              : "relative flex h-12 w-12 shrink-0 items-center justify-center rounded-full border border-gray-700/80 bg-gray-950 p-0 opacity-60 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all overflow-hidden"}
          >
            <img
              src={ALL_CHAMPIONS_ICON_URL}
              alt="All Champions"
              class="opacity-80 w-[70%] h-[70%] object-contain"
            />
          </button>

          <!-- Circular Champion Avatars (Edge-Cropped) -->
          {#each displayedChampions as champ (champ.id)}
            <button
              type="button"
              title={champ.name}
              onclick={() => selectChampion(champ)}
              class={selectedChampionId === champ.id
                ? "relative flex h-12 w-12 shrink-0 overflow-hidden rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 scale-105 cursor-pointer transition-all"
                : "relative flex h-12 w-12 shrink-0 overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 opacity-60 hover:opacity-100 hover:border-gray-500 hover:scale-105 cursor-pointer transition-all"}
            >
              {#if champ.square_icon_url}
                <img
                  src={champ.square_icon_url}
                  alt={champ.name}
                  class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+5px)] w-[calc(100%+5px)] object-cover rounded-full"
                />
              {:else}
                <div class="h-full w-full rounded-full bg-gray-800"></div>
              {/if}

              {#if champ.has_saved_runes}
                <span
                  class="absolute bottom-0 right-0 z-10 flex h-2.5 w-2.5 rounded-full bg-emerald-400 border border-gray-900 shadow-sm"
                ></span>
              {/if}
            </button>
          {/each}
        </div>

        <!-- Right Arrow Button (Only if can scroll right) -->
        {#if canScrollRight}
          <button
            type="button"
            title="Scroll right"
            onclick={scrollRight}
            class="absolute right-0 z-10 flex h-7 w-7 translate-x-1.5 items-center justify-center rounded-full border border-gray-700 bg-gray-800/90 text-gray-300 shadow-md hover:border-purple-500 hover:bg-purple-600 hover:text-white transition-all cursor-pointer"
          >
            <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
            </svg>
          </button>
        {/if}
      </div>
    </div>

    <!-- Separator -->
    <hr class="border-gray-800" />

    {#if loading}
      <!-- Skeleton Loaders -->
      <div class="space-y-6 animate-pulse">
        <div class="space-y-3">
          <div class="h-3.5 w-28 rounded-full bg-gray-800"></div>
          <div class="h-16 w-full rounded-2xl bg-gray-800/40"></div>
          <div class="h-16 w-full rounded-2xl bg-gray-800/40"></div>
        </div>
      </div>
    {:else}
      <div id="saved-runes-container" class="space-y-6">
        <!-- Saved Companion Runes -->
        <SavedRunesList
          items={filteredSavedRunes}
          {selectedKey}
          {activeCompanionName}
          onSelect={(id) => onSelectPage(`saved:${id}`)}
          onDelete={onDeletePage}
        />

        <!-- Client Pages -->
        <ClientRunesList
          items={clientPages}
          {selectedKey}
          onSelect={(id) => onSelectPage(`client:${id}`)}
          onImport={onImportPage}
        />
      </div>
    {/if}
  </div>
</aside>
