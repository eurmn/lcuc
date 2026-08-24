<script lang="ts">
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";

  let {
    selectedChampion = null,
    onDeselect,
    onToggleFavorite,
  }: {
    selectedChampion: ChampionSummary | null;
    onDeselect: () => void;
    onToggleFavorite?: (championId: number) => void;
  } = $props();
</script>

<div class="relative flex items-center justify-center shrink-0">
  {#if selectedChampion === null}
    <!-- Active "All Champions" State -->
    <button
      type="button"
      title="All Champions (Viewing universal runes)"
      onclick={onDeselect}
      class="group relative flex h-11 w-11 shrink-0 items-center justify-center rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 cursor-pointer transition-all hover:scale-105 overflow-hidden"
    >
      <img
        src={ALL_CHAMPIONS_ICON_URL}
        alt="All Champions"
        class="opacity-80 w-[70%] h-[70%] object-contain transition-transform group-hover:scale-110"
      />
    </button>
  {:else}
    <!-- Active Specific Selected Champion State with Overlay Actions -->
    <div class="group relative flex items-center justify-center">
      <!-- Favorite Bookmark Button (Overlay Top-Left) -->
      <button
        type="button"
        title={selectedChampion.is_favorite ? `Unfavorite ${selectedChampion.name}` : `Favorite ${selectedChampion.name}`}
        onclick={(e) => {
          e.stopPropagation();
          if (selectedChampion) {
            onToggleFavorite?.(selectedChampion.id);
          }
        }}
        class="absolute -top-1.5 -left-1.5 z-20 flex items-center justify-center cursor-pointer focus:outline-none drop-shadow-[0_1px_3px_rgba(0,0,0,0.9)]"
      >
        <span
          class="icon-[heroicons--bookmark-20-solid] h-5 w-5 transition-opacity {selectedChampion.is_favorite ? 'text-purple-400 drop-shadow-[0_0_5px_rgba(168,85,247,0.6)] opacity-90 hover:opacity-100' : 'text-slate-200 opacity-60 hover:opacity-100'}"
        ></span>
      </button>

      <!-- Champion Avatar Button -->
      <button
        type="button"
        title={`${selectedChampion.name} (Click to deselect / show all)`}
        onclick={onDeselect}
        class="relative flex h-11 w-11 shrink-0 overflow-hidden rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 cursor-pointer transition-all hover:scale-105 hover:brightness-110"
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

      <!-- Clear / Deselect Badge Button (Overlay Top-Right) -->
      <button
        type="button"
        title="Deselect champion"
        onclick={(e) => {
          e.stopPropagation();
          onDeselect();
        }}
        class="absolute -top-1 -right-1 z-20 flex h-4 w-4 items-center justify-center rounded-full bg-gray-900 border border-gray-700 text-gray-400 opacity-80 hover:opacity-100 hover:bg-red-500 hover:text-white hover:border-red-600 transition-all cursor-pointer shadow-sm"
      >
        <span class="icon-[heroicons--x-mark] h-2.5 w-2.5"></span>
      </button>
    </div>
  {/if}
</div>
