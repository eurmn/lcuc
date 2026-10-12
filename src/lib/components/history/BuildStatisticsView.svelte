<script lang="ts">
  import { untrack } from "svelte";
  import { getCachedStatistics, requestStatistics, type StatisticsKind, type StatisticsScope } from "../../utils/statistics-data";
  import type { ServerStatisticRow } from "../../api/agent";

  let { scope = "personal", hideLowPickrate = false }: { scope?: StatisticsScope; hideLowPickrate?: boolean } = $props();
  let kind = $state<StatisticsKind>("item");
  let rows = $state<ServerStatisticRow[]>([]);
  let selectedId = $state<string | null>(null);
  let search = $state("");
  let debouncedSearch = $state("");
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  type SortKey = "name" | "games" | "pickRate" | "winRate" | "personalWinRate" | "globalWinRate" | "wpa" | "personalWpa" | "globalWpa" | "kda";
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
      rows = cached.rows; sampleCount = cached.sample_count; baseline = cached.baseline;  loading = false;
    } else if (rows.length) refreshing = true;
    try {
      const response = await requestStatistics(currentQuery, { force });
      if (version !== requestVersion) return;
      rows = response.rows; sampleCount = response.sample_count; baseline = response.baseline;  error = null;
    } catch (err) {
      if (version === requestVersion) error = String(err);
    } finally {
      if (version === requestVersion) { loading = false; refreshing = false; }
    }
  }

  const visibleRows = $derived(
    rows.filter((row) => {
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
        if (itemClass === "other") return showStarters; // small consumables/trinkets
        // legendary items always visible
        return true;
      }

      return true;
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
  function partnerLabel(row: ServerStatisticRow) {
    return row.kind === "augment" ? "Often paired with this" : "Often paired with this augment";
  }

  function filterAndSortPartners(partners: ServerStatisticRow["partners"], parentGames: number, kind: string) {
    const list = partners.filter((partner) => kind !== "item" || String(partner.id).startsWith("augment:"));
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
      .slice(0, 8);
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
    <div class="mt-3 grid flex-1 animate-pulse grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3"><div class="rounded-lg border border-gray-800 bg-gray-900/50"></div><div class="rounded-lg border border-gray-800 bg-gray-900/50"></div></div>
  {:else if error && rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-red-400">{error}</div>
  {:else if rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-gray-500">No build statistics available.</div>
  {:else}
    <div class="mt-3 grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3">
      <section class="min-h-0 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
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
        <table class="w-full text-left text-xs"><thead class="sticky top-[57px] bg-gray-900 text-[10px] uppercase tracking-wider text-gray-500"><tr>
          <th class="cursor-pointer px-3 py-2" onclick={() => setSort("name")}>Build {sortIndicator("name")}</th><th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("games")}>Games {sortIndicator("games")}</th><th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("pickRate")}>Pick rate {sortIndicator("pickRate")}</th><th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWinRate" : "winRate")}>{scope === "compare" ? "Mine WR" : "WR"} {sortIndicator(scope === "compare" ? "personalWinRate" : "winRate")}</th>{#if scope === "compare"}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWinRate")}>Global WR {sortIndicator("globalWinRate")}</th>{/if}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWpa" : "wpa")}>{scope === "compare" ? "Mine WPA" : "WPA"} {sortIndicator(scope === "compare" ? "personalWpa" : "wpa")}</th>{#if scope === "compare"}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWpa")}>Global WPA {sortIndicator("globalWpa")}</th>{/if}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("kda")}>KDA {sortIndicator("kda")}</th>
        </tr></thead><tbody class="divide-y divide-gray-800">{#each visibleRows as row (row.id)}{@const localWpa = scope === "compare" ? row.personal_wpa ?? 0 : row.wpa}{@const globalWpa = row.global_wpa ?? row.wpa}<tr class="cursor-pointer hover:bg-gray-800/60 {selected?.id === row.id ? 'bg-purple-950/40' : ''}" onclick={() => (selectedId = row.id)}><td class="flex max-w-[360px] items-center gap-2 px-3 py-2">{#if row.icon_url}<img src={row.icon_url} alt={row.name} class="h-7 w-7 rounded object-cover" />{/if}<span class="truncate">{row.name}</span>{#if row.rarity}<span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider {row.rarity === 'silver' ? 'bg-slate-700/60 text-slate-300 border border-slate-600/40' : row.rarity === 'gold' ? 'bg-yellow-950/60 text-yellow-300 border border-yellow-700/40' : 'bg-fuchsia-950/60 text-fuchsia-300 border border-fuchsia-700/40'}">{row.rarity}</span>{/if}</td><td class="px-3 py-2 text-right text-gray-400">{scope === "compare" ? `${row.personal_games ?? 0}/${row.global_observed ? row.global_games ?? row.games : "—"}` : row.games}</td><td class="px-3 py-2 text-right text-gray-400">{percent(row.pick_rate)}</td><td class="px-3 py-2 text-right">{scope === "compare" ? (row.personal_games ? percent(row.personal_win_rate ?? 0) : "—") : percent(row.win_rate)}</td>{#if scope === "compare"}<td class="px-3 py-2 text-right">{row.global_observed ? percent(row.global_win_rate ?? row.win_rate) : "—"}</td>{/if}<td class="px-3 py-2 text-right {localWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{scope === "compare" && !row.personal_games ? "—" : formatWpaPoints(localWpa)}</td>{#if scope === "compare"}<td class="px-3 py-2 text-right {globalWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{row.global_observed ? formatWpaPoints(globalWpa) : "—"}</td>{/if}<td class="px-3 py-2 text-right text-gray-400">{averageKda(row).toFixed(2)}</td></tr>{/each}</tbody></table>
      </section>
      <aside class="min-h-0 overflow-y-auto rounded-lg border border-gray-800 bg-gray-900/40 p-3 scrollbar-thin scrollbar-thumb-gray-800">
        {#if selected}<div class="border-b border-gray-800 pb-3"><div class="flex items-center gap-2">{#if selected.icon_url}<img src={selected.icon_url} alt={selected.name} class="h-10 w-10 rounded object-cover" />{/if}<div><div class="flex items-center gap-2"><h2 class="font-bold text-white">{selected.name}</h2>{#if selected.rarity}<span class="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider {selected.rarity === 'silver' ? 'bg-slate-700/60 text-slate-300 border border-slate-600/40' : selected.rarity === 'gold' ? 'bg-yellow-950/60 text-yellow-300 border border-yellow-700/40' : 'bg-fuchsia-950/60 text-fuchsia-300 border border-fuchsia-700/40'}">{selected.rarity}</span>{/if}</div>{#if scope === "compare"}<p class="text-xs text-gray-400">Mine: {selected.personal_games ? `${selected.personal_games} games · ${percent(selected.personal_win_rate ?? 0)} WR · ${formatWpaPoints(selected.personal_wpa ?? 0)} WPA` : "No personal observations yet"}</p><p class="text-xs text-gray-400">Database: {selected.global_observed ? `${selected.global_games ?? selected.games} games · ${percent(selected.global_win_rate ?? selected.win_rate)} WR · ${formatWpaPoints(selected.global_wpa ?? selected.wpa)} WPA` : "No database observations yet"}</p>{:else}<p class="text-xs text-gray-400">{selected.games} games · {percent(selected.win_rate)} WR · {formatWpaPoints(selected.wpa)} WPA</p>{/if}</div></div><p class="mt-2 text-[11px] text-gray-500">Server-calculated statistics for {selected.champions.length} associated champions.</p></div>
          <dl class="grid grid-cols-2 gap-2 border-b border-gray-800 py-3 text-xs"><div><dt class="text-gray-500">KDA</dt><dd>{averageKda(selected).toFixed(2)}</dd></div><div><dt class="text-gray-500">Avg damage</dt><dd>{Math.round(selected.damage / Math.max(1, selected.games)).toLocaleString()}</dd></div><div><dt class="text-gray-500">Avg gold</dt><dd>{Math.round(selected.gold / Math.max(1, selected.games)).toLocaleString()}</dd></div><div><dt class="text-gray-500">{scope === "compare" ? "Mine WPA" : "WPA"}</dt><dd class={scope === "compare" ? (selected.personal_wpa ?? 0) >= 0 ? "text-emerald-300" : "text-red-300" : selected.wpa >= 0 ? "text-emerald-300" : "text-red-300"}>{formatWpaPoints(scope === "compare" ? (selected.personal_wpa ?? 0) : selected.wpa)}</dd></div>{#if scope === "compare"}<div><dt class="text-gray-500">Global WPA</dt><dd class={(selected.global_wpa ?? 0) >= 0 ? "text-emerald-300" : "text-red-300"}>{selected.global_observed ? formatWpaPoints(selected.global_wpa ?? 0) : "—"}</dd></div>{/if}</dl>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Champions used with this build</h3>
          <div class="space-y-1.5">{#each selected.champions.slice(0, 8) as champion (champion.id)}
            <div class="flex items-center justify-between gap-2 text-xs">
              <div class="flex min-w-0 items-center gap-1.5">
                {#if champion.icon_url}
                  <img src={champion.icon_url} alt={champion.name} class="h-5 w-5 shrink-0 rounded-full object-cover" />
                {/if}
                <span class="truncate" title={champion.name}>{champion.name}</span>
              </div>
              <span class="shrink-0 text-right text-gray-400">{champion.games} games · {percent(champion.win_rate)} WR · <span class={champion.wpa >= 0 ? "text-emerald-300" : "text-red-300"}>{formatWpa(champion.wpa)} WPA</span></span>
            </div>
          {/each}</div>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">{partnerLabel(selected)}</h3>
          <div class="space-y-1.5">
            {#each filterAndSortPartners(selected.partners, selected.games, selected.kind) as partner (partner.id)}
              <div class="flex items-center justify-between gap-2 text-xs">
                <div class="flex min-w-0 items-center gap-1.5">
                  {#if partner.icon_url}
                    <img src={partner.icon_url} alt={partner.name} class="h-5 w-5 shrink-0 rounded object-cover" />
                  {/if}
                  <span class="truncate" title={partner.name}>{partner.name}</span>
                </div>
                <span class="shrink-0 text-right text-gray-400">{partner.games} games · {percent(partner.win_rate)} WR · <span class={partner.wpa >= 0 ? "text-emerald-300" : "text-red-300"}>{formatWpa(partner.wpa)} WPA</span></span>
              </div>
            {/each}
          </div>
        {:else}<div class="flex h-full items-center justify-center text-center text-sm text-gray-500">Select a build to inspect its context.</div>{/if}
      </aside>
    </div>
  {/if}
</div>
