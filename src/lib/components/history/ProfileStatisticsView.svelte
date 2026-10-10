<script lang="ts">
  import { onMount } from "svelte";
  import { getMatchHistory } from "../../api/history";
  import type { MatchHistoryItem } from "../../types/history";

  let { showHeader = true }: { showHeader?: boolean } = $props();

  type SortKey = "games" | "winRate" | "wpa" | "kda" | "damagePerMinute";
  type ChampionStats = {
    id: number;
    name: string;
    iconUrl: string;
    games: number;
    wins: number;
    kills: number;
    deaths: number;
    assists: number;
    damage: number;
    damageGames: number;
    gold: number;
    duration: number;
  };

  let matches = $state<MatchHistoryItem[]>([]);
  let selectedChampionId = $state<number | null>(null);
  let sortKey = $state<SortKey>("wpa");
  let sortDescending = $state(true);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const championStats = $derived.by(() => {
    const stats = new Map<number, ChampionStats>();
    for (const match of matches) {
      if (!match.champion) continue;
      const id = match.champion.id;
      const current = stats.get(id) ?? {
        id,
        name: match.champion.name,
        iconUrl: match.champion.square_icon_url,
        games: 0,
        wins: 0,
        kills: 0,
        deaths: 0,
        assists: 0,
        damage: 0,
        damageGames: 0,
        gold: 0,
        duration: 0,
      };
      current.games += 1;
      current.wins += match.win ? 1 : 0;
      current.kills += match.kills;
      current.deaths += match.deaths;
      current.assists += match.assists;
      current.gold += match.gold_earned;
      current.duration += match.game_duration_secs;
      if (match.damage_dealt_to_champions !== null) {
        current.damage += match.damage_dealt_to_champions;
        current.damageGames += 1;
      }
      stats.set(id, current);
    }
    return [...stats.values()];
  });

  const championBaseline = $derived(
    championStats.length
      ? championStats.reduce((wins, champion) => wins + champion.wins, 0) /
        championStats.reduce((games, champion) => games + champion.games, 0)
      : 0,
  );

  function championWpa(champion: ChampionStats) {
    return champion.wins / champion.games - championBaseline;
  }

  const sortedChampions = $derived(
    [...championStats].sort((a, b) => {
      const value = (item: ChampionStats) => {
        if (sortKey === "winRate") return item.wins / item.games;
        if (sortKey === "wpa") return championWpa(item);
        if (sortKey === "kda") return (item.kills + item.assists) / Math.max(1, item.deaths);
        if (sortKey === "damagePerMinute") {
          return item.damageGames ? item.damage / Math.max(1, item.duration / 60) : 0;
        }
        return item.games;
      };
      const difference = value(a) - value(b);
      return (sortDescending ? -1 : 1) * (difference || a.name.localeCompare(b.name));
    }),
  );

  const selectedStats = $derived(
    championStats.find((champion) => champion.id === selectedChampionId) ?? null,
  );
  const selectedMatches = $derived(
    matches.filter((match) => match.champion?.id === selectedChampionId),
  );
  const selectedItems = $derived.by(() => {
    const items = new Map<number, { id: number; name: string; iconUrl: string; games: number; wins: number }>();
    for (const match of selectedMatches) {
      for (const item of match.items) {
        if (!item) continue;
        const current = items.get(item.id) ?? { id: item.id, name: item.name, iconUrl: item.icon_url, games: 0, wins: 0 };
        current.games += 1;
        current.wins += match.win ? 1 : 0;
        items.set(item.id, current);
      }
    }
    return [...items.values()].sort((a, b) => (b.wins / b.games) - (a.wins / a.games));
  });

  const selectedAugments = $derived.by(() => {
    const augments = new Map<string, { id: string; name: string; iconUrl: string; games: number }>();
    for (const match of selectedMatches) {
      for (const augment of match.augments) {
        const current = augments.get(augment.id) ?? { id: augment.id, name: augment.name, iconUrl: augment.icon_url, games: 0 };
        current.games += 1;
        augments.set(augment.id, current);
      }
    }
    return [...augments.values()].sort((a, b) => b.games - a.games);
  });

  function setSort(key: SortKey) {
    if (sortKey === key) sortDescending = !sortDescending;
    else {
      sortKey = key;
      sortDescending = true;
    }
  }

  function percent(value: number) {
    return `${Math.round(value * 100)}%`;
  }

  function formatWpa(value: number) {
    const points = value * 100;
    return `${points >= 0 ? "+" : ""}${points.toFixed(2)}%`;
  }

  function normalizeMatches(value: MatchHistoryItem[]): MatchHistoryItem[] {
    return value.map((match) => ({
      ...match,
      damage_dealt_to_champions: match.damage_dealt_to_champions ?? null,
      augments: Array.isArray(match.augments) ? match.augments : [],
      items: Array.isArray(match.items) ? match.items : [],
    }));
  }

  function averageKda(stats: ChampionStats) {
    return `${(stats.kills / stats.games).toFixed(1)} / ${(stats.deaths / stats.games).toFixed(1)} / ${(stats.assists / stats.games).toFixed(1)}`;
  }

  async function loadMatches() {
    try {
      const cached = localStorage.getItem("lcu-match-history");
      if (cached) matches = normalizeMatches(JSON.parse(cached) as MatchHistoryItem[]);
      const fresh = await getMatchHistory(0, 20);
      const merged = new Map<number, MatchHistoryItem>();
      for (const match of [...fresh, ...matches]) {
        if (!merged.has(match.game_id)) merged.set(match.game_id, match);
      }
      matches = normalizeMatches([...merged.values()]);
      localStorage.setItem("lcu-match-history", JSON.stringify(matches));
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  onMount(loadMatches);
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-gray-950 p-4 text-gray-200">
{#if showHeader}
  <div class="flex shrink-0 items-center justify-between border-b border-gray-800 pb-3">
  <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100">
    <span class="icon-[heroicons--chart-bar] h-5 w-5 text-purple-400"></span>
    Statistics
  </h1>
  <span class="text-xs text-gray-500">{matches.length} matches analyzed</span>
  </div>
{/if}

  {#if loading}
    <div class="mt-4 flex-1 animate-pulse rounded-lg border border-gray-800 bg-gray-900/50"></div>
  {:else if error && matches.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-red-400">{error}</div>
  {:else if matches.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-gray-500">No match data available.</div>
  {:else}
    <div class="mt-3 grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3">
      <section class="min-h-0 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
        <table class="w-full text-left text-xs">
          <thead class="sticky top-0 bg-gray-900 text-xs uppercase tracking-wider text-gray-400">
            <tr>
              <th class="px-3 py-2">Champion</th>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("games")}>Games</th>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("winRate")}>WR</th>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("wpa")}>WPA</th>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("kda")}>Avg KDA</th>
              <th class="cursor-pointer px-3 py-2" onclick={() => setSort("damagePerMinute")}>Damage/min</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each sortedChampions as champion (champion.id)}
              <tr class="cursor-pointer hover:bg-gray-800/60 {selectedChampionId === champion.id ? 'bg-purple-950/40' : ''}" onclick={() => (selectedChampionId = champion.id)}>
                <td class="flex items-center gap-2 px-3 py-2">
                  <img src={champion.iconUrl} alt={champion.name} class="h-7 w-7 rounded-full object-cover" />
                  <span>{champion.name}</span>
                </td>
                <td class="px-3 py-2">{champion.games}</td>
                <td class="px-3 py-2">{percent(champion.wins / champion.games)}</td>
                <td class="px-3 py-2 {championWpa(champion) >= 0 ? 'text-emerald-300' : 'text-red-300'}">{formatWpa(championWpa(champion))}</td>
                <td class="px-3 py-2">{averageKda(champion)}</td>
                <td class="px-3 py-2">{champion.damageGames ? Math.round(champion.damage / Math.max(1, champion.duration / 60)) : "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>

      <aside class="min-h-0 overflow-y-auto rounded-lg border border-gray-800 bg-gray-900/40 p-3 scrollbar-thin scrollbar-thumb-gray-800">
        {#if selectedStats}
          <div class="flex items-center gap-2.5 border-b border-gray-800 pb-3">
            <img src={selectedStats.iconUrl} alt={selectedStats.name} class="h-10 w-10 rounded-full object-cover" />
            <div>
              <h2 class="font-bold text-white">{selectedStats.name}</h2>
              <p class="text-xs text-gray-400">{selectedStats.games} games · {percent(selectedStats.wins / selectedStats.games)} WR · {formatWpa(championWpa(selectedStats))} WPA</p>
            </div>
          </div>
          <dl class="grid grid-cols-2 gap-2 border-b border-gray-800 py-3 text-xs">
            <div><dt class="text-gray-500">Avg KDA</dt><dd>{averageKda(selectedStats)}</dd></div>
            <div><dt class="text-gray-500">WPA</dt><dd class="{championWpa(selectedStats) >= 0 ? 'text-emerald-300' : 'text-red-300'}">{formatWpa(championWpa(selectedStats))}</dd></div>
            <div><dt class="text-gray-500">Avg gold/min</dt><dd>{Math.round(selectedStats.gold / Math.max(1, selectedStats.duration / 60))}</dd></div>
            <div><dt class="text-gray-500">Avg damage/min</dt><dd>{selectedStats.damageGames ? Math.round(selectedStats.damage / Math.max(1, selectedStats.duration / 60)) : "—"}</dd></div>
            <div><dt class="text-gray-500">Avg game time</dt><dd>{Math.round(selectedStats.duration / selectedStats.games / 60)} min</dd></div>
          </dl>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Highest win-rate items</h3>
          <div class="space-y-1.5">
            {#each selectedItems.slice(0, 6) as item (item.id)}
              <div class="flex items-center justify-between text-xs">
                <span class="flex items-center gap-1.5"><img src={item.iconUrl} alt={item.name} class="h-5 w-5 rounded object-cover" />{item.name}</span>
                <span>{percent(item.wins / item.games)}</span>
              </div>
            {/each}
          </div>
          {#if selectedAugments.length > 0}
            <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Most played augments</h3>
            <div class="space-y-1.5">
              {#each selectedAugments as augment (augment.id)}
                <div class="flex items-center justify-between text-xs">
                  <span class="flex items-center gap-1.5"><img src={augment.iconUrl} alt={augment.name} class="h-5 w-5 rounded object-cover" />{augment.name}</span>
                  <span>{augment.games} games</span>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <div class="flex h-full items-center justify-center text-center text-sm text-gray-500">Select a champion to see detailed statistics.</div>
        {/if}
      </aside>
    </div>
  {/if}
</div>
