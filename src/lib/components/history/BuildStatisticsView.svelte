<script lang="ts">
  import { untrack } from "svelte";
  import { getCachedStatistics, requestStatistics, type StatisticsKind, type StatisticsScope } from "../../utils/statistics-data";
  import type { ServerStatisticRow } from "../../api/agent";
  import { parseGameDescription } from "../../utils/gameDescription";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";

  let { scope = "personal", hideLowPickrate = false }: { scope?: StatisticsScope; hideLowPickrate?: boolean } = $props();
  let kind = $state<StatisticsKind>("item");
  let rows = $state<ServerStatisticRow[]>([]);
  let selectedId = $state<string | null>(null);
  let search = $state("");
  let debouncedSearch = $state("");
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  type SortKey = "name" | "games" | "pickRate" | "winRate" | "personalWinRate" | "globalWinRate" | "wpa" | "personalWpa" | "globalWpa" | "kda" | "rarity" | "champWpa";
  let sortKey = $state<SortKey>("wpa");
  let sortDescending = $state(true);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let sampleCount = $state(0);
  let baseline = $state(0);
  let requestVersion = 0;

  // Item category toggles (default: only legendary items are shown)
  let showStarters = $state(false);
  let showComponents = $state(false);
  let showBoots = $state(false);

  // Augment tier filter ("all" | "silver" | "gold" | "prismatic")
  type AugmentTierFilter = "all" | "silver" | "gold" | "prismatic";
  let augmentTier = $state<AugmentTierFilter>("all");

  function handleSearchInput(e: Event) {
    const value = (e.currentTarget as HTMLInputElement).value;
    search = value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      debouncedSearch = value;
    }, 200);
  }

  function query() {
    const effectiveSortKey = scope === "compare"
      ? sortKey === "winRate" ? "personalWinRate" : sortKey === "wpa" ? "personalWpa" : sortKey
      : sortKey === "personalWinRate" || sortKey === "globalWinRate" ? "winRate" : sortKey === "personalWpa" || sortKey === "globalWpa" ? "wpa" : sortKey;
    return { scope: scope ?? "personal", kind, search: debouncedSearch, hide_low_pickrate: hideLowPickrate, sort_key: effectiveSortKey, descending: sortDescending, limit: 100 };
  }

  async function load(force = false) {
    const version = ++requestVersion;
    const currentQuery = query();
    const cached = getCachedStatistics(currentQuery);
    if (cached && !force) {
      rows = cached.rows; sampleCount = cached.sample_count; baseline = cached.baseline; loading = false;
    } else if (rows.length) refreshing = true;
    try {
      const response = await requestStatistics(currentQuery, { force });
      if (version !== requestVersion) return;
      rows = response.rows; sampleCount = response.sample_count; baseline = response.baseline; error = null;
    } catch (err) {
      if (version === requestVersion) error = String(err);
    } finally {
      if (version === requestVersion) { loading = false; refreshing = false; }
    }
  }

  function rarityScore(r?: string) {
    const lower = (r ?? "").toLowerCase();
    if (lower === "prismatic") return 3;
    if (lower === "gold") return 2;
    if (lower === "silver") return 1;
    return 0;
  }

  function getTopChampions(row: ServerStatisticRow, count = 3): ServerStatisticRow["champions"] {
    if (!row.champions || row.champions.length === 0) return [];
    return row.champions
      .slice()
      .sort((a, b) => {
        if (b.wpa !== a.wpa) return b.wpa - a.wpa;
        return b.games - a.games;
      })
      .slice(0, count);
  }

  function getBestChampionWpa(row: ServerStatisticRow): number {
    if (!row.champions || row.champions.length === 0) return -999;
    let best = -999;
    for (const c of row.champions) {
      if (c.wpa > best) best = c.wpa;
    }
    return best === -999 ? 0 : best;
  }

  const visibleRows = $derived(
    rows
      .filter((row) => {
        // 1. Augment tier filtering
        if (kind === "augment") {
          if (augmentTier !== "all") {
            const rowRarity = (row.rarity ?? "").toLowerCase();
            if (rowRarity !== augmentTier) return false;
          }
        }

        // 2. Item class filtering (default: only legendary items are shown)
        if (kind === "item") {
          const itemClass = row.item_class ?? "legendary";
          if (itemClass === "boot") return showBoots;
          if (itemClass === "starter") return showStarters;
          if (itemClass === "component") return showComponents;
          if (itemClass === "other") return showStarters;
          return true;
        }

        return true;
      })
      .sort((a, b) => {
        if (sortKey === "rarity") {
          const diff = rarityScore(b.rarity) - rarityScore(a.rarity);
          return sortDescending ? diff : -diff;
        }
        if (sortKey === "champWpa") {
          const diff = getBestChampionWpa(b) - getBestChampionWpa(a);
          return sortDescending ? diff : -diff;
        }
        return 0;
      })
  );

  const selected = $derived(visibleRows.find((row) => row.id === selectedId) ?? visibleRows[0] ?? null);

  function setSort(key: typeof sortKey) {
    if (sortKey === key) sortDescending = !sortDescending;
    else { sortKey = key; sortDescending = key !== "name"; }
    void load();
  }
  function percent(value: number) { return `${Math.round(value * 100)}%`; }
  function formatWpa(value: number) { return `${value >= 0 ? "+" : ""}${(value * 100).toFixed(1)}`; }
  function formatWpaPoints(value: number) { return `${value >= 0 ? "+" : ""}${(value * 100).toFixed(1)}`; }
  function averageKda(row: ServerStatisticRow) { return (row.kills + row.assists) / Math.max(1, row.deaths); }
  function sortIndicator(key: SortKey) {
    const activeKey = scope === "compare"
      ? sortKey === "winRate" ? "personalWinRate" : sortKey === "wpa" ? "personalWpa" : sortKey
      : sortKey === "personalWinRate" || sortKey === "globalWinRate" ? "winRate" : sortKey === "personalWpa" || sortKey === "globalWpa" ? "wpa" : sortKey;
    return activeKey === key ? (sortDescending ? "↓" : "↑") : "";
  }

  function filterAndSortPartnersByKind(
    partners: ServerStatisticRow["partners"],
    targetKind: "item" | "augment",
    parentGames: number,
  ) {
    const list = partners.filter((p) => {
      const isAug = String(p.id).startsWith("augment:") || Boolean(p.rarity);
      return targetKind === "augment" ? isAug : !isAug;
    });
    const minGames = parentGames >= 4 ? 2 : 1;
    let filtered = list.filter((p) => p.games >= minGames);
    if (filtered.length === 0) {
      filtered = list;
    }
    return filtered
      .slice()
      .sort((a, b) => {
        if (b.wpa !== a.wpa) return b.wpa - a.wpa;
        return b.games - a.games;
      })
      .slice(0, 6);
  }

  function cleanDomId(id: string | number): string {
    return `dom-${String(id).replace(/[^a-zA-Z0-9_-]/g, "_")}`;
  }

  $effect(() => {
    void scope;
    void kind;
    void debouncedSearch;
    void hideLowPickrate;
    untrack(() => {
      void load();
    });
  });
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-gray-950 p-4 text-gray-200">
  {#if loading && rows.length === 0}
    <div class="mt-3 flex flex-1 flex-col animate-pulse gap-3"><div class="h-44 rounded-lg border border-gray-800 bg-gray-900/50"></div><div class="flex-1 rounded-lg border border-gray-800 bg-gray-900/50"></div></div>
  {:else if error && rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-red-400">{error}</div>
  {:else if rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-gray-500">No build statistics available.</div>
  {:else}
    <div class="mt-3 flex min-h-0 flex-1 flex-col gap-3">
      <!-- Horizontal Inspector Grid (On Top of the Table) -->
      {#if selected}
        {@const pairedItems = filterAndSortPartnersByKind(selected.partners, "item", selected.games)}
        {@const pairedAugments = filterAndSortPartnersByKind(selected.partners, "augment", selected.games)}
        <section class="rounded-lg border border-gray-800 bg-gray-900/50 p-3 shadow-lg shrink-0 max-h-[300px] overflow-y-auto scrollbar-thin scrollbar-thumb-gray-800">
          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
            <!-- 1. Identity, Tier/Cost & What It Does -->
            <div class="flex flex-col justify-between rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <div>
                <div class="flex items-start justify-between gap-2">
                  <div class="flex items-start gap-2.5 min-w-0">
                    {#if selected.icon_url}
                      <img src={selected.icon_url} alt={selected.name} class="h-10 w-10 rounded-lg border border-gray-700 object-cover shrink-0 shadow-sm" />
                    {/if}
                    <div class="min-w-0 flex-1">
                      <div class="flex flex-wrap items-center gap-1.5">
                        <h2 class="font-bold text-sm text-white truncate" title={selected.name}>{selected.name}</h2>
                        {#if selected.rarity}
                          <span class="shrink-0 rounded px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wider {selected.rarity === 'silver' ? 'bg-slate-700/60 text-slate-300 border border-slate-600/40' : selected.rarity === 'gold' ? 'bg-yellow-950/60 text-yellow-300 border border-yellow-700/40' : 'bg-fuchsia-950/60 text-fuchsia-300 border border-fuchsia-700/40'}">
                            {selected.rarity}
                          </span>
                        {/if}
                      </div>
                      <div class="mt-0.5 flex flex-wrap items-center gap-2 text-[11px]">
                        {#if (selected.price_total ?? 0) > 0}
                          <span class="inline-flex items-center gap-1 font-semibold text-yellow-400">
                            <img src="https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/gameplay/goldcoins.png" alt="gold" class="h-3 w-3 object-contain inline" />
                            {(selected.price_total ?? 0).toLocaleString()}g
                          </span>
                        {/if}
                        {#if selected.item_class}
                          <span class="capitalize text-gray-400 font-medium">{selected.item_class}</span>
                        {/if}
                        {#if selected.categories?.length}
                          <span class="text-gray-500 truncate max-w-[140px]" title={selected.categories.join(", ")}>{selected.categories.slice(0, 2).join(", ")}</span>
                        {/if}
                      </div>
                    </div>
                  </div>
                  <button
                    type="button"
                    class="rounded p-1 text-gray-500 hover:bg-gray-800 hover:text-gray-300 transition-colors"
                    onclick={() => (selectedId = null)}
                    title="Close inspector"
                  >
                    <span class="icon-[heroicons--x-mark] h-4 w-4"></span>
                  </button>
                </div>

                {#if selected.description}
                  <div class="mt-2.5 rounded-md border border-gray-800/90 bg-black/60 p-2.5 shadow-inner">
                    <div class="mb-1 flex items-center gap-1 text-[9px] font-bold uppercase tracking-wider text-purple-400">
                      <span class="icon-[heroicons--sparkles] h-3 w-3"></span>
                      What It Does
                    </div>
                    <div class="text-[11px] leading-relaxed text-gray-300 max-h-[120px] overflow-y-auto scrollbar-thin scrollbar-thumb-gray-800 select-text">
                      {@html parseGameDescription(selected.description)}
                    </div>
                  </div>
                {/if}
              </div>
            </div>

            <!-- 2. Performance Metrics & Top Champions -->
            <div class="flex flex-col rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <dl class="grid grid-cols-2 gap-2 pb-2.5 border-b border-gray-800 text-[11px]">
                <div class="rounded bg-gray-900/60 p-1.5 border border-gray-800/50">
                  <dt class="text-[10px] uppercase text-gray-500">Games · WR</dt>
                  <dd class="font-semibold text-gray-200 truncate">{scope === "compare" ? `${selected.personal_games ?? 0}g · ${percent(selected.personal_win_rate ?? 0)}` : `${selected.games}g · ${percent(selected.win_rate)}`}</dd>
                </div>
                <div class="rounded bg-gray-900/60 p-1.5 border border-gray-800/50">
                  <dt class="text-[10px] uppercase text-gray-500">{scope === "compare" ? "Mine WPA" : "WPA"}</dt>
                  <dd class="font-semibold {scope === "compare" ? (selected.personal_wpa ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400' : selected.wpa >= 0 ? 'text-emerald-400' : 'text-red-400'}">
                    {formatWpaPoints(scope === "compare" ? (selected.personal_wpa ?? 0) : selected.wpa)}
                  </dd>
                </div>
                <div class="rounded bg-gray-900/60 p-1.5 border border-gray-800/50">
                  <dt class="text-[10px] uppercase text-gray-500">KDA Ratio</dt>
                  <dd class="font-semibold text-gray-200">{averageKda(selected).toFixed(2)}</dd>
                </div>
                <div class="rounded bg-gray-900/60 p-1.5 border border-gray-800/50">
                  <dt class="text-[10px] uppercase text-gray-500">Avg Damage</dt>
                  <dd class="font-semibold text-gray-200">{Math.round(selected.damage / Math.max(1, selected.games)).toLocaleString()}</dd>
                </div>
              </dl>

              <div class="mt-2.5 flex-1 min-h-0">
                <h3 class="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-gray-400 flex items-center justify-between">
                  <span>Top Champions</span>
                  <span class="text-[9px] text-gray-500 font-normal">WPA</span>
                </h3>
                <div class="space-y-1 overflow-y-auto max-h-[110px] scrollbar-thin scrollbar-thumb-gray-800 pr-1">
                  {#each selected.champions.slice(0, 4) as champion (champion.id)}
                    <div class="flex items-center justify-between gap-1.5 text-[11px]">
                      <div class="flex min-w-0 items-center gap-1.5">
                        {#if champion.icon_url}
                          <img src={champion.icon_url} alt={champion.name} class="h-4 w-4 shrink-0 rounded-full object-cover" />
                        {/if}
                        <span class="truncate font-medium text-gray-300" title={champion.name}>{champion.name}</span>
                      </div>
                      <span class="shrink-0 text-right text-[10px]">
                        <span class="text-gray-400 mr-1">{champion.games}g</span>
                        <span class={champion.wpa >= 0 ? "text-emerald-400 font-medium" : "text-red-400 font-medium"}>{formatWpa(champion.wpa)}</span>
                      </span>
                    </div>
                  {:else}
                    <span class="text-gray-600 text-[11px] italic">No champion data</span>
                  {/each}
                </div>
              </div>
            </div>

            <!-- 3. Often Paired Items -->
            <div class="flex flex-col rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <h3 class="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-amber-400">
                <span class="icon-[heroicons--archive-box] h-3.5 w-3.5"></span>
                Often Paired Items
              </h3>
              <div class="space-y-1.5 overflow-y-auto max-h-[160px] scrollbar-thin scrollbar-thumb-gray-800 pr-1 flex-1">
                {#each pairedItems as partner (partner.id)}
                  {@const pDomId = cleanDomId(`item-${partner.id}`)}
                  <div id={pDomId} class="flex items-center justify-between gap-1.5 text-[11px] hover:bg-gray-800/50 p-1 rounded transition-colors cursor-help">
                    <div class="flex min-w-0 items-center gap-1.5">
                      {#if partner.icon_url}
                        <img src={partner.icon_url} alt={partner.name} class="h-4 w-4 shrink-0 rounded object-cover" />
                      {/if}
                      <span class="truncate font-medium text-gray-300" title={partner.name}>{partner.name}</span>
                    </div>
                    <span class="shrink-0 text-right text-[10px]">
                      <span class="text-gray-500 mr-1">{partner.games}g</span>
                      <span class={partner.wpa >= 0 ? "text-emerald-400 font-medium" : "text-red-400 font-medium"}>{formatWpa(partner.wpa)}</span>
                    </span>
                  </div>
                  <GameEntityPopover
                    targetId={pDomId}
                    name={partner.name}
                    iconUrl={partner.icon_url}
                    description={partner.description}
                    itemClass={partner.item_class}
                    priceTotal={partner.price_total}
                    placement="bottom"
                  />
                {:else}
                  <div class="flex h-full items-center justify-center text-center text-gray-600 text-[11px] italic py-4">
                    No item synergies found
                  </div>
                {/each}
              </div>
            </div>

            <!-- 4. Often Paired Augments -->
            <div class="flex flex-col rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <h3 class="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-purple-400">
                <span class="icon-[heroicons--sparkles] h-3.5 w-3.5"></span>
                Often Paired Augments
              </h3>
              <div class="space-y-1.5 overflow-y-auto max-h-[160px] scrollbar-thin scrollbar-thumb-gray-800 pr-1 flex-1">
                {#each pairedAugments as partner (partner.id)}
                  {@const pDomId = cleanDomId(`aug-${partner.id}`)}
                  <div id={pDomId} class="flex items-center justify-between gap-1.5 text-[11px] hover:bg-gray-800/50 p-1 rounded transition-colors cursor-help">
                    <div class="flex min-w-0 items-center gap-1.5">
                      {#if partner.icon_url}
                        <img src={partner.icon_url} alt={partner.name} class="h-4 w-4 shrink-0 rounded object-cover" />
                      {/if}
                      <span class="truncate font-medium text-gray-300" title={partner.name}>{partner.name}</span>
                      {#if partner.rarity}
                        <span class="shrink-0 rounded px-1 py-0.2 text-[8px] font-bold uppercase tracking-wider {partner.rarity === 'silver' ? 'bg-slate-700/60 text-slate-300' : partner.rarity === 'gold' ? 'bg-yellow-950/60 text-yellow-300' : 'bg-fuchsia-950/60 text-fuchsia-300'}">
                          {partner.rarity.slice(0, 1)}
                        </span>
                      {/if}
                    </div>
                    <span class="shrink-0 text-right text-[10px]">
                      <span class="text-gray-500 mr-1">{partner.games}g</span>
                      <span class={partner.wpa >= 0 ? "text-emerald-400 font-medium" : "text-red-400 font-medium"}>{formatWpa(partner.wpa)}</span>
                    </span>
                  </div>
                  <GameEntityPopover
                    targetId={pDomId}
                    name={partner.name}
                    iconUrl={partner.icon_url}
                    description={partner.description}
                    rarity={partner.rarity}
                    placement="bottom"
                  />
                {:else}
                  <div class="flex h-full items-center justify-center text-center text-gray-600 text-[11px] italic py-4">
                    No augment synergies found
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </section>
      {/if}

      <!-- Main Table Section (Spanning 100% Width) -->
      <section class="min-h-0 flex-1 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
        <div class="sticky top-0 z-10 flex flex-wrap items-center justify-between gap-2 border-b border-gray-800 bg-gray-900 px-3 py-2">
          <div><h2 class="text-sm font-semibold text-white">Build performance</h2><p class="text-[11px] text-gray-500">{scope === "compare" ? "Compare your performance with the database" : `WPA = win-rate delta · baseline ${percent(baseline)}`} · {sampleCount} games {#if refreshing}· refreshing{/if}</p></div>
          <div class="flex flex-wrap items-center gap-2">
            <input
              value={search}
              oninput={handleSearchInput}
              type="search"
              placeholder="Search builds..."
              class="w-36 rounded-md border border-gray-700 bg-gray-950 px-2 py-1.5 text-[11px] text-gray-100 outline-none placeholder:text-gray-600 focus:border-purple-500"
            />

            <!-- Item class toggles (when viewing items) -->
            {#if kind === "item"}
              <div class="flex items-center gap-1 rounded-lg border border-gray-800 bg-gray-950 p-0.5 text-[11px]">
                <button
                  type="button"
                  title="Toggle starter items (e.g. Doran's, Guardian's)"
                  class="rounded px-2 py-1 transition-colors {showStarters ? 'bg-amber-600/80 text-white font-medium' : 'text-gray-400 hover:text-white'}"
                  onclick={() => (showStarters = !showStarters)}
                >
                  Starters
                </button>
                <button
                  type="button"
                  title="Toggle component items (e.g. B.F. Sword, Sheen)"
                  class="rounded px-2 py-1 transition-colors {showComponents ? 'bg-sky-600/80 text-white font-medium' : 'text-gray-400 hover:text-white'}"
                  onclick={() => (showComponents = !showComponents)}
                >
                  Components
                </button>
                <button
                  type="button"
                  title="Toggle boots (e.g. Berserker's, Plated Steelcaps)"
                  class="rounded px-2 py-1 transition-colors {showBoots ? 'bg-emerald-600/80 text-white font-medium' : 'text-gray-400 hover:text-white'}"
                  onclick={() => (showBoots = !showBoots)}
                >
                  Boots
                </button>
              </div>
            {/if}

            <!-- Augment tier filters (when viewing augments) -->
            {#if kind === "augment"}
              <div class="flex items-center gap-0.5 rounded-lg border border-gray-800 bg-gray-950 p-0.5 text-[11px]">
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'all' ? 'bg-purple-600 text-white font-medium' : 'text-gray-400 hover:text-white'}"
                  onclick={() => (augmentTier = 'all')}
                >
                  All
                </button>
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'silver' ? 'bg-slate-300 text-gray-900 font-semibold shadow-sm' : 'text-slate-400 hover:text-slate-200'}"
                  onclick={() => (augmentTier = 'silver')}
                >
                  Silver
                </button>
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'gold' ? 'bg-yellow-500 text-gray-950 font-semibold shadow-sm' : 'text-yellow-400 hover:text-yellow-200'}"
                  onclick={() => (augmentTier = 'gold')}
                >
                  Gold
                </button>
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'prismatic' ? 'bg-gradient-to-r from-fuchsia-500 to-cyan-400 text-white font-semibold shadow-sm' : 'text-fuchsia-400 hover:text-fuchsia-200'}"
                  onclick={() => (augmentTier = 'prismatic')}
                >
                  Prismatic
                </button>
              </div>
            {/if}

            <div class="flex rounded-lg border border-gray-700 bg-gray-950 p-0.5 text-[11px]">
              {#each [["item", "Items"], ["augment", "Augments"], ["combo", "Combos"]] as option}
                <button
                  type="button"
                  class="rounded-md px-2 py-1 {kind === option[0] ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}"
                  onclick={() => { kind = option[0] as StatisticsKind; selectedId = null; }}
                >
                  {option[1]}
                </button>
              {/each}
            </div>
          </div>
        </div>
        <table class="w-full text-left text-xs">
          <thead class="sticky top-[57px] bg-gray-900 text-[10px] uppercase tracking-wider text-gray-500">
            <tr>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("name")}>Build {sortIndicator("name")}</th>
              {#if kind === "augment"}
                <th class="cursor-pointer px-3 py-2" onclick={() => setSort("rarity")}>Tier {sortIndicator("rarity")}</th>
              {/if}
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("champWpa")} title="Champions benefiting most from this build (by WPA)">
                Best Champs {sortIndicator("champWpa")}
              </th>
              <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("games")}>Games {sortIndicator("games")}</th>
              <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("pickRate")}>Pick rate {sortIndicator("pickRate")}</th>
              <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWinRate" : "winRate")}>{scope === "compare" ? "Mine WR" : "WR"} {sortIndicator(scope === "compare" ? "personalWinRate" : "winRate")}</th>
              {#if scope === "compare"}
                <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWinRate")}>Global WR {sortIndicator("globalWinRate")}</th>
              {/if}
              <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWpa" : "wpa")}>{scope === "compare" ? "Mine WPA" : "WPA"} {sortIndicator(scope === "compare" ? "personalWpa" : "wpa")}</th>
              {#if scope === "compare"}
                <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWpa")}>Global WPA {sortIndicator("globalWpa")}</th>
              {/if}
              <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("kda")}>KDA {sortIndicator("kda")}</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each visibleRows as row (row.id)}
              {@const localWpa = scope === "compare" ? row.personal_wpa ?? 0 : row.wpa}
              {@const globalWpa = row.global_wpa ?? row.wpa}
              {@const domId = cleanDomId(row.id)}
              {@const topChamps = getTopChampions(row, 3)}
              <tr
                id={domId}
                class="cursor-pointer hover:bg-gray-800/60 {selected?.id === row.id ? 'bg-purple-950/40' : ''}"
                onclick={() => (selectedId = selectedId === row.id ? null : row.id)}
              >
                <td class="flex max-w-[340px] items-center gap-2 px-3 py-2">
                  {#if row.icon_url}
                    <img src={row.icon_url} alt={row.name} class="h-7 w-7 rounded object-cover shrink-0" />
                  {/if}
                  <span class="truncate font-medium text-gray-200">{row.name}</span>
                </td>
                {#if kind === "augment"}
                  <td class="px-3 py-2">
                    {#if row.rarity}
                      <span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {row.rarity === 'silver' ? 'bg-slate-700/60 text-slate-300 border border-slate-600/40' : row.rarity === 'gold' ? 'bg-yellow-950/60 text-yellow-300 border border-yellow-700/40' : 'bg-fuchsia-950/60 text-fuchsia-300 border border-fuchsia-700/40'}">
                        {row.rarity}
                      </span>
                    {:else}
                      <span class="text-gray-600">—</span>
                    {/if}
                  </td>
                {/if}
                <!-- Best Champions Preview (Card Deck Hand) -->
                <td class="px-3 py-2">
                  {#if topChamps.length > 0}
                    <div class="flex items-center gap-2">
                      <div class="flex items-center -space-x-2.5 hover:space-x-0.5 transition-all duration-200 py-0.5">
                        {#each topChamps as champ, i (champ.id)}
                          <div
                            class="relative rounded-full transition-transform hover:scale-125 hover:z-30 cursor-pointer"
                            style="z-index: {10 + i};"
                            title={`${champ.name}: ${champ.wpa >= 0 ? '+' : ''}${(champ.wpa * 100).toFixed(1)}% WPA (${champ.games} games, ${percent(champ.win_rate)} WR)`}
                          >
                            {#if champ.icon_url}
                              <img
                                src={champ.icon_url}
                                alt={champ.name}
                                class="h-6 w-6 rounded-full border-2 border-gray-900 object-cover shadow-sm bg-gray-950"
                              />
                            {:else}
                              <div class="h-6 w-6 rounded-full border-2 border-gray-900 bg-gray-800 text-[8px] flex items-center justify-center font-bold text-gray-200">
                                {champ.name.slice(0, 1)}
                              </div>
                            {/if}
                          </div>
                        {/each}
                      </div>
                      {#if topChamps[0]}
                        <span class="text-[10px] font-semibold {topChamps[0].wpa >= 0 ? 'text-emerald-400' : 'text-red-400'}">
                          {formatWpaPoints(topChamps[0].wpa)}
                        </span>
                      {/if}
                    </div>
                  {:else}
                    <span class="text-gray-600 text-[11px]">—</span>
                  {/if}
                </td>
                <td class="px-3 py-2 text-right text-gray-400">{scope === "compare" ? `${row.personal_games ?? 0}/${row.global_observed ? row.global_games ?? row.games : "—"}` : row.games}</td>
                <td class="px-3 py-2 text-right text-gray-400">{percent(row.pick_rate)}</td>
                <td class="px-3 py-2 text-right">{scope === "compare" ? (row.personal_games ? percent(row.personal_win_rate ?? 0) : "—") : percent(row.win_rate)}</td>
                {#if scope === "compare"}
                  <td class="px-3 py-2 text-right">{row.global_observed ? percent(row.global_win_rate ?? row.win_rate) : "—"}</td>
                {/if}
                <td class="px-3 py-2 text-right {localWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{scope === "compare" && !row.personal_games ? "—" : formatWpaPoints(localWpa)}</td>
                {#if scope === "compare"}
                  <td class="px-3 py-2 text-right {globalWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{row.global_observed ? formatWpaPoints(globalWpa) : "—"}</td>
                {/if}
                <td class="px-3 py-2 text-right text-gray-400">{averageKda(row).toFixed(2)}</td>
              </tr>
              <GameEntityPopover
                targetId={domId}
                name={row.name}
                iconUrl={row.icon_url}
                description={row.description}
                rarity={row.rarity}
                itemClass={row.item_class}
                priceTotal={row.price_total}
                categories={row.categories}
                placement="right"
              />
            {/each}
          </tbody>
        </table>
      </section>
    </div>
  {/if}
</div>
