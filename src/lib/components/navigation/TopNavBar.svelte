<script lang="ts">
  import ProfileCard from "../ProfileCard.svelte";
  import TopChampionSearch from "./TopChampionSearch.svelte";
  import type { ChampionSummary } from "../../types/champions";

  let {
    isSidebarOpen = true,
    onToggleSidebar,
    selectedChampion = null,
    allChampions = [],
    activeRuneChampionId = null,
    onSelectChampionFilter,
    onToggleFavoriteChampion,
  }: {
    isSidebarOpen?: boolean;
    onToggleSidebar?: () => void;
    selectedChampion: ChampionSummary | null;
    allChampions: ChampionSummary[];
    activeRuneChampionId?: number | null;
    onSelectChampionFilter?: (champion: ChampionSummary | null) => void;
    onToggleFavoriteChampion?: (championId: number) => void;
  } = $props();

  let profileCardRef = $state<ProfileCard | null>(null);
  let championSearchRef = $state<TopChampionSearch | null>(null);

  export function refreshProfile() {
    profileCardRef?.refreshProfile();
  }

  export function refreshChampions() {
    championSearchRef?.updateScrollArrows();
  }
</script>

<nav
  id="top-navbar"
  class="h-[4.5rem] shrink-0 bg-gray-900 border-b border-gray-800 px-3 pb-2.5 pt-0.5 flex items-center gap-2.5 select-none z-30 min-w-0"
>
  <!-- 1. Sidebar Toggle Button -->
  {#if onToggleSidebar}
    <button
      type="button"
      onclick={onToggleSidebar}
      title={isSidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
      class="flex h-[3.25rem] w-[3.25rem] shrink-0 items-center justify-center rounded-2xl transition-all cursor-pointer {isSidebarOpen
        ? 'bg-gray-800 text-purple-300 hover:text-white'
        : 'bg-gray-900 text-gray-400 hover:text-white hover:bg-gray-800'}"
    >
      {#if isSidebarOpen}
        <span class="icon-[heroicons--bars-3-bottom-left] h-5 w-5"></span>
      {:else}
        <span class="icon-[heroicons--bars-3] h-5 w-5"></span>
      {/if}
    </button>
  {/if}

  <!-- 2. Summoner Profile Card -->
  <ProfileCard bind:this={profileCardRef} />

  <!-- 3. Champion Search Component (spans all remaining horizontal space) -->
  <TopChampionSearch
    bind:this={championSearchRef}
    {selectedChampion}
    {allChampions}
    {activeRuneChampionId}
    onSelectChampion={(champ) => onSelectChampionFilter?.(champ)}
    onToggleFavorite={(id) => onToggleFavoriteChampion?.(id)}
  />
</nav>
