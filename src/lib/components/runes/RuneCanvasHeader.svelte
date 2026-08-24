<script lang="ts">
  import RuneFilterTray from "./RuneFilterTray.svelte";
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import type { RuneTreePayload } from "../../types/runes";
  import { LEAGUE_ROLES_MAP, type LeagueRole } from "../../types/rune_builder";

  let {
    tree = null,
    runeChampion = null,
    pageName = $bindable("Custom Rune Page"),
    isEditing = false,
    saving = false,
    exporting = false,
    selectedRole = 1,
    selectedTiers = [4, 5, 6, 7],
    onOpenChampionModal,
    onEdit,
    onCancel,
    onSave,
    onExport,
    onSelectRole,
    onToggleTier,
  }: {
    tree: RuneTreePayload | null;
    runeChampion: ChampionSummary | null;
    pageName: string;
    isEditing: boolean;
    saving: boolean;
    exporting: boolean;
    selectedRole: number;
    selectedTiers: number[];
    onOpenChampionModal: () => void;
    onEdit: () => void;
    onCancel: () => void;
    onSave: () => void;
    onExport: () => void;
    onSelectRole: (role: number) => void;
    onToggleTier: (tier: number) => void;
  } = $props();
</script>

<div class="flex flex-col rounded-2xl border border-gray-800 bg-gray-900 p-4 shadow-md">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <!-- Page Title & Champion Avatar / Asterisk -->
    <div class="flex items-center gap-2.5 flex-1 min-w-[240px]">
      <!-- Indicator Dot -->
      <span
        class="flex h-3 w-3 rounded-full {tree?.page_id ? 'bg-blue-500 shadow-blue-500' : 'bg-purple-500 shadow-purple-500'} shadow-sm shrink-0"
      ></span>

      <!-- Champion Icon with Clear Hover/Edit State in Edit Mode -->
      <button
        type="button"
        onclick={() => isEditing && onOpenChampionModal()}
        disabled={!isEditing}
        title={isEditing ? `Champion: ${runeChampion?.name || 'All Champions'} (Click to change)` : (runeChampion?.name || "All Champions")}
        class={isEditing
          ? "group relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 hover:border-purple-400 hover:scale-105 cursor-pointer transition-all"
          : "relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-md cursor-default"}
      >
        {#if runeChampion && runeChampion.id > 0 && runeChampion.square_icon_url}
          <img
            src={runeChampion.square_icon_url}
            alt={runeChampion.name}
            class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+5px)] w-[calc(100%+5px)] object-cover rounded-full transition-all group-hover:brightness-50"
          />
        {:else}
          <img
            src={ALL_CHAMPIONS_ICON_URL}
            alt="All Champions"
            class="opacity-80 w-[70%] h-[70%] object-contain transition-all group-hover:brightness-50"
          />
        {/if}

        {#if isEditing}
          <!-- Hover Edit Badge Overlay -->
          <div
            class="absolute inset-0 bg-black/60 rounded-full flex flex-col items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity z-10"
          >
            <span class="icon-[heroicons--pencil] h-3.5 w-3.5 text-purple-300"></span>
            <span class="text-[7px] font-black tracking-wider text-purple-200 uppercase">EDIT</span>
          </div>
        {/if}
      </button>

      <!-- Page Name Display (View Mode) or Input (Edit Mode) -->
      {#if isEditing}
        <input
          type="text"
          bind:value={pageName}
          placeholder="Rune page name..."
          class="w-full max-w-xs rounded-xl border border-gray-700 bg-gray-900 px-3 py-1.5 text-sm font-bold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
        />
      {:else}
        <div class="flex items-center gap-2">
          <h2 class="text-base font-extrabold text-white truncate max-w-md">
            {pageName || tree?.page_name || "Custom Rune Page"}
          </h2>
          <span class="inline-flex items-center rounded-lg bg-gray-800/90 border border-gray-700/60 px-2 py-0.5 text-[11px] font-semibold text-gray-300">
            {LEAGUE_ROLES_MAP[selectedRole as LeagueRole]?.label || "Jungle"}
          </span>
        </div>
      {/if}
    </div>

    <!-- Action Buttons -->
    <div class="flex items-center gap-2 shrink-0">
      {#if isEditing}
        <!-- Cancel Button (Dark / Solid Grey, Fixed 5rem / w-20) -->
        <button
          type="button"
          onclick={onCancel}
          class="h-8 inline-flex w-20 items-center justify-center gap-2 rounded-xl text-white bg-gray-800 hover:bg-gray-700 focus:ring-4 focus:ring-gray-700 box-border border border-transparent shadow-xs px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
        >
          <span>Cancel</span>
        </button>

        <!-- Save Changes Button (Purple Action, Fixed 8rem / w-32) -->
        <button
          type="button"
          disabled={saving}
          onclick={onSave}
          class="h-8 inline-flex w-32 items-center justify-center gap-2 rounded-xl bg-purple-600 px-3.5 py-1.5 text-xs font-semibold text-white shadow-md shadow-purple-950/50 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
        >
          <span class="icon-[flowbite--floppy-disk-solid] h-4.5 w-4.5"></span>
          <span>{saving ? "Saving..." : "Save changes"}</span>
        </button>
      {:else}
        <!-- View Mode: Edit Button (Dark / Solid Grey, Fixed 5rem / w-20) -->
        <button
          title="Edit rune page"
          type="button"
          onclick={onEdit}
          class="h-8 inline-flex w-20 items-center justify-center gap-2 rounded-xl text-white bg-gray-800 hover:bg-gray-700 focus:ring-4 focus:ring-gray-700 box-border border border-transparent shadow-xs px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
        >
          <span class="icon-[heroicons--pencil-square-16-solid] h-4.5 w-4.5"></span>
          <span>Edit</span>
        </button>

        <!-- View Mode: Send to Client Button (Gradient Duotone Purple to Blue, Fixed 8rem / w-34) -->
        <button
          type="button"
          disabled={exporting}
          onclick={onExport}
          class="h-8 inline-flex w-34 items-center justify-center gap-2 rounded-xl text-white bg-gradient-to-br from-purple-600 to-blue-500 hover:bg-gradient-to-bl focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-800 shadow-md shadow-purple-950/40 px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
        >
          <span class="icon-[heroicons--bolt-16-solid] h-4.5 w-4.5"></span>
          <span>{exporting ? "Sending..." : "Send to Client"}</span>
        </button>
      {/if}
    </div>
  </div>

  <!-- Filters (Role & Rank Tiers) Animated Expansion and Collapse -->
  <RuneFilterTray
    {isEditing}
    {selectedRole}
    {selectedTiers}
    {onSelectRole}
    {onToggleTier}
  />
</div>
