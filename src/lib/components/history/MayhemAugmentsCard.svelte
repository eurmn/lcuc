<script lang="ts">
  import { getMayhemAugments } from "../../api/champ_select";
  import type { MayhemAugmentsResponse } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import { logger } from "../../utils/logger";

  let {
    championId = 0,
    championName = "",
    championWinRate = 0.5,
    totalGames = 0,
    hideLowPickrate = false,
  }: {
    championId?: number;
    championName?: string;
    championWinRate?: number;
    totalGames?: number;
    hideLowPickrate?: boolean;
  } = $props();

  let augmentsResponse = $state<MayhemAugmentsResponse | null>(null);
  let loadingAugments = $state(false);
  let augmentTier = $state<string>("all");
  let augmentCategory = $state<string>("all");
  let augmentSearch = $state<string>("");
  let debouncedSearch = $state<string>("");
  let augmentSortKey = $state<string>("wpa");
  let augmentDescending = $state<boolean>(true);
  let augmentPage = $state<number>(1);
  let augmentPageSize = $state<number>(10);
  let onlySampled = $state<boolean>(false);
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function formatPct(val: number | undefined): string {
    if (val === undefined || isNaN(val)) return "0.0%";
    return (val * 100).toFixed(1) + "%";
  }

  function formatWpa(val: number | undefined): string {
    if (val === undefined || isNaN(val)) return "+0.0%";
    const sign = val >= 0 ? "+" : "";
    return sign + (val * 100).toFixed(1) + "%";
  }

  function handleTierSelect(tier: string) {
    if (augmentTier === tier) return;
    augmentTier = tier;
    augmentPage = 1;
  }

  function handleCategoryChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    augmentCategory = target.value;
    augmentPage = 1;
  }

  function handleSortChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    augmentSortKey = target.value;
    augmentPage = 1;
  }

  function handleHeaderSort(key: string) {
    if (augmentSortKey === key) {
      augmentDescending = !augmentDescending;
    } else {
      augmentSortKey = key;
      // Name sorts ascending by default; numeric metrics sort descending by default
      augmentDescending = key !== "name";
    }
    augmentPage = 1;
  }

  function toggleSortDirection() {
    augmentDescending = !augmentDescending;
  }

  function toggleOnlySampled() {
    onlySampled = !onlySampled;
    augmentPage = 1;
  }

  function handleSearchInput(event: Event) {
    const val = (event.target as HTMLInputElement).value;
    augmentSearch = val;
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      augmentPage = 1;
      debouncedSearch = val;
    }, 250);
  }

  function clearSearch() {
    augmentSearch = "";
    debouncedSearch = "";
    augmentPage = 1;
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  }

  function handlePageChange(newPage: number) {
    if (!augmentsResponse) return;
    if (newPage < 1 || newPage > augmentsResponse.total_pages) return;
    augmentPage = newPage;
  }

  function handlePageSizeChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    augmentPageSize = parseInt(target.value, 10) || 10;
    augmentPage = 1;
  }

  function resetAllFilters() {
    augmentTier = "all";
    augmentCategory = "all";
    augmentSearch = "";
    debouncedSearch = "";
    augmentSortKey = "wpa";
    augmentDescending = true;
    onlySampled = false;
    augmentPage = 1;
  }

  // Reactive effect fetching data whenever any parameter changes
  $effect(() => {
    const _cid = championId;
    const _os = onlySampled;
    const _hlp = hideLowPickrate;
    const _tier = augmentTier;
    const _cat = augmentCategory;
    const _sort = augmentSortKey;
    const _desc = augmentDescending;
    const _page = augmentPage;
    const _size = augmentPageSize;
    const _search = debouncedSearch;

    void (async () => {
      loadingAugments = true;
      try {
        augmentsResponse = await getMayhemAugments({
          champion_id: _cid > 0 ? _cid : null,
          tier: _tier === "all" ? undefined : _tier,
          category: _cat === "all" ? undefined : _cat,
          search: _search.trim() || undefined,
          sort_key: _sort,
          descending: _desc,
          page: _page,
          page_size: _size,
          only_sampled: _os,
          hide_low_pickrate: _hlp,
        });
      } catch (err) {
        logger.error("frontend.mayhem_augments_card", "Failed fetching augments", err);
      } finally {
        loadingAugments = false;
      }
    })();
  });
</script>

<div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-6 shadow-xl flex flex-col gap-4">
  <!-- Card Header -->
  <div class="flex flex-wrap items-center justify-between gap-3 border-b border-gray-800 pb-4">
    <div class="flex items-center gap-3">
      <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-purple-600/20 text-purple-400 border border-purple-500/30 shadow-inner">
        <span class="icon-[heroicons--sparkles] h-5 w-5"></span>
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h2 class="text-base font-bold text-gray-100">ARAM Mayhem Augments</h2>
          <span class="rounded bg-purple-950 border border-purple-800/80 px-2 py-0.5 text-[11px] font-mono font-semibold text-purple-300">
            Catalog &amp; Stats
          </span>
        </div>
        <p class="text-xs text-gray-400 mt-0.5">
          {#if championName}
            Synergies calculated for <strong class="text-gray-200">{championName}</strong> ({formatPct(championWinRate)} Base WR)
          {:else}
            Complete catalog &amp; statistical win probabilities
          {/if}
        </p>
      </div>
    </div>

    <!-- Quick search input in header -->
    <div class="relative w-full sm:w-64">
      <span class="icon-[heroicons--magnifying-glass] pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400"></span>
      <input
        type="text"
        placeholder="Filter by name or effect..."
        value={augmentSearch}
        oninput={handleSearchInput}
        class="w-full rounded-xl border border-gray-700 bg-gray-950/80 pl-9 pr-8 py-2 text-xs text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
      />
      {#if augmentSearch}
        <button
          type="button"
          title="Clear search"
          onclick={clearSearch}
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-200 cursor-pointer"
        >
          <span class="icon-[heroicons--x-mark] h-4 w-4"></span>
        </button>
      {/if}
    </div>
  </div>

  <!-- Filters Toolbar -->
  <div class="flex flex-wrap items-center justify-between gap-3 text-xs">
    <!-- Left: Tier Filter Pills with official palette -->
    <div class="flex items-center gap-1.5 p-1 rounded-xl bg-gray-950/60 border border-gray-800/80">
      <button
        type="button"
        onclick={() => handleTierSelect("all")}
        class="rounded-lg px-2.5 py-1 font-semibold transition-all cursor-pointer {augmentTier === 'all'
          ? 'bg-purple-600 text-white shadow-sm'
          : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800/60'}"
      >
        All Tiers
      </button>
      <button
        type="button"
        onclick={() => handleTierSelect("prismatic")}
        class="rounded-lg px-2.5 py-1 font-semibold transition-all cursor-pointer flex items-center gap-1.5 {augmentTier === 'prismatic'
          ? 'bg-purple-950/90 text-[#f2c2ff] border border-[#e78fff]/60 shadow-sm'
          : 'text-purple-300/70 hover:text-purple-200 hover:bg-purple-950/40'}"
      >
        <span class="h-2 w-2 rounded-full bg-[#f2c2ff] shadow-sm"></span>
        Prismatic
      </button>
      <button
        type="button"
        onclick={() => handleTierSelect("gold")}
        class="rounded-lg px-2.5 py-1 font-semibold transition-all cursor-pointer flex items-center gap-1.5 {augmentTier === 'gold'
          ? 'bg-amber-950/90 text-[#e3c49f] border border-[#e3c49f]/60 shadow-sm'
          : 'text-amber-300/70 hover:text-amber-200 hover:bg-amber-950/40'}"
      >
        <span class="h-2 w-2 rounded-full bg-[#e3c49f] shadow-sm"></span>
        Gold
      </button>
      <button
        type="button"
        onclick={() => handleTierSelect("silver")}
        class="rounded-lg px-2.5 py-1 font-semibold transition-all cursor-pointer flex items-center gap-1.5 {augmentTier === 'silver'
          ? 'bg-slate-900/90 text-[#a3b9bd] border border-[#a3b9bd]/60 shadow-sm'
          : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/40'}"
      >
        <span class="h-2 w-2 rounded-full bg-[#a3b9bd] shadow-sm"></span>
        Silver
      </button>
    </div>

    <!-- Right: Category, Scope & Sort Controls -->
    <div class="flex flex-wrap items-center gap-2">
      <!-- Category Dropdown -->
      <select
        value={augmentCategory}
        onchange={handleCategoryChange}
        class="rounded-lg border border-gray-700 bg-gray-950/80 px-2.5 py-1 text-xs text-gray-200 focus:border-purple-500 focus:outline-none cursor-pointer"
      >
        <option value="all">All Roles &amp; Classes</option>
        <option value="physical">Physical / AD</option>
        <option value="magic">Magic / AP</option>
        <option value="defensive">Tank / Defense</option>
        <option value="utility">Utility / Speed</option>
      </select>

      <!-- Scope Toggle -->
      <button
        type="button"
        onclick={toggleOnlySampled}
        class="rounded-lg border px-2.5 py-1 font-medium transition-all cursor-pointer flex items-center gap-1.5 {onlySampled
          ? 'bg-purple-950/60 border-purple-500 text-purple-300 shadow-sm'
          : 'bg-gray-950/60 border-gray-700 text-gray-400 hover:text-gray-200 hover:border-gray-600'}"
        title={onlySampled ? "Showing only augments played on this champion" : "Showing all augments in catalog"}
      >
        <span class="icon-[heroicons--user] h-3.5 w-3.5 {onlySampled ? 'text-purple-400' : 'text-gray-500'}"></span>
        {onlySampled ? "Played Only" : "All Catalog"}
      </button>

      <!-- Sort dropdown & toggle -->
      <div class="flex items-center gap-1">
        <select
          value={augmentSortKey}
          onchange={handleSortChange}
          class="rounded-lg border border-gray-700 bg-gray-950/80 px-2.5 py-1 text-xs text-gray-200 focus:border-purple-500 focus:outline-none cursor-pointer"
        >
          <option value="wpa">Sort: WPA (Synergy)</option>
          <option value="win_rate">Sort: Win Rate</option>
          <option value="games">Sort: Games Sampled</option>
          <option value="pick_rate">Sort: Pick Rate</option>
          <option value="tier">Sort: Augment Tier</option>
          <option value="name">Sort: Name (A-Z)</option>
        </select>
        <button
          type="button"
          onclick={toggleSortDirection}
          title={augmentDescending ? "Descending order" : "Ascending order"}
          class="flex h-7 w-7 items-center justify-center rounded-lg border border-gray-700 bg-gray-950/80 text-gray-300 hover:border-purple-500 hover:text-white transition-colors cursor-pointer"
        >
          {#if augmentDescending}
            <span class="icon-[heroicons--bars-arrow-down] h-3.5 w-3.5"></span>
          {:else}
            <span class="icon-[heroicons--bars-arrow-up] h-3.5 w-3.5"></span>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <!-- Augments List / Table Content -->
  <div class="min-h-[380px] relative flex flex-col justify-between">
    {#if loadingAugments && (!augmentsResponse || augmentsResponse.items.length === 0)}
      <div class="flex h-80 items-center justify-center text-xs text-gray-400">
        <span class="icon-[heroicons--arrow-path] h-5 w-5 animate-spin mr-2 text-purple-400"></span>
        Mining augment statistics...
      </div>
    {:else if !augmentsResponse || augmentsResponse.items.length === 0}
      <div class="flex h-80 flex-col items-center justify-center text-center p-6 rounded-xl border border-dashed border-gray-800 bg-gray-950/30">
        <span class="icon-[heroicons--magnifying-glass] h-8 w-8 text-gray-600 mb-2"></span>
        <span class="text-sm font-semibold text-gray-300">No augments match the selected filters</span>
        <p class="text-xs text-gray-500 mt-1 max-w-sm">
          Try clearing search filters or changing the tier and category options.
        </p>
        <button
          type="button"
          onclick={resetAllFilters}
          class="mt-3 rounded-lg border border-purple-500/50 bg-purple-600/20 px-3 py-1.5 text-xs font-semibold text-purple-300 hover:bg-purple-600 hover:text-white transition-colors cursor-pointer"
        >
          Reset All Filters
        </button>
      </div>
    {:else}
      <div class="overflow-x-auto rounded-xl border border-gray-800/90 bg-gray-950/50">
        <table class="w-full text-left text-xs">
          <thead class="border-b border-gray-800 bg-gray-900/80 text-[11px] font-semibold uppercase tracking-wider text-gray-400 select-none">
            <tr>
              <!-- Augment Column Header -->
              <th scope="col" class="py-2.5 pl-3 pr-2">
                <button
                  type="button"
                  onclick={() => handleHeaderSort("name")}
                  class="flex items-center gap-1.5 hover:text-white transition-colors cursor-pointer group focus:outline-none text-left"
                  title="Sort by augment name"
                >
                  <span>Augment</span>
                  {#if augmentSortKey === "name"}
                    <span class="text-purple-400">
                      {#if augmentDescending}
                        <span class="icon-[heroicons--chevron-down] h-3.5 w-3.5"></span>
                      {:else}
                        <span class="icon-[heroicons--chevron-up] h-3.5 w-3.5"></span>
                      {/if}
                    </span>
                  {:else}
                    <span class="icon-[heroicons--chevron-up-down] h-3.5 w-3.5 opacity-0 group-hover:opacity-40 transition-opacity"></span>
                  {/if}
                </button>
              </th>

              <!-- Tier Column Header -->
              <th scope="col" class="px-2 py-2.5 text-center">
                <button
                  type="button"
                  onclick={() => handleHeaderSort("tier")}
                  class="inline-flex items-center gap-1 hover:text-white transition-colors cursor-pointer group focus:outline-none"
                  title="Sort by augment tier"
                >
                  <span>Tier</span>
                  {#if augmentSortKey === "tier"}
                    <span class="text-purple-400">
                      {#if augmentDescending}
                        <span class="icon-[heroicons--chevron-down] h-3.5 w-3.5"></span>
                      {:else}
                        <span class="icon-[heroicons--chevron-up] h-3.5 w-3.5"></span>
                      {/if}
                    </span>
                  {:else}
                    <span class="icon-[heroicons--chevron-up-down] h-3.5 w-3.5 opacity-0 group-hover:opacity-40 transition-opacity"></span>
                  {/if}
                </button>
              </th>

              <!-- Sample / Pick Rate Column Header -->
              <th scope="col" class="px-2 py-2.5 text-right">
                <button
                  type="button"
                  onclick={() => handleHeaderSort("games")}
                  class="inline-flex items-center gap-1 ml-auto hover:text-white transition-colors cursor-pointer group focus:outline-none"
                  title="Sort by sample games and slot-adjusted pick rate"
                >
                  <span>Sample / Pick %</span>
                  {#if augmentSortKey === "games" || augmentSortKey === "pick_rate"}
                    <span class="text-purple-400">
                      {#if augmentDescending}
                        <span class="icon-[heroicons--chevron-down] h-3.5 w-3.5"></span>
                      {:else}
                        <span class="icon-[heroicons--chevron-up] h-3.5 w-3.5"></span>
                      {/if}
                    </span>
                  {:else}
                    <span class="icon-[heroicons--chevron-up-down] h-3.5 w-3.5 opacity-0 group-hover:opacity-40 transition-opacity"></span>
                  {/if}
                </button>
              </th>

              <!-- Win Rate Column Header -->
              <th scope="col" class="px-2 py-2.5 text-right">
                <button
                  type="button"
                  onclick={() => handleHeaderSort("win_rate")}
                  class="inline-flex items-center gap-1 ml-auto hover:text-white transition-colors cursor-pointer group focus:outline-none"
                  title="Sort by Bayesian win rate"
                >
                  <span>Win Rate</span>
                  {#if augmentSortKey === "win_rate"}
                    <span class="text-purple-400">
                      {#if augmentDescending}
                        <span class="icon-[heroicons--chevron-down] h-3.5 w-3.5"></span>
                      {:else}
                        <span class="icon-[heroicons--chevron-up] h-3.5 w-3.5"></span>
                      {/if}
                    </span>
                  {:else}
                    <span class="icon-[heroicons--chevron-up-down] h-3.5 w-3.5 opacity-0 group-hover:opacity-40 transition-opacity"></span>
                  {/if}
                </button>
              </th>

              <!-- WPA Column Header -->
              <th scope="col" class="py-2.5 pl-2 pr-3 text-right">
                <button
                  type="button"
                  onclick={() => handleHeaderSort("wpa")}
                  class="inline-flex items-center gap-1 ml-auto hover:text-white transition-colors cursor-pointer group focus:outline-none"
                  title="Sort by Bayesian WPA (synergy with champion)"
                >
                  <span>WPA</span>
                  {#if augmentSortKey === "wpa"}
                    <span class="text-purple-400">
                      {#if augmentDescending}
                        <span class="icon-[heroicons--chevron-down] h-3.5 w-3.5"></span>
                      {:else}
                        <span class="icon-[heroicons--chevron-up] h-3.5 w-3.5"></span>
                      {/if}
                    </span>
                  {:else}
                    <span class="icon-[heroicons--chevron-up-down] h-3.5 w-3.5 opacity-0 group-hover:opacity-40 transition-opacity"></span>
                  {/if}
                </button>
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800/60">
            {#each augmentsResponse.items as aug, idx (`aug-table-${aug.id}-${idx}`)}
              {@const hasChampData = aug.games > 0}
              {@const rowId = `aug-row-${aug.id}-${idx}`}
              <tr id={rowId} class="hover:bg-gray-800/40 transition-colors group cursor-help">
                <!-- Augment Icon & Name -->
                <td class="py-2 pl-3 pr-2 flex items-center gap-2.5">
                  <AugmentIcon src={aug.icon_url} name={aug.name} rarity={aug.rarity} class="h-8 w-8 shrink-0" />
                  <div class="flex flex-col min-w-0">
                    <span class="font-bold text-gray-100 truncate group-hover:text-purple-300 transition-colors">
                      {aug.name}
                    </span>
                    <div class="flex items-center gap-1 mt-0.5">
                      {#each aug.categories.slice(0, 2) as cat}
                        <span class="rounded bg-gray-800/80 px-1 py-0.2 text-[9px] text-gray-400 capitalize">
                          {cat}
                        </span>
                      {/each}
                    </div>
                  </div>
                  <GameEntityPopover
                    targetId={rowId}
                    name={aug.name}
                    iconUrl={aug.icon_url}
                    description={aug.tooltip}
                    rarity={aug.rarity}
                    categories={aug.categories}
                    placement="left"
                  />
                </td>

                <!-- Tier Badge -->
                <td class="px-2 py-2 text-center whitespace-nowrap">
                  <span class="rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {aug.rarity === 'prismatic'
                    ? 'bg-purple-950/90 text-[#f2c2ff] border border-[#e78fff]/60'
                    : aug.rarity === 'gold'
                    ? 'bg-amber-950/90 text-[#e3c49f] border border-[#e3c49f]/60'
                    : 'bg-slate-900/90 text-[#a3b9bd] border border-[#a3b9bd]/60'}">
                    {aug.rarity || "Silver"}
                  </span>
                </td>

                <!-- Sample Games & Slot-adjusted Pick Rate -->
                <td class="px-2 py-2 text-right whitespace-nowrap">
                  {#if hasChampData}
                    <span class="font-mono font-medium text-gray-200">
                      {aug.games.toLocaleString()}
                    </span>
                    {#if aug.pick_rate !== undefined}
                      <span
                        class="block text-[10px] text-purple-300 font-mono font-medium"
                        title={`Slot-adjusted pick rate: ${formatPct(aug.pick_rate)} (${aug.games} games out of ${totalGames} games, 4 augment slots per game)`}
                      >
                        {formatPct(aug.pick_rate)}
                      </span>
                    {:else if totalGames > 0}
                      <span class="block text-[10px] text-gray-500 font-mono">
                        {formatPct(aug.games / totalGames)}
                      </span>
                    {/if}
                  {:else}
                    <span
                      class="text-[10px] text-gray-500 font-mono"
                      title={`Global sample: ${aug.global_games.toLocaleString()} games`}
                    >
                      0 <span class="text-gray-600">({aug.global_games.toLocaleString()})</span>
                    </span>
                    {#if aug.global_pick_rate !== undefined && aug.global_pick_rate > 0}
                      <span
                        class="block text-[9px] text-gray-500 font-mono"
                        title={`Global slot-adjusted pick rate: ${formatPct(aug.global_pick_rate)}`}
                      >
                        {formatPct(aug.global_pick_rate)} <span class="text-gray-600">GL</span>
                      </span>
                    {/if}
                  {/if}
                </td>

                <!-- Win Rate -->
                <td class="px-2 py-2 text-right whitespace-nowrap">
                  {#if hasChampData}
                    <span class="font-mono font-bold text-gray-200">
                      {formatPct(aug.win_rate)}
                    </span>
                  {:else}
                    <span class="font-mono text-gray-400 text-[11px]" title="Global ARAM win rate benchmark">
                      {formatPct(aug.global_win_rate)} <span class="text-[9px] text-gray-500">GL</span>
                    </span>
                  {/if}
                </td>

                <!-- WPA Badge -->
                <td class="py-2 pl-2 pr-3 text-right whitespace-nowrap">
                  {#if hasChampData}
                    <span class="rounded px-2 py-0.5 text-xs font-mono font-bold {aug.wpa >= 0
                      ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60 shadow-sm'
                      : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                      {formatWpa(aug.wpa)}
                    </span>
                  {:else}
                    <span class="rounded px-1.5 py-0.5 text-[11px] font-mono text-gray-400 bg-gray-950 border border-gray-800" title="Global WPA benchmark">
                      {formatWpa(aug.global_wpa)} <span class="text-[9px] text-gray-500">GL</span>
                    </span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Pagination Footer -->
      <div class="flex flex-wrap items-center justify-between gap-3 pt-3 border-t border-gray-800/80 text-xs text-gray-400">
        <div class="flex items-center gap-2">
          <span>
            Showing <strong class="text-gray-200">{(augmentPage - 1) * augmentPageSize + 1}</strong>
            –<strong class="text-gray-200">{Math.min(augmentPage * augmentPageSize, augmentsResponse.total_items)}</strong>
            of <strong class="text-gray-200">{augmentsResponse.total_items.toLocaleString()}</strong> augments
          </span>
          <select
            value={augmentPageSize}
            onchange={handlePageSizeChange}
            class="rounded border border-gray-800 bg-gray-950 px-1.5 py-0.5 text-[11px] text-gray-300 focus:outline-none cursor-pointer"
          >
            <option value="10">10 / page</option>
            <option value="15">15 / page</option>
            <option value="25">25 / page</option>
          </select>
        </div>

        <div class="flex items-center gap-1">
          <button
            type="button"
            title="First page"
            disabled={augmentPage <= 1 || loadingAugments}
            onclick={() => handlePageChange(1)}
            class="flex h-7 w-7 items-center justify-center rounded border border-gray-800 bg-gray-950 text-gray-300 hover:border-gray-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            <span class="icon-[heroicons--chevron-double-left] h-3.5 w-3.5"></span>
          </button>
          <button
            type="button"
            title="Previous page"
            disabled={augmentPage <= 1 || loadingAugments}
            onclick={() => handlePageChange(augmentPage - 1)}
            class="flex h-7 w-7 items-center justify-center rounded border border-gray-800 bg-gray-950 text-gray-300 hover:border-gray-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            <span class="icon-[heroicons--chevron-left] h-3.5 w-3.5"></span>
          </button>

          <span class="px-2 text-xs font-mono text-gray-300">
            Page <strong class="text-white">{augmentPage}</strong> / {augmentsResponse.total_pages || 1}
          </span>

          <button
            type="button"
            title="Next page"
            disabled={!augmentsResponse || augmentPage >= augmentsResponse.total_pages || loadingAugments}
            onclick={() => augmentsResponse && handlePageChange(augmentPage + 1)}
            class="flex h-7 w-7 items-center justify-center rounded border border-gray-800 bg-gray-950 text-gray-300 hover:border-gray-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            <span class="icon-[heroicons--chevron-right] h-3.5 w-3.5"></span>
          </button>
          <button
            type="button"
            title="Last page"
            disabled={!augmentsResponse || augmentPage >= augmentsResponse.total_pages || loadingAugments}
            onclick={() => augmentsResponse && handlePageChange(augmentsResponse.total_pages)}
            class="flex h-7 w-7 items-center justify-center rounded border border-gray-800 bg-gray-950 text-gray-300 hover:border-gray-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            <span class="icon-[heroicons--chevron-double-right] h-3.5 w-3.5"></span>
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
