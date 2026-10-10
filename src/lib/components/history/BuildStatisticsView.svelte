<script lang="ts">
  import { onMount } from "svelte";
  import { getAgentBuildStatistics } from "../../api/agent";
  import { getMatchHistory } from "../../api/history";
  import type { BuildStatisticChampion, BuildStatisticEntry, MatchHistoryItem } from "../../types/history";
  import { logger } from "../../utils/logger";

  type Kind = "item" | "augment" | "combo";

  let matches = $state<MatchHistoryItem[]>([]);
  let selectedId = $state<string | null>(null);
  let kind = $state<Kind>("item");
  let loading = $state(true);
  let error = $state<string | null>(null);

  function normalizeMatches(value: MatchHistoryItem[]) {
    return value.map((match) => ({
      ...match,
      damage_dealt_to_champions: match.damage_dealt_to_champions ?? null,
      augments: Array.isArray(match.augments) ? match.augments : [],
      items: Array.isArray(match.items) ? match.items : [],
    }));
  }

  const baseline = $derived.by(() => {
    let observations = 0;
    let wins = 0;
    for (const match of matches) {
      const hasObservation = kind === "item"
        ? match.items.some(Boolean)
        : kind === "augment"
          ? match.augments.length > 0
          : match.items.filter(Boolean).length + match.augments.length > 1;
      if (hasObservation) {
        observations += 1;
        wins += match.win ? 1 : 0;
      }
    }
    return observations ? wins / observations : 0;
  });

  function addChampion(map: Map<number, BuildStatisticChampion>, match: MatchHistoryItem) {
    if (!match.champion) return;
    const champion = map.get(match.champion.id) ?? {
      id: match.champion.id,
      name: match.champion.name,
      icon_url: match.champion.square_icon_url,
      games: 0,
      wins: 0,
      kills: 0,
      deaths: 0,
      assists: 0,
      damage: 0,
      gold: 0,
    };
    champion.games += 1;
    champion.wins += match.win ? 1 : 0;
    champion.kills += match.kills;
    champion.deaths += match.deaths;
    champion.assists += match.assists;
    champion.damage += match.damage_dealt_to_champions ?? 0;
    champion.gold += match.gold_earned;
    map.set(champion.id, champion);
  }

  const entries = $derived.by(() => {
    const result = new Map<string, BuildStatisticEntry>();
    const add = (id: string, name: string, icon_url: string, match: MatchHistoryItem) => {
      const entry = result.get(id) ?? {
        id,
        name,
        icon_url,
        kind,
        games: 0,
        wins: 0,
        kills: 0,
        deaths: 0,
        assists: 0,
        damage: 0,
        gold: 0,
        duration: 0,
        champions: [],
        partners: [],
      };
      entry.games += 1;
      entry.wins += match.win ? 1 : 0;
      entry.kills += match.kills;
      entry.deaths += match.deaths;
      entry.assists += match.assists;
      entry.damage += match.damage_dealt_to_champions ?? 0;
      entry.gold += match.gold_earned;
      entry.duration += match.game_duration_secs;
      result.set(id, entry);
    };

    for (const match of matches) {
      if (kind === "item") {
        for (const item of match.items) {
          if (item) add(`item:${item.id}`, item.name, item.icon_url, match);
        }
      } else if (kind === "augment") {
        for (const augment of match.augments) {
          add(`augment:${augment.id}`, augment.name, augment.icon_url, match);
        }
      } else {
        const items = match.items.filter((item): item is NonNullable<typeof item> => Boolean(item));
        const augments = match.augments;
        const parts = [...items.map((item) => `i${item.id}`), ...augments.map((augment) => `a${augment.id}`)].sort();
        if (parts.length < 2) continue;
        const labels = [...items.map((item) => item.name), ...augments.map((augment) => augment.name)].sort();
        add(`combo:${parts.join("+")}`, labels.join(" + "), items[0]?.icon_url ?? augments[0]?.icon_url ?? "", match);
      }
    }

    for (const entry of result.values()) {
      const championMap = new Map<number, BuildStatisticChampion>();
      const partnerMap = new Map<string, { id: string; name: string; icon_url: string; games: number }>();
      for (const match of matches) {
        const candidateIds = kind === "item"
          ? match.items.filter(Boolean).map((item) => `item:${item!.id}`)
          : kind === "augment"
            ? match.augments.map((augment) => `augment:${augment.id}`)
            : (() => {
                const itemIds = match.items.filter(Boolean).map((item) => `i${item!.id}`);
                const augmentIds = match.augments.map((augment) => `a${augment.id}`);
                return itemIds.concat(augmentIds).sort().length > 1
                  ? [`combo:${itemIds.concat(augmentIds).sort().join("+")}`]
                  : [];
              })();
        if (!candidateIds.includes(entry.id)) continue;
        addChampion(championMap, match);
        for (const item of match.items) {
          if (item && `item:${item.id}` !== entry.id) {
            const partner = partnerMap.get(`item:${item.id}`) ?? { id: `item:${item.id}`, name: item.name, icon_url: item.icon_url, games: 0 };
            partner.games += 1;
            partnerMap.set(partner.id, partner);
          }
        }
        for (const augment of match.augments) {
          if (`augment:${augment.id}` !== entry.id) {
            const partner = partnerMap.get(`augment:${augment.id}`) ?? { id: `augment:${augment.id}`, name: augment.name, icon_url: augment.icon_url, games: 0 };
            partner.games += 1;
            partnerMap.set(partner.id, partner);
          }
        }
      }
      entry.champions = [...championMap.values()].sort((a, b) => b.games - a.games);
      entry.partners = [...partnerMap.values()].sort((a, b) => b.games - a.games);
    }
    return [...result.values()].sort((a, b) => wpa(b) - wpa(a) || b.games - a.games);
  });

  const selected = $derived(entries.find((entry) => entry.id === selectedId) ?? entries[0] ?? null);

  function percent(value: number) {
    return `${Math.round(value * 100)}%`;
  }

  function formatWpa(value: number) {
    const points = value * 100;
    return `${points >= 0 ? "+" : ""}${points.toFixed(2)}%`;
  }

  function wpa(entry: BuildStatisticEntry) {
    return entry.wins / entry.games - baseline;
  }

  function averageKda(entry: BuildStatisticEntry) {
    return `${(entry.kills / entry.games).toFixed(1)} / ${(entry.deaths / entry.games).toFixed(1)} / ${(entry.assists / entry.games).toFixed(1)}`;
  }

  async function loadMatches() {
    try {
      try {
        const agentData = await getAgentBuildStatistics();
        logger.debug("frontend.build-statistics", `Agent has ${agentData.facts.length} persisted participant facts`);
      } catch (agentError) {
        logger.debug("frontend.build-statistics", "Agent facts unavailable; using cached detailed history", agentError);
      }
      const cached = localStorage.getItem("lcu-match-history");
      if (cached) matches = normalizeMatches(JSON.parse(cached) as MatchHistoryItem[]);
      const fresh = await getMatchHistory(0, 20);
      const merged = new Map<number, MatchHistoryItem>();
      for (const match of [...fresh, ...matches]) if (!merged.has(match.game_id)) merged.set(match.game_id, match);
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
  {#if loading && matches.length === 0}
    <div class="mt-3 grid flex-1 animate-pulse grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3">
      <div class="rounded-lg border border-gray-800 bg-gray-900/50"></div>
      <div class="rounded-lg border border-gray-800 bg-gray-900/50"></div>
    </div>
  {:else if error && matches.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-red-400">{error}</div>
  {:else if matches.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-gray-500">No match data available.</div>
  {:else}
    <div class="mt-3 grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(280px,360px)] gap-3">
      <section class="min-h-0 overflow-auto rounded-lg border border-gray-800 bg-black/40 scrollbar-thin scrollbar-thumb-gray-800">
        <div class="sticky top-0 z-10 flex items-center justify-between border-b border-gray-800 bg-gray-900 px-3 py-2">
          <div>
            <h2 class="text-sm font-semibold text-white">Build performance</h2>
            <p class="text-[11px] text-gray-500">WPA = win-rate percentage-point delta · peer baseline {percent(baseline)} · {matches.length} matches</p>
          </div>
          <div class="flex rounded-lg border border-gray-700 bg-gray-950 p-0.5 text-[11px]">
            {#each [["item", "Items"], ["augment", "Augments"], ["combo", "Combos"]] as option}
              <button type="button" class="rounded-md px-2 py-1 transition-colors {kind === option[0] ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => { kind = option[0] as Kind; selectedId = null; }}>{option[1]}</button>
            {/each}
          </div>
        </div>
        <table class="w-full text-left text-xs">
          <thead class="sticky top-[57px] bg-gray-900 text-[10px] uppercase tracking-wider text-gray-500">
            <tr><th class="px-3 py-2">Build</th><th class="px-3 py-2">Games</th><th class="px-3 py-2">WR</th><th class="px-3 py-2">WPA</th><th class="px-3 py-2">Avg KDA</th></tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each entries as entry (entry.id)}
              <tr class="cursor-pointer hover:bg-gray-800/60 {selected?.id === entry.id ? 'bg-purple-950/40' : ''}" onclick={() => (selectedId = entry.id)}>
                <td class="flex max-w-[360px] items-center gap-2 px-3 py-2">
                  {#if entry.icon_url}<img src={entry.icon_url} alt={entry.name} class="h-7 w-7 rounded object-cover" />{/if}
                  <span class="truncate text-gray-200">{entry.name}</span>
                </td>
                <td class="px-3 py-2 text-gray-400">{entry.games}</td>
                <td class="px-3 py-2">{percent(entry.wins / entry.games)}</td>
                <td class="px-3 py-2 {wpa(entry) >= 0 ? 'text-emerald-300' : 'text-red-300'}">{formatWpa(wpa(entry))}</td>
                <td class="px-3 py-2 text-gray-400">{averageKda(entry)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>

      <aside class="min-h-0 overflow-y-auto rounded-lg border border-gray-800 bg-gray-900/40 p-3 scrollbar-thin scrollbar-thumb-gray-800">
        {#if selected}
          <div class="border-b border-gray-800 pb-3">
            <div class="flex items-center gap-2">
              {#if selected.icon_url}<img src={selected.icon_url} alt={selected.name} class="h-10 w-10 rounded object-cover" />{/if}
              <div><h2 class="font-bold text-white">{selected.name}</h2><p class="text-xs text-gray-400">{selected.games} games · {percent(selected.wins / selected.games)} win rate</p></div>
            </div>
            <p class="mt-2 text-[11px] leading-relaxed text-gray-500">This build is associated with <span class="text-gray-300">{selected.champions.length}</span> champions. WPA compares its observed win rate with the {percent(baseline)} peer baseline; it is not a causal win-probability model.</p>
          </div>
          <dl class="grid grid-cols-2 gap-2 border-b border-gray-800 py-3 text-xs">
            <div><dt class="text-gray-500">WPA</dt><dd class="{wpa(selected) >= 0 ? 'text-emerald-300' : 'text-red-300'}">{formatWpa(wpa(selected))}</dd></div>
            <div><dt class="text-gray-500">Avg KDA</dt><dd>{averageKda(selected)}</dd></div>
            <div><dt class="text-gray-500">Avg damage</dt><dd>{Math.round(selected.damage / selected.games).toLocaleString()}</dd></div>
            <div><dt class="text-gray-500">Avg gold</dt><dd>{Math.round(selected.gold / selected.games).toLocaleString()}</dd></div>
          </dl>
          <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Champions used with this build</h3>
          <div class="space-y-1.5">
            {#each selected.champions.slice(0, 8) as champion (champion.id)}
              <div class="flex items-center justify-between text-xs"><span class="flex items-center gap-1.5"><img src={champion.icon_url} alt={champion.name} class="h-5 w-5 rounded-full object-cover" />{champion.name}</span><span class="text-gray-400">{champion.games} games · {percent(champion.wins / champion.games)}</span></div>
            {/each}
          </div>
          {#if selected.partners.length > 0}
            <h3 class="mb-1.5 mt-3 text-xs font-semibold uppercase tracking-wider text-gray-500">Often paired with</h3>
            <div class="space-y-1.5">
              {#each selected.partners.slice(0, 6) as partner (partner.id)}
                <div class="flex items-center justify-between text-xs"><span class="flex items-center gap-1.5"><img src={partner.icon_url} alt={partner.name} class="h-5 w-5 rounded object-cover" />{partner.name}</span><span class="text-gray-400">{partner.games} games</span></div>
              {/each}
            </div>
          {/if}
        {:else}
          <div class="flex h-full items-center justify-center text-center text-sm text-gray-500">Select an item, augment, or combo to inspect its champion and performance context.</div>
        {/if}
      </aside>
    </div>
  {/if}
</div>
