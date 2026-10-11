<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getCachedStatistics, requestStatistics, type StatisticsScope } from "../../utils/statistics-data";
  import type { ServerStatisticRow } from "../../api/agent";

  let { showHeader = true, scope = "personal", hideLowPickrate = false }: { showHeader?: boolean; scope?: StatisticsScope; hideLowPickrate?: boolean } = $props();

  type SortKey = "name" | "games" | "pickRate" | "winRate" | "personalWinRate" | "globalWinRate" | "wpa" | "personalWpa" | "globalWpa" | "kda" | "damagePerMinute";
  let rows = $state<ServerStatisticRow[]>([]);
  let selectedId = $state<string | null>(null);
  let sortKey = $state<SortKey>(scope === "compare" ? "personalWpa" : "wpa");
  let sortDescending = $state(true);
  let search = $state("");
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let sampleCount = $state(0);
  let total = $state(0);
  let requestVersion = 0;

  function query() {
    const effectiveSortKey = scope === "compare"
      ? sortKey === "winRate" ? "personalWinRate" : sortKey === "wpa" ? "personalWpa" : sortKey
      : sortKey === "personalWinRate" || sortKey === "globalWinRate" ? "winRate" : sortKey === "personalWpa" || sortKey === "globalWpa" ? "wpa" : sortKey;
    return { scope: scope ?? "personal", kind: "champion" as const, search, hide_low_pickrate: hideLowPickrate, sort_key: effectiveSortKey, descending: sortDescending, limit: 100 };
  }

  async function load(force = false) {
    const version = ++requestVersion;
    const cached = getCachedStatistics(query());
    if (cached && !force) {
      rows = cached.rows;
      sampleCount = cached.sample_count;
      total = cached.total;
      loading = false;
    } else if (rows.length) {
      refreshing = true;
    }
    try {
      const response = await requestStatistics(query(), { force });
      if (version !== requestVersion) return;
      rows = response.rows;
      sampleCount = response.sample_count;
      total = response.total;
      error = null;
    } catch (err) {
      if (version === requestVersion) error = String(err);
    } finally {
      if (version === requestVersion) {
        loading = false;
        refreshing = false;
      }
    }
  }

  const visibleRows = $derived(rows);

  const selected = $derived(rows.find((row) => row.id === selectedId) ?? visibleRows[0] ?? null);

  function setSort(key: SortKey) {
    if (sortKey === key) sortDescending = !sortDescending;
    else {
      sortKey = key;
      sortDescending = key !== "name";
    }
    void load(true);
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

  $effect(() => {
    void scope;
    void search;
    void hideLowPickrate;
    const timer = setTimeout(() => void untrack(() => load()), 180);
    return () => clearTimeout(timer);
  });

  onMount(() => void load());
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-gray-950 p-4 text-gray-200">
  {#if showHeader}
    <div class="flex shrink-0 items-center justify-between border-b border-gray-800 pb-3">
      <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100"><span class="icon-[heroicons--chart-bar] h-5 w-5 text-purple-400"></span>Statistics</h1>
      <span class="text-xs text-gray-500">{total || rows.length} champions · {sampleCount} games {#if refreshing}· refreshing{/if}</span>
    </div>
  {/if}
  {#if loading && rows.length === 0}
    <div class="mt-4 flex-1 animate-pulse rounded-lg border border-gray-800 bg-gray-900/50"></div>
  {:else if error && rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-red-400">{error}</div>
  {:else if rows.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-gray-500">No statistics available.</div>
  {:else}
    <div class="mt-3 grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3">
      <section class="min-h-0 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
        <div class="sticky top-0 z-10 flex justify-end border-b border-gray-800 bg-gray-900 px-3 py-2">
          <input bind:value={search} type="search" placeholder="Search champions..." class="w-40 rounded-md border border-gray-700 bg-gray-950 px-2 py-1.5 text-[11px] text-gray-100 outline-none placeholder:text-gray-600 focus:border-purple-500" />
        </div>
        <table class="w-full text-left text-xs">
          <thead class="sticky top-[41px] bg-gray-900 text-xs uppercase tracking-wider text-gray-400"><tr>
            <th class="cursor-pointer px-3 py-2" onclick={() => setSort("name")}>Champion {sortIndicator("name")}</th>
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("games")}>Games {sortIndicator("games")}</th>
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("pickRate")}>Pick rate {sortIndicator("pickRate")}</th>
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWinRate" : "winRate")}>{scope === "compare" ? "Mine WR" : "WR"} {sortIndicator(scope === "compare" ? "personalWinRate" : "winRate")}</th>
            {#if scope === "compare"}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWinRate")}>Global WR {sortIndicator("globalWinRate")}</th>{/if}
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort(scope === "compare" ? "personalWpa" : "wpa")}>{scope === "compare" ? "Mine WPA" : "WPA"} {sortIndicator(scope === "compare" ? "personalWpa" : "wpa")}</th>
            {#if scope === "compare"}<th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("globalWpa")}>Global WPA {sortIndicator("globalWpa")}</th>{/if}
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("kda")}>KDA {sortIndicator("kda")}</th>
            <th class="cursor-pointer px-3 py-2 text-right" onclick={() => setSort("damagePerMinute")}>Damage/min {sortIndicator("damagePerMinute")}</th>
          </tr></thead>
          <tbody class="divide-y divide-gray-800">
            {#each visibleRows as row (row.id)}
              {@const localWpa = scope === "compare" ? row.personal_wpa ?? 0 : row.wpa}
              {@const globalWpa = row.global_wpa ?? row.wpa}
              <tr class="cursor-pointer hover:bg-gray-800/60 {selected?.id === row.id ? 'bg-purple-950/40' : ''}" onclick={() => (selectedId = row.id)}>
                <td class="flex items-center gap-2 px-3 py-2">{#if row.icon_url}<img src={row.icon_url} alt={row.name} class="h-7 w-7 rounded-full object-cover" />{/if}<span>{row.name}</span></td>
                <td class="px-3 py-2 text-right">{scope === "compare" ? `${row.personal_games ?? 0}/${row.global_observed ? row.global_games ?? row.games : "—"}` : row.games}</td><td class="px-3 py-2 text-right">{percent(row.pick_rate)}</td><td class="px-3 py-2 text-right">{scope === "compare" ? (row.personal_games ? percent(row.personal_win_rate ?? 0) : "—") : percent(row.win_rate)}</td>{#if scope === "compare"}<td class="px-3 py-2 text-right">{row.global_observed ? percent(row.global_win_rate ?? row.win_rate) : "—"}</td>{/if}<td class="px-3 py-2 text-right {localWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{scope === "compare" && !row.personal_games ? "—" : formatWpaPoints(localWpa)}</td>{#if scope === "compare"}<td class="px-3 py-2 text-right {globalWpa >= 0 ? 'text-emerald-300' : 'text-red-300'}">{row.global_observed ? formatWpaPoints(globalWpa) : "—"}</td>{/if}<td class="px-3 py-2 text-right">{averageKda(row).toFixed(2)}</td><td class="px-3 py-2 text-right">{row.damage_per_minute ? Math.round(row.damage_per_minute) : "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
      <aside class="min-h-0 overflow-y-auto rounded-lg border border-gray-800 bg-gray-900/40 p-3 scrollbar-thin scrollbar-thumb-gray-800">
        {#if selected}
          <div class="flex items-center gap-2.5 border-b border-gray-800 pb-3">{#if selected.icon_url}<img src={selected.icon_url} alt={selected.name} class="h-10 w-10 rounded-full object-cover" />{/if}<div><h2 class="font-bold text-white">{selected.name}</h2>{#if scope === "compare"}<p class="text-xs text-gray-400">Mine: {selected.personal_games ? `${selected.personal_games} games · ${percent(selected.personal_win_rate ?? 0)} WR · ${formatWpaPoints(selected.personal_wpa ?? 0)} WPA` : "No personal observations yet"}</p><p class="text-xs text-gray-400">Database: {selected.global_observed ? `${selected.global_games ?? selected.games} games · ${percent(selected.global_win_rate ?? selected.win_rate)} WR · ${formatWpaPoints(selected.global_wpa ?? selected.wpa)} WPA` : "No database observations yet"}</p>{:else}<p class="text-xs text-gray-400">{selected.games} games · {percent(selected.win_rate)} WR · {formatWpaPoints(selected.wpa)} WPA</p>{/if}</div></div>
          <dl class="grid grid-cols-2 gap-2 border-b border-gray-800 py-3 text-xs">
            <div><dt class="text-gray-500">KDA</dt><dd>{averageKda(selected).toFixed(2)}</dd></div><div><dt class="text-gray-500">Damage/min</dt><dd>{Math.round(selected.damage_per_minute)}</dd></div>
            <div><dt class="text-gray-500">Avg gold</dt><dd>{Math.round(selected.gold / Math.max(1, selected.games)).toLocaleString()}</dd></div><div><dt class="text-gray-500">{scope === "compare" ? "Mine WPA" : "WPA"}</dt><dd class={((scope === "compare" ? selected.personal_wpa ?? 0 : selected.wpa) >= 0) ? "text-emerald-300" : "text-red-300"}>{formatWpaPoints(scope === "compare" ? (selected.personal_wpa ?? 0) : selected.wpa)}</dd></div>{#if scope === "compare"}<div><dt class="text-gray-500">Global WPA</dt><dd class={(selected.global_wpa ?? 0) >= 0 ? "text-emerald-300" : "text-red-300"}>{selected.global_observed ? formatWpaPoints(selected.global_wpa ?? 0) : "—"}</dd></div>{/if}
          </dl>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Associated items</h3>
          <div class="space-y-1.5">{#each selected.partners.filter((partner) => String(partner.id).startsWith("item:")).slice(0, 6) as partner (partner.id)}
            <div class="flex items-center justify-between gap-2 text-xs"><span class="truncate">{partner.name}</span><span class="shrink-0 text-right text-gray-400">{partner.games} games · {percent(partner.win_rate)} WR · <span class={partner.wpa >= 0 ? "text-emerald-300" : "text-red-300"}>{formatWpa(partner.wpa)} WPA</span></span></div>
          {/each}</div>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Associated augments</h3>
          <div class="space-y-1.5">{#each selected.partners.filter((partner) => String(partner.id).startsWith("augment:")).slice(0, 6) as partner (partner.id)}
            <div class="flex items-center justify-between gap-2 text-xs"><span class="truncate">{partner.name}</span><span class="shrink-0 text-right text-gray-400">{partner.games} games · {percent(partner.win_rate)} WR · <span class={partner.wpa >= 0 ? "text-emerald-300" : "text-red-300"}>{formatWpa(partner.wpa)} WPA</span></span></div>
          {/each}</div>
        {:else}<div class="flex h-full items-center justify-center text-center text-sm text-gray-500">Select a champion to inspect its statistics.</div>{/if}
      </aside>
    </div>
  {/if}
</div>
