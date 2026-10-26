<script lang="ts">
  import { untrack } from "svelte";
  import { getCachedStatistics, requestStatistics, type StatisticsKind, type StatisticsScope } from "../../utils/statistics-data";
  import type { ServerStatisticRow } from "../../api/agent";
  import { parseGameDescription } from "../../utils/gameDescription";
  import * as dom from "@floating-ui/dom";
  import { createPopoverMiddlewares } from "../../utils/popoverPlacement";
  import AugmentIcon from "../common/AugmentIcon.svelte";

  let { scope = "personal", hideLowPickrate = false, refreshKey = 0 }: { scope?: StatisticsScope; hideLowPickrate?: boolean; refreshKey?: number } = $props();
  let lastRefreshKey = 0;
  let kind = $state<StatisticsKind>("item");
  let rows = $state<ServerStatisticRow[]>([]);
  let selectedId = $state<string | null>(null);
  let search = $state("");
  let debouncedSearch = $state("");
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  type SortKey = "name" | "games" | "pickRate" | "winRate" | "personalWinRate" | "globalWinRate" | "wpa" | "personalWpa" | "globalWpa" | "kda" | "rarity" | "champWpa";
  const cachedSortKey = localStorage.getItem("lcu-builds-sort-key") as SortKey | null;
  const cachedSortDesc = localStorage.getItem("lcu-builds-sort-desc");
  let sortKey = $state<SortKey>(cachedSortKey ?? "globalWpa");
  let sortDescending = $state(cachedSortDesc !== null ? cachedSortDesc === "true" : true);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let sampleCount = $state(0);
  let baseline = $state(0);
  const gamesWon = $derived(Math.round(sampleCount * baseline));
  const overallWpa = $derived(baseline - 0.5);
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
    const effectiveScope = kind === "combo" && scope === "personal" ? "global" : (scope ?? "personal");
    const effectiveSortKey = effectiveScope === "compare"
      ? sortKey === "winRate" ? "personalWinRate" : sortKey === "wpa" ? "personalWpa" : sortKey
      : sortKey === "personalWinRate" || sortKey === "globalWinRate" ? "winRate" : sortKey === "personalWpa" || sortKey === "globalWpa" ? "wpa" : sortKey;
    return { scope: effectiveScope, kind, hide_low_pickrate: hideLowPickrate, sort_key: effectiveSortKey, descending: sortDescending, limit: 1000 };
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

  function getAugmentTitleColor(r?: string) {
    const lower = (r ?? "").toLowerCase();
    if (lower.includes("prismatic")) return "text-[#f2c2ff]";
    if (lower.includes("gold")) return "text-[#e3c49f]";
    if (lower.includes("silver")) return "text-[#a3b9bd]";
    return "text-gray-200";
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

        // 3. Search query matching across row name and components
        const q = debouncedSearch.trim().toLowerCase();
        if (q) {
          const matchesName = row.name.toLowerCase().includes(q);
          const matchesComponent = row.components?.some((c) => c.name.toLowerCase().includes(q));
          if (!matchesName && !matchesComponent) return false;
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
    localStorage.setItem("lcu-builds-sort-key", sortKey);
    localStorage.setItem("lcu-builds-sort-desc", String(sortDescending));
    void load();
  }

  function switchKind(newKind: StatisticsKind) {
    if (kind === newKind) return;
    kind = newKind;
    selectedId = null;
    const currentQuery = query();
    const cached = getCachedStatistics(currentQuery);
    if (cached) {
      rows = cached.rows;
      sampleCount = cached.sample_count;
      baseline = cached.baseline;
      loading = false;
      refreshing = false;
    } else {
      rows = [];
      loading = true;
      refreshing = false;
    }
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
        if (b.pick_rate !== a.pick_rate) return b.pick_rate - a.pick_rate;
        if (b.games !== a.games) return b.games - a.games;
        return b.wpa - a.wpa;
      })
      .slice(0, 6);
  }

  function sortBestMatchChampions(
    champions: ServerStatisticRow["champions"],
  ): ServerStatisticRow["champions"] {
    if (!champions || champions.length === 0) return [];
    return champions
      .slice()
      .sort((a, b) => {
        if (b.pick_rate !== a.pick_rate) return b.pick_rate - a.pick_rate;
        if (b.games !== a.games) return b.games - a.games;
        return b.wpa - a.wpa;
      });
  }

  interface ActiveHoverEntity {
    name: string;
    iconUrl?: string;
    description?: string;
    rarity?: string;
    itemClass?: string;
    priceTotal?: number;
    categories?: string[];
  }

  let activeHover = $state<ActiveHoverEntity | null>(null);
  let popoverPos = $state<{ x: number; y: number }>({ x: 0, y: 0 });
  let popoverEl: HTMLElement | null = $state(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  function triggerHover(e: MouseEvent, entity: ActiveHoverEntity, placement: "top" | "bottom" = "top") {
    if (hoverTimer) clearTimeout(hoverTimer);
    const target = e.currentTarget as HTMLElement;
    hoverTimer = setTimeout(async () => {
      activeHover = entity;
      await new Promise((r) => requestAnimationFrame(r));
      if (popoverEl && target) {
        const middlewares = createPopoverMiddlewares(placement, 8, 12);
        const { x, y } = await dom.computePosition(target, popoverEl, {
          placement,
          middleware: middlewares,
        });
        popoverPos = { x, y };
      }
    }, 180);
  }

  function clearHover() {
    if (hoverTimer) clearTimeout(hoverTimer);
    activeHover = null;
  }

  let displayLimit = $state(35);
  const renderedRows = $derived(visibleRows.slice(0, displayLimit));

  function handleTableScroll(e: UIEvent) {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 100) {
      if (displayLimit < visibleRows.length) {
        displayLimit = Math.min(displayLimit + 35, visibleRows.length);
      }
    }
  }

  $effect(() => {
    void scope;
    void kind;
    void hideLowPickrate;
    void sortKey;
    const forceRefresh = refreshKey !== lastRefreshKey;
    lastRefreshKey = refreshKey;
    displayLimit = 35;
    clearHover();
    untrack(() => {
      void load(forceRefresh);
    });
  });

  $effect(() => {
    void debouncedSearch;
    void augmentTier;
    void showStarters;
    void showComponents;
    void showBoots;
    displayLimit = 35;
    clearHover();
  });
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-gray-950 p-4 text-gray-200">
  <div class="mt-3 flex min-h-0 flex-1 flex-col gap-3">
      <!-- Horizontal Inspector Grid (On Top of the Table) -->
      {#if selected}
        {@const pairedChampions = sortBestMatchChampions(selected.champions)}
        {@const pairedItems = filterAndSortPartnersByKind(selected.partners, "item", selected.games)}
        {@const pairedAugments = filterAndSortPartnersByKind(selected.partners, "augment", selected.games)}
        <section class="rounded-lg border border-gray-800 bg-gray-900/50 p-3 shadow-lg shrink-0 max-h-[300px] overflow-y-auto scrollbar-thin scrollbar-thumb-gray-800">
          <div class="flex flex-col lg:flex-row gap-3">
            <!-- 1. Selected Build Details & Description (Reduced width by 4rem) -->
            <div class="w-full lg:w-[calc(33.333%-4rem)] shrink-0 flex flex-col justify-between rounded-lg border border-gray-800/80 bg-gray-950/60 p-2.5">
              <div>
                <div class="flex items-start gap-2.5 min-w-0">
                  {#if kind === "combo" && selected.components && selected.components.length > 0}
                    <div class="flex items-center gap-1.5 shrink-0 p-1.5 bg-gray-900 rounded-lg border border-gray-800">
                      {#each selected.components as comp, cIdx}
                        {#if cIdx > 0}
                          <span class="text-gray-500 text-xs font-bold select-none">+</span>
                        {/if}
                        {#if comp.kind === "champion"}
                          {#if comp.icon_url}
                            <img src={comp.icon_url} alt={comp.name} class="h-8 w-8 rounded-full object-cover border border-cyan-700/60" />
                          {:else}
                            <div class="h-8 w-8 rounded-full bg-cyan-950 border border-cyan-700 flex items-center justify-center text-[10px] font-bold text-cyan-300">
                              {comp.name.slice(0, 1)}
                            </div>
                          {/if}
                        {:else if comp.kind === "augment"}
                          <AugmentIcon src={comp.icon_url} name={comp.name} rarity={comp.rarity} class="h-8 w-8" />
                        {:else}
                          {#if comp.icon_url}
                            <img src={comp.icon_url} alt={comp.name} class="h-8 w-8 rounded-md object-cover border border-gray-700" />
                          {:else}
                            <div class="h-8 w-8 rounded-md bg-gray-800 border border-gray-700 flex items-center justify-center text-[10px] font-bold text-gray-300">
                              {comp.name.slice(0, 1)}
                            </div>
                          {/if}
                        {/if}
                      {/each}
                    </div>
                  {:else if selected.icon_url}
                    {#if kind === "augment" || selected.rarity}
                      <AugmentIcon src={selected.icon_url} name={selected.name} rarity={selected.rarity} class="h-10 w-10" roundedClass="rounded-lg" />
                    {:else}
                      <img src={selected.icon_url} alt={selected.name} class="h-10 w-10 rounded-lg border border-gray-700 object-cover shrink-0 shadow-sm" />
                    {/if}
                  {/if}
                  <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-1.5">
                      <h2 class="font-bold text-sm {selected.rarity ? getAugmentTitleColor(selected.rarity) : 'text-white'} truncate" title={selected.name}>{selected.name}</h2>
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
                        <span class="text-gray-500 truncate max-w-[120px]" title={selected.categories.join(", ")}>{selected.categories.slice(0, 2).join(", ")}</span>
                      {/if}
                    </div>
                  </div>
                </div>

                <!-- Description directly inside the card (no nested box/container) -->
                {#if kind === "combo" && selected.components}
                  <div class="mt-2 text-xs text-gray-300 space-y-1.5 max-h-[125px] overflow-y-auto scrollbar-thin scrollbar-thumb-gray-800 select-text">
                    {#each selected.components as comp}
                      <div class="flex items-center gap-2">
                        <span class="text-[9px] uppercase font-bold text-gray-400 bg-gray-800/80 px-1 py-0.5 rounded border border-gray-700/60 shrink-0">{comp.kind}</span>
                        <span class="font-medium text-gray-200 truncate">{comp.name}</span>
                        {#if comp.rarity}
                          <span class="text-[9px] font-bold uppercase tracking-wider {getAugmentTitleColor(comp.rarity)}">({comp.rarity})</span>
                        {/if}
                        {#if comp.price_total}
                          <span class="text-[10px] font-semibold text-yellow-400">({comp.price_total}g)</span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {:else if selected.description}
                  <div class="mt-2 text-xs leading-relaxed text-gray-300 max-h-[125px] overflow-y-auto scrollbar-thin scrollbar-thumb-gray-800 select-text">
                    {@html parseGameDescription(selected.description)}
                  </div>
                {/if}
              </div>

              <!-- Info (WR, WPA, etc.) underneath item description -->
              <div class="mt-2.5 pt-2 border-t border-gray-800/80 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
                <div class="flex items-center gap-1">
                  <span class="text-gray-500 text-[10px] uppercase font-semibold">Record</span>
                  <span class="font-semibold text-gray-200">
                    {scope === "compare" ? `${selected.personal_games ?? 0} · ${percent(selected.personal_win_rate ?? 0)} WR` : `${selected.games} · ${percent(selected.win_rate)} WR`}
                  </span>
                </div>
                <div class="flex items-center gap-1">
                  <span class="text-gray-500 text-[10px] uppercase font-semibold">{scope === "compare" ? "Mine WPA" : "WPA"}</span>
                  <span class="font-bold {scope === "compare" ? (selected.personal_wpa ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400' : selected.wpa >= 0 ? 'text-emerald-400' : 'text-red-400'}">
                    {formatWpaPoints(scope === "compare" ? (selected.personal_wpa ?? 0) : selected.wpa)}
                  </span>
                </div>
                <div class="flex items-center gap-1">
                  <span class="text-gray-500 text-[10px] uppercase font-semibold">KDA</span>
                  <span class="font-semibold text-gray-200">{averageKda(selected).toFixed(2)}</span>
                </div>
                <div class="flex items-center gap-1">
                  <span class="text-gray-500 text-[10px] uppercase font-semibold">Avg Dmg</span>
                  <span class="font-semibold text-gray-200">{Math.round(selected.damage / Math.max(1, selected.games)).toLocaleString()}</span>
                </div>
              </div>
            </div>

            <!-- 2. "Best Matches" Combined Card (Champions, Items, Augments - Expanded, no gap between columns) -->
            <div class="w-full lg:flex-1 min-w-0 flex flex-col rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <div class="flex items-center justify-between pb-2 border-b border-gray-800/80">
                <div class="flex items-center gap-1.5">
                  <span class="icon-[heroicons--sparkles] h-4 w-4 text-purple-400"></span>
                  <h3 class="text-xs font-bold uppercase tracking-wider text-white">Best Matches</h3>
                </div>
                <span class="text-[10px] text-gray-500">Most commonly picked</span>
              </div>

              <div class="grid grid-cols-1 sm:grid-cols-3 gap-0 pt-2 flex-1 min-h-0 divide-y sm:divide-y-0 sm:divide-x divide-gray-800/60">
                <!-- Champions Column -->
                <div class="flex flex-col min-h-0 sm:pl-0 sm:pr-0">
                  <h4 class="text-[10px] font-semibold uppercase tracking-wider text-cyan-400 mb-1.5 flex items-center gap-1">
                    <span class="icon-[heroicons--user] h-3 w-3"></span> Champions
                  </h4>
                  <div class="space-y-1.5 overflow-y-auto max-h-[180px] scrollbar-thin scrollbar-thumb-gray-800 pr-0 flex-1">
                    {#each pairedChampions.slice(0, 5) as champion (champion.id)}
                      <div class="flex items-center gap-2 p-1 rounded-md hover:bg-gray-800/50 transition-colors">
                        {#if champion.icon_url}
                          <img src={champion.icon_url} alt={champion.name} class="h-7 w-7 shrink-0 rounded-full object-cover border border-gray-700 shadow-sm" />
                        {:else}
                          <div class="h-7 w-7 shrink-0 rounded-full bg-gray-800 border border-gray-700 flex items-center justify-center text-[10px] font-bold text-gray-300">
                            {champion.name.slice(0, 1)}
                          </div>
                        {/if}
                        <div class="min-w-0 flex-1">
                          <p class="truncate font-medium text-xs text-gray-200" title={champion.name}>{champion.name}</p>
                          <p class="flex items-center flex-wrap gap-1 text-[10px] text-gray-400 mt-0.5 leading-none">
                            <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-400 shrink-0">WR</span>
                            <span class="text-gray-400 font-medium">{Math.round(champion.games * champion.win_rate)}/{champion.games} ({percent(champion.win_rate)})</span>
                            {#if champion.wpa !== undefined}
                              <span class="text-gray-600">|</span>
                              <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-300 shrink-0">WPA</span>
                              <span class="{champion.wpa >= 0 ? 'text-emerald-400' : 'text-red-400'} font-medium">{formatWpa(champion.wpa)}</span>
                            {/if}
                          </p>
                        </div>
                      </div>
                    {:else}
                      <span class="text-gray-600 text-[11px] italic py-2">No champion data</span>
                    {/each}
                  </div>
                </div>

                <!-- Items Column -->
                <div class="flex flex-col min-h-0 sm:pl-1.5 sm:pr-0">
                  <h4 class="text-[10px] font-semibold uppercase tracking-wider text-amber-400 mb-1.5 flex items-center gap-1">
                    <span class="icon-[heroicons--archive-box] h-3 w-3"></span> Items
                  </h4>
                  <div class="space-y-1.5 overflow-y-auto max-h-[180px] scrollbar-thin scrollbar-thumb-gray-800 pr-0 flex-1">
                    {#each pairedItems.slice(0, 5) as partner (partner.id)}
                      <div
                        class="flex items-center gap-2 p-1 rounded-md hover:bg-gray-800/50 transition-colors cursor-help"
                        onmouseenter={(e) => triggerHover(e, { name: partner.name, iconUrl: partner.icon_url, description: partner.description, itemClass: partner.item_class, priceTotal: partner.price_total }, "bottom")}
                        onmouseleave={clearHover}
                      >
                        {#if partner.icon_url}
                          <img src={partner.icon_url} alt={partner.name} class="h-7 w-7 shrink-0 rounded-md object-cover border border-gray-700 shadow-sm" />
                        {:else}
                          <div class="h-7 w-7 shrink-0 rounded-md bg-gray-800 border border-gray-700 flex items-center justify-center text-[10px] font-bold text-gray-300">
                            {partner.name.slice(0, 1)}
                          </div>
                        {/if}
                        <div class="min-w-0 flex-1">
                          <p class="truncate font-medium text-xs text-gray-200" title={partner.name}>{partner.name}</p>
                          <p class="flex items-center flex-wrap gap-1 text-[10px] text-gray-400 mt-0.5 leading-none">
                            <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-400 shrink-0">WR</span>
                            <span class="text-gray-400 font-medium">{Math.round(partner.games * partner.win_rate)}/{partner.games} ({percent(partner.win_rate)})</span>
                            {#if partner.wpa !== undefined}
                              <span class="text-gray-600">|</span>
                              <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-300 shrink-0">WPA</span>
                              <span class="{partner.wpa >= 0 ? 'text-emerald-400' : 'text-red-400'} font-medium">{formatWpa(partner.wpa)}</span>
                            {/if}
                          </p>
                        </div>
                      </div>
                    {:else}
                      <span class="text-gray-600 text-[11px] italic py-2">No item matches</span>
                    {/each}
                  </div>
                </div>

                <!-- Augments Column -->
                <div class="flex flex-col min-h-0 sm:pl-1.5 sm:pr-0">
                  <h4 class="text-[10px] font-semibold uppercase tracking-wider text-purple-400 mb-1.5 flex items-center gap-1">
                    <span class="icon-[heroicons--sparkles] h-3 w-3"></span> Augments
                  </h4>
                  <div class="space-y-1.5 overflow-y-auto max-h-[180px] scrollbar-thin scrollbar-thumb-gray-800 pr-0 flex-1">
                    {#each pairedAugments.slice(0, 5) as partner (partner.id)}
                      <div
                        class="flex items-center gap-2 p-1 rounded-md hover:bg-gray-800/50 transition-colors cursor-help"
                        onmouseenter={(e) => triggerHover(e, { name: partner.name, iconUrl: partner.icon_url, description: partner.description, rarity: partner.rarity }, "bottom")}
                        onmouseleave={clearHover}
                      >
                        <AugmentIcon src={partner.icon_url} name={partner.name} rarity={partner.rarity} class="h-7 w-7" />
                        <div class="min-w-0 flex-1">
                          <p class="truncate font-medium text-xs {getAugmentTitleColor(partner.rarity)}" title={partner.name}>{partner.name}</p>
                          <p class="flex items-center flex-wrap gap-1 text-[10px] text-gray-400 mt-0.5 leading-none">
                            <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-400 shrink-0">WR</span>
                            <span class="text-gray-400 font-medium">{Math.round(partner.games * partner.win_rate)}/{partner.games} ({percent(partner.win_rate)})</span>
                            {#if partner.wpa !== undefined}
                              <span class="text-gray-600">|</span>
                              <span class="rounded bg-gray-800/90 border border-gray-700/60 px-1 py-0.5 text-[8px] font-bold uppercase tracking-wider text-gray-300 shrink-0">WPA</span>
                              <span class="{partner.wpa >= 0 ? 'text-emerald-400' : 'text-red-400'} font-medium">{formatWpa(partner.wpa)}</span>
                            {/if}
                          </p>
                        </div>
                      </div>
                    {:else}
                      <span class="text-gray-600 text-[11px] italic py-2">No augment matches</span>
                    {/each}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>
      {:else if loading && rows.length === 0}
        <!-- Skeleton Inspector Card matching the exact shape -->
        <section class="rounded-lg border border-gray-800 bg-gray-900/50 p-3 shadow-lg shrink-0 max-h-[300px] overflow-hidden animate-pulse">
          <div class="flex flex-col lg:flex-row gap-3">
            <!-- 1. Build Details Skeleton -->
            <div class="w-full lg:w-[calc(33.333%-4rem)] shrink-0 flex flex-col justify-between rounded-lg border border-gray-800/80 bg-gray-950/60 p-2.5 min-h-[140px]">
              <div>
                <div class="flex items-start gap-2.5">
                  <div class="h-10 w-10 rounded-lg bg-gray-800 shrink-0"></div>
                  <div class="min-w-0 flex-1 space-y-2">
                    <div class="h-4 w-28 rounded bg-gray-800"></div>
                    <div class="h-3 w-16 rounded bg-gray-800/70"></div>
                  </div>
                </div>
                <div class="mt-3 space-y-1.5">
                  <div class="h-3 w-full rounded bg-gray-800/50"></div>
                  <div class="h-3 w-3/4 rounded bg-gray-800/40"></div>
                </div>
              </div>
              <div class="mt-3 flex items-center justify-between border-t border-gray-800/80 pt-2">
                <div class="h-4 w-16 rounded bg-gray-800/60"></div>
                <div class="h-4 w-14 rounded bg-gray-800/60"></div>
                <div class="h-4 w-14 rounded bg-gray-800/60"></div>
                <div class="h-4 w-16 rounded bg-gray-800/60"></div>
              </div>
            </div>

            <!-- 2. Best Matches Skeleton -->
            <div class="w-full lg:flex-1 min-w-0 flex flex-col rounded-lg border border-gray-800/80 bg-gray-950/60 p-3">
              <div class="flex items-center justify-between pb-2 border-b border-gray-800/80">
                <div class="h-3.5 w-24 rounded bg-gray-800"></div>
                <div class="h-3 w-20 rounded bg-gray-800/50"></div>
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-3 gap-0 pt-2 flex-1 min-h-[120px] divide-y sm:divide-y-0 sm:divide-x divide-gray-800/60">
                {#each [0, 1, 2] as col}
                  <div class="space-y-2 {col > 0 ? 'sm:pl-2' : ''} {col < 2 ? 'sm:pr-2' : ''}">
                    <div class="h-3 w-16 rounded bg-gray-800/60 mb-2"></div>
                    {#each [0, 1] as _item}
                      <div class="flex items-center gap-2">
                        <div class="h-7 w-7 rounded-full bg-gray-800 shrink-0"></div>
                        <div class="flex-1 space-y-1">
                          <div class="h-3 w-20 rounded bg-gray-800"></div>
                          <div class="h-2.5 w-28 rounded bg-gray-800/50"></div>
                        </div>
                      </div>
                    {/each}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </section>
      {/if}

      <!-- Main Table Section (Spanning 100% Width) -->
      <section onscroll={handleTableScroll} class="relative min-h-0 flex-1 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
        {#if refreshing}
          <div class="pointer-events-none absolute left-0 right-0 top-0 z-30 h-0.5 overflow-hidden bg-purple-950">
            <div class="h-full w-full bg-purple-500 animate-[pulse_1s_infinite]"></div>
          </div>
        {/if}
        <div class="sticky top-0 z-20 flex flex-wrap items-center justify-between gap-2 border-b border-gray-800 bg-gray-900 px-3 py-2">
          <div>
            <h2 class="text-sm font-semibold text-white">Build performance</h2>
            <p class="text-[11px] text-gray-400">
              WR {gamesWon}/{sampleCount} ({percent(baseline)}) <span class="{overallWpa >= 0 ? 'text-emerald-400 font-semibold' : 'text-red-400 font-semibold'}">{formatWpaPoints(overallWpa)}</span>{#if refreshing} · refreshing{/if}
            </p>
          </div>
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
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'silver' ? 'bg-[#a3b9bd] text-gray-950 font-semibold shadow-sm' : 'text-[#a3b9bd]/70 hover:text-[#a3b9bd]'}"
                  onclick={() => (augmentTier = 'silver')}
                >
                  Silver
                </button>
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'gold' ? 'bg-[#e3c49f] text-gray-950 font-semibold shadow-sm' : 'text-[#e3c49f]/70 hover:text-[#e3c49f]'}"
                  onclick={() => (augmentTier = 'gold')}
                >
                  Gold
                </button>
                <button
                  type="button"
                  class="rounded px-2 py-1 transition-colors {augmentTier === 'prismatic' ? 'bg-[linear-gradient(180deg,#e78fff,#70008f)] text-white font-semibold shadow-sm' : 'text-[#f2c2ff]/70 hover:text-[#f2c2ff]'}"
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
                  onclick={() => switchKind(option[0] as StatisticsKind)}
                >
                  {option[1]}
                </button>
              {/each}
            </div>
          </div>
        </div>
        <table class="w-full text-left text-xs">
          <thead class="sticky top-[57px] z-20 bg-gray-900 text-[10px] uppercase tracking-wider text-gray-500">
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
            {#if loading && rows.length === 0}
              {#each Array(10) as _}
                <tr class="animate-pulse">
                  <td class="flex items-center gap-2 px-3 py-2.5">
                    <div class="h-7 w-7 rounded bg-gray-800 shrink-0"></div>
                    <div class="h-3.5 w-32 rounded bg-gray-800"></div>
                  </td>
                  {#if kind === "augment"}
                    <td class="px-3 py-2.5">
                      <div class="h-4 w-14 rounded bg-gray-800/60"></div>
                    </td>
                  {/if}
                  <td class="px-3 py-2.5">
                    <div class="flex items-center -space-x-2">
                      <div class="h-6 w-6 rounded-full bg-gray-800 border-2 border-gray-900"></div>
                      <div class="h-6 w-6 rounded-full bg-gray-800/80 border-2 border-gray-900"></div>
                      <div class="h-6 w-6 rounded-full bg-gray-800/60 border-2 border-gray-900"></div>
                    </div>
                  </td>
                  <td class="px-3 py-2.5 text-right"><div class="h-3 w-10 rounded bg-gray-800 ml-auto"></div></td>
                  <td class="px-3 py-2.5 text-right"><div class="h-3 w-10 rounded bg-gray-800 ml-auto"></div></td>
                  <td class="px-3 py-2.5 text-right"><div class="h-3 w-12 rounded bg-gray-800 ml-auto"></div></td>
                  {#if scope === "compare"}
                    <td class="px-3 py-2.5 text-right"><div class="h-3 w-12 rounded bg-gray-800 ml-auto"></div></td>
                  {/if}
                  <td class="px-3 py-2.5 text-right"><div class="h-3 w-12 rounded bg-gray-800 ml-auto"></div></td>
                  {#if scope === "compare"}
                    <td class="px-3 py-2.5 text-right"><div class="h-3 w-12 rounded bg-gray-800 ml-auto"></div></td>
                  {/if}
                  <td class="px-3 py-2.5 text-right"><div class="h-3 w-8 rounded bg-gray-800 ml-auto"></div></td>
                </tr>
              {/each}
            {:else if error && rows.length === 0}
              <tr>
                <td colspan={kind === "augment" ? (scope === "compare" ? 10 : 8) : (scope === "compare" ? 9 : 7)} class="px-3 py-12 text-center text-sm text-red-400">
                  {error}
                </td>
              </tr>
            {:else}
              {#each renderedRows as row (row.id)}
              {@const localWpa = scope === "compare" ? row.personal_wpa ?? 0 : row.wpa}
              {@const globalWpa = row.global_wpa ?? row.wpa}
              {@const topChamps = getTopChampions(row, 3)}
              <tr
                class="cursor-pointer hover:bg-gray-800/60 {selected?.id === row.id ? 'bg-purple-950/40' : ''}"
                onclick={() => (selectedId = selectedId === row.id ? null : row.id)}
              >
                <td
                  class="flex max-w-[380px] items-center gap-2 px-3 py-2 cursor-help"
                  onmouseenter={(e) => {
                    if (kind !== "combo") {
                      triggerHover(e, { name: row.name, iconUrl: row.icon_url, description: row.description, rarity: row.rarity, itemClass: row.item_class, priceTotal: row.price_total, categories: row.categories }, "top");
                    }
                  }}
                  onmouseleave={clearHover}
                >
                  {#if kind === "combo" && row.components && row.components.length > 0}
                    <div class="flex items-center gap-1.5 shrink-0 flex-wrap">
                      {#each row.components as comp, cIdx}
                        {#if cIdx > 0}
                          <span class="text-gray-500 text-xs font-bold select-none">+</span>
                        {/if}
                        <div
                          class="relative shrink-0 cursor-help"
                          onmouseenter={(e) => triggerHover(e, { name: comp.name, iconUrl: comp.icon_url, description: comp.description, rarity: comp.rarity, itemClass: comp.item_class, priceTotal: comp.price_total, categories: comp.categories }, "top")}
                          onmouseleave={clearHover}
                        >
                          {#if comp.kind === "champion"}
                            {#if comp.icon_url}
                              <img src={comp.icon_url} alt={comp.name} class="h-7 w-7 rounded-full object-cover border border-cyan-700/60 shadow-sm" />
                            {:else}
                              <div class="h-7 w-7 rounded-full bg-cyan-950 border border-cyan-700 flex items-center justify-center text-[10px] font-bold text-cyan-300">
                                {comp.name.slice(0, 1)}
                              </div>
                            {/if}
                          {:else if comp.kind === "augment"}
                            <AugmentIcon src={comp.icon_url} name={comp.name} rarity={comp.rarity} class="h-7 w-7" />
                          {:else}
                            {#if comp.icon_url}
                              <img src={comp.icon_url} alt={comp.name} class="h-7 w-7 rounded-md object-cover border border-gray-700 shadow-sm" />
                            {:else}
                              <div class="h-7 w-7 rounded-md bg-gray-800 border border-gray-700 flex items-center justify-center text-[10px] font-bold text-gray-300">
                                {comp.name.slice(0, 1)}
                              </div>
                            {/if}
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {:else if kind === "augment" || row.rarity}
                    <AugmentIcon src={row.icon_url} name={row.name} rarity={row.rarity} class="h-7 w-7" />
                    <span class="truncate font-medium {kind === 'augment' && row.rarity ? getAugmentTitleColor(row.rarity) : 'text-gray-200'}">{row.name}</span>
                  {:else if row.icon_url}
                    <img src={row.icon_url} alt={row.name} class="h-7 w-7 rounded object-cover shrink-0 border border-gray-800" />
                    <span class="truncate font-medium text-gray-200">{row.name}</span>
                  {:else}
                    <span class="truncate font-medium text-gray-200">{row.name}</span>
                  {/if}
                </td>
                {#if kind === "augment"}
                  <td class="px-3 py-2">
                    {#if row.rarity}
                      {@const lowerRarity = row.rarity.toLowerCase()}
                      <span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {lowerRarity === 'silver' ? 'bg-[#a3b9bd]/15 text-[#a3b9bd] border border-[#a3b9bd]/40' : lowerRarity === 'gold' ? 'bg-[#e3c49f]/15 text-[#e3c49f] border border-[#e3c49f]/40' : 'bg-[#70008f]/30 text-[#f2c2ff] border border-[#e78fff]/50'}">
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
                      <div class="isolate flex items-center -space-x-2.5 py-0.5">
                        {#each topChamps as champ, i (champ.id)}
                          <div
                            class="relative rounded-full cursor-default"
                            style="z-index: {i};"
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
            {:else}
              <tr><td colspan={kind === "augment" ? (scope === "compare" ? 10 : 8) : (scope === "compare" ? 9 : 7)} class="px-3 py-6 text-center text-gray-500">No matching builds found.</td></tr>
            {/each}
            {/if}
            {#if visibleRows.length > displayLimit}
              <tr class="border-t border-gray-800">
                <td colspan={kind === "augment" ? (scope === "compare" ? 10 : 8) : (scope === "compare" ? 9 : 7)} class="py-2.5 text-center">
                  <button
                    type="button"
                    class="text-xs text-purple-400 hover:text-purple-300 font-medium px-4 py-1.5 rounded-md bg-purple-950/40 hover:bg-purple-900/40 border border-purple-800/40 transition-colors"
                    onclick={() => (displayLimit = Math.min(displayLimit + 35, visibleRows.length))}
                  >
                    Show more ({displayLimit} of {visibleRows.length})
                  </button>
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      </section>
    </div>

  {#if activeHover && (activeHover.description || activeHover.name)}
    {@const normRarity = (activeHover.rarity ?? "").toLowerCase()}
    <div
      bind:this={popoverEl}
      style="position: fixed; left: {popoverPos.x}px; top: {popoverPos.y}px;"
      class="overflow-hidden z-50 w-80 max-w-[90vw] rounded-xl border {normRarity === 'prismatic' ? 'border-[#e78fff]/60 shadow-[0_0_16px_rgba(231,143,255,0.25)]' : normRarity === 'gold' ? 'border-[#e3c49f]/60 shadow-[0_0_16px_rgba(227,196,159,0.2)]' : normRarity === 'silver' ? 'border-[#a3b9bd]/60 shadow-[0_0_16px_rgba(163,185,189,0.15)]' : 'border-gray-700 shadow-2xl'} bg-gray-900/95 backdrop-blur-md p-3 text-gray-200 text-xs font-sans pointer-events-none transition-opacity duration-150"
    >
      <div class="flex items-center gap-2.5 pb-2.5 border-b border-gray-800">
        {#if activeHover.iconUrl}
          {#if activeHover.rarity}
            <AugmentIcon src={activeHover.iconUrl} name={activeHover.name} rarity={activeHover.rarity} class="h-10 w-10" roundedClass="rounded-lg" />
          {:else}
            <div class="relative flex h-10 w-10 shrink-0 items-center justify-center rounded-lg overflow-hidden bg-gray-950 border border-gray-700">
              <img src={activeHover.iconUrl} alt={activeHover.name} class="h-full w-full object-cover" />
            </div>
          {/if}
        {/if}
        <div class="min-w-0 flex-1">
          <div class="flex items-center justify-between gap-1.5">
            <h3 class="font-bold text-sm tracking-wide truncate {normRarity === 'prismatic' ? 'text-[#f2c2ff]' : normRarity === 'gold' ? 'text-[#e3c49f]' : normRarity === 'silver' ? 'text-[#a3b9bd]' : 'text-white'}">
              {activeHover.name}
            </h3>
            {#if activeHover.rarity}
              <span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider {normRarity === 'silver' ? 'bg-[#a3b9bd]/15 text-[#a3b9bd] border border-[#a3b9bd]/40' : normRarity === 'gold' ? 'bg-[#e3c49f]/15 text-[#e3c49f] border border-[#e3c49f]/40' : 'bg-[#70008f]/30 text-[#f2c2ff] border border-[#e78fff]/50'}">
                {activeHover.rarity}
              </span>
            {/if}
          </div>
          <div class="flex flex-wrap items-center gap-2 mt-0.5 text-[11px] text-gray-400">
            {#if (activeHover.priceTotal ?? 0) > 0}
              <span class="inline-flex items-center gap-1 font-semibold text-yellow-400">
                <img src="https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/gameplay/goldcoins.png" alt="gold" class="h-3 w-3 object-contain inline" />
                {(activeHover.priceTotal ?? 0).toLocaleString()}g
              </span>
            {/if}
            {#if activeHover.itemClass}
              <span class="capitalize text-gray-400">{activeHover.itemClass}</span>
            {/if}
            {#if activeHover.categories && activeHover.categories.length > 0}
              <span class="text-gray-500 truncate">{activeHover.categories.slice(0, 3).join(", ")}</span>
            {/if}
          </div>
        </div>
      </div>

      {#if activeHover.description}
        <div class="mt-2 text-xs text-gray-300 leading-relaxed select-text space-y-1">
          {@html parseGameDescription(activeHover.description)}
        </div>
      {:else}
        <p class="mt-2 text-xs text-gray-500 italic">No description available.</p>
      {/if}
    </div>
  {/if}
</div>
