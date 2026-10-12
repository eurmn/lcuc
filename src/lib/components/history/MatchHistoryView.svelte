<script lang="ts">
  import { onMount } from "svelte";
  import { getMatchHistory } from "../../api/history";
  import type { MatchHistoryItem } from "../../types/history";
  import { logger } from "../../utils/logger";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";

  let matches = $state<MatchHistoryItem[]>([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let currentPage = $state(1);
  const pageSize = 10;
  const historyCacheKey = "lcu-match-history";

  const totalPages = $derived(Math.max(1, Math.ceil(matches.length / pageSize)));
  const visibleMatches = $derived(
    matches.slice((currentPage - 1) * pageSize, currentPage * pageSize),
  );

  function formatGameMode(mode: string): string {
    const upper = (mode || "").toUpperCase();
    if (upper === "KIWI") return "ARAM: Mayhem";
    if (upper === "ARAM") return "ARAM";
    if (upper === "CLASSIC") return "Summoner's Rift";
    if (upper === "URF") return "URF";
    if (upper === "CHERRY") return "Arena";
    return mode || "Normal";
  }

  function formatDuration(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
  }

  function formatDate(isoString: string): string {
    if (!isoString) return "";
    const date = new Date(isoString);
    if (Number.isNaN(date.getTime())) return isoString;
    return date.toLocaleDateString(undefined, { month: "2-digit", day: "2-digit", year: "numeric" });
  }

  function formatMap(mapId: number): string {
    if (mapId === 11) return "Summoner's Rift";
    if (mapId === 12) return "Howling Abyss";
    if (mapId === 30) return "Koeshin's Crossing";
    return "League of Legends";
  }

  function mergeMatches(freshMatches: MatchHistoryItem[], cachedMatches: MatchHistoryItem[]) {
    const merged = new Map<number, MatchHistoryItem>();
    for (const match of [...freshMatches, ...cachedMatches]) {
      if (!merged.has(match.game_id)) {
        merged.set(match.game_id, match);
      }
    }
    return [...merged.values()];
  }

  async function loadHistory() {
    try {
      const pageSize = 20;
      refreshing = true;
      error = null;
      logger.info("frontend.history", "Starting progressive match-history scan");
      const firstPage = await getMatchHistory(0, pageSize - 1);
      matches = mergeMatches(firstPage, matches);
      currentPage = 1;
      localStorage.setItem(historyCacheKey, JSON.stringify(matches));
      logger.info("frontend.history", `Loaded page 0..${pageSize - 1} with ${firstPage.length} matches; ${matches.length} available after cache merge`);

      const seenGameIds = new Set(firstPage.map((match) => match.game_id));
      let begin = pageSize;
      let pageNumber = 1;
      const maxPages = 1000;

      while (pageNumber < maxPages) {
        const end = begin + pageSize - 1;
        const page = await getMatchHistory(begin, end);
        if (page.length === 0) {
          logger.info("frontend.history", `Match-history scan reached an empty page at ${begin}..${end}`);
          break;
        }

        const newMatches = page.filter((match) => !seenGameIds.has(match.game_id));
        for (const match of newMatches) {
          seenGameIds.add(match.game_id);
        }

        matches = mergeMatches(page, matches);
        localStorage.setItem(historyCacheKey, JSON.stringify(matches));
        logger.info(
          "frontend.history",
          `Loaded page ${begin}..${end}: ${page.length} matches, ${newMatches.length} new, ${matches.length} total`,
        );

        if (newMatches.length === 0) {
          logger.info("frontend.history", `Match-history scan stopped on repeated page at ${begin}..${end}`);
          break;
        }

        begin = end + 1;
        pageNumber += 1;
      }

      if (pageNumber >= maxPages) {
        logger.warn("frontend.history", `Match-history scan stopped at the ${maxPages}-page safety limit`);
      }

      logger.info("frontend.history", `Match-history scan complete with ${matches.length} matches available`);
    } catch (err) {
      error = String(err);
      logger.error("frontend.history", "Match history fetch failed", err);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  function restoreCachedHistory() {
    const cached = localStorage.getItem(historyCacheKey);
    if (!cached) return;
    try {
      const parsed = JSON.parse(cached) as MatchHistoryItem[];
      if (Array.isArray(parsed)) {
        matches = parsed;
        currentPage = 1;
        loading = false;
        logger.debug("frontend.history", `Restored ${matches.length} cached matches`);
      }
    } catch (err) {
      logger.warn("frontend.history", "Ignoring invalid cached history", err);
      localStorage.removeItem(historyCacheKey);
    }
  }

  onMount(() => {
    restoreCachedHistory();
    loadHistory();
  });
</script>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-gray-950 p-4 text-gray-200 select-none">
  <div class="flex items-center justify-between border-b border-gray-800 pb-3">
    <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100">
      <span class="icon-[heroicons--clock] h-5 w-5 text-purple-400"></span>
      Match History
    </h1>
    <button
      onclick={loadHistory}
      class="flex cursor-pointer items-center gap-1.5 rounded-lg bg-gray-800 px-2.5 py-1 text-xs font-semibold text-gray-300 transition-colors hover:bg-gray-700"
    >
      <span class="icon-[heroicons--arrow-path] h-4 w-4 {refreshing ? 'animate-spin' : ''}"></span>
      {refreshing ? "Updating..." : "Refresh"}
    </button>
  </div>

  {#if loading && matches.length === 0}
    <div class="mt-3 min-h-0 flex-1 overflow-y-auto overflow-x-auto rounded-lg border border-gray-800 bg-black/50 scrollbar-thin scrollbar-thumb-gray-800 scrollbar-track-gray-950">
      {#each Array(5) as _}
        <div class="grid min-w-[760px] grid-cols-[260px_310px_1fr] items-center gap-3 border-b border-gray-800/80 px-4 py-2.5 last:border-b-0">
          <div class="flex items-center gap-2.5">
            <div class="h-12 w-12 animate-pulse rounded-full bg-gray-800"></div>
            <div class="space-y-1.5">
              <div class="h-4 w-24 animate-pulse rounded bg-gray-800"></div>
              <div class="h-3 w-28 animate-pulse rounded bg-gray-800"></div>
              <div class="h-3 w-16 animate-pulse rounded bg-gray-800"></div>
            </div>
          </div>
          <div class="space-y-2">
            <div class="flex gap-1">
              {#each Array(7) as _}
                <div class="h-8 w-8 animate-pulse rounded bg-gray-800"></div>
              {/each}
            </div>
            <div class="mx-auto h-3 w-48 animate-pulse rounded bg-gray-800"></div>
          </div>
          <div class="space-y-2">
            <div class="h-3 w-32 animate-pulse rounded bg-gray-800"></div>
            <div class="h-3 w-44 animate-pulse rounded bg-gray-800"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if error && matches.length === 0}
    <div class="flex-1 flex flex-col items-center justify-center p-12 text-center text-red-400 text-sm gap-2">
      <span class="icon-[heroicons--exclamation-circle] h-8 w-8 text-red-500"></span>
      <p>Failed to load match history: {error}</p>
      <p class="text-xs text-gray-500">Make sure League of Legends client is running and logged in.</p>
    </div>
  {:else if matches.length === 0}
    <div class="flex-1 flex items-center justify-center p-12 text-gray-500 text-sm">
      No recent matches found.
    </div>
  {:else}
    <div class="mt-3 min-h-0 flex-1 overflow-y-auto overflow-x-auto rounded-lg border border-gray-800 bg-black/50 scrollbar-thin scrollbar-thumb-gray-800 scrollbar-track-gray-950">
      {#if error}
        <div class="border-b border-red-500/30 bg-red-950/20 px-3 py-1.5 text-xs text-red-300">
          Showing cached history. Refresh failed: {error}
        </div>
      {/if}
      {#each visibleMatches as match (match.game_id)}
        <article class="history-card relative grid min-w-[760px] grid-cols-[250px_310px_1fr] items-center gap-4 overflow-hidden border-b border-gray-800/80 px-4 py-2.5 last:border-b-0 hover:bg-gray-900/80">
          {#if match.win}
            <div class="pointer-events-none absolute inset-0 bg-gradient-to-r from-cyan-500/14 via-cyan-500/5 to-cyan-500/0"></div>
            <div class="pointer-events-none absolute inset-y-0 left-0 w-1 bg-emerald-300/70 shadow-[0_0_14px_2px_rgba(52,211,153,0.35)]"></div>
          {:else}
            <div class="pointer-events-none absolute inset-0 bg-gradient-to-r from-red-500/14 via-red-500/5 to-red-500/0"></div>
            <div class="pointer-events-none absolute inset-y-0 left-0 w-1 bg-red-400/70 shadow-[0_0_14px_2px_rgba(248,113,113,0.35)]"></div>
          {/if}
          <div class="relative z-10 flex items-center gap-2.5">
            <div class="relative h-[60px] w-[60px] shrink-0">
              <div class="h-full w-full overflow-hidden rounded-full border-2 border-amber-500/60 bg-gray-900">
                {#if match.champion?.square_icon_url}
                  <img src={match.champion.square_icon_url} alt={match.champion.name} class="h-full w-full scale-110 object-cover" />
                {:else}
                  <div class="h-full w-full bg-gray-800"></div>
                {/if}
              </div>
              <span class="absolute -bottom-1 -right-1 flex h-5 min-w-5 items-center justify-center rounded-full border border-amber-400 bg-gray-950 px-1 text-[10px] font-bold text-gray-200">
                {match.champ_level}
              </span>
            </div>
            <div class="min-w-0">
              <div class="text-[11px] font-bold uppercase tracking-[0.14em] {match.win ? 'text-cyan-400' : 'text-red-400'}">
                {match.win ? "Victory" : "Defeat"}
              </div>
              <div class="truncate text-[15px] font-medium text-gray-200">{match.champion?.name ?? "Unknown"}</div>
              <div class="truncate text-xs text-gray-500">{formatGameMode(match.game_mode)}</div>
            </div>
          </div>

          <div class="relative z-10 flex flex-col items-end gap-1">
            <div class="flex h-8 justify-end gap-0.5" aria-label="Items">
              {#each match.items as item, itemIndex (item?.id ?? `empty-${match.game_id}-${itemIndex}`)}
                {#if item}
                  {@const itemDomId = `mh-${match.game_id}-item-${itemIndex}-${item.id}`}
                  <img
                    id={itemDomId}
                    src={item.icon_url}
                    alt={item.name}
                    class="h-8 w-8 rounded-sm border border-gray-700 object-cover cursor-help"
                  />
                  <GameEntityPopover
                    targetId={itemDomId}
                    name={item.name}
                    iconUrl={item.icon_url}
                    description={item.description}
                    priceTotal={item.price_total}
                    placement="top"
                  />
                {:else}
                  <span class="h-8 w-8 rounded-sm border border-gray-800 bg-gray-950/80"></span>
                {/if}
              {/each}
            </div>
            <div class="text-right text-base font-semibold tracking-wide text-gray-300">
              {match.kills} <span class="text-gray-600">/</span> {match.deaths} <span class="text-gray-600">/</span> {match.assists}
            </div>
            <div class="flex items-baseline justify-end gap-2 text-right text-[11px] uppercase tracking-wider text-gray-500">
              <span>{match.minions_killed} CS</span>
              <span>{match.gold_earned.toLocaleString()} gold</span>
              {#if match.damage_dealt_to_champions !== null}
                <span title="Damage dealt to champions">{match.damage_dealt_to_champions.toLocaleString()} damage</span>
              {/if}
            </div>
          </div>

          <div class="relative z-10 flex items-center justify-center gap-3 text-center text-xs text-gray-400">
            <div>
              <div class="mt-0.5 text-sm text-gray-200">{formatMap(match.map_id)}</div>
              <div class="mt-0.5">{formatDuration(match.game_duration_secs)} <span class="mx-1 text-gray-600">•</span> {formatDate(match.game_creation_date)}</div>
              {#if match.augments?.length}
                <div class="mt-1.5 flex items-center justify-center gap-1.5" title="Augments">
                  {#each match.augments as augment, augIndex (augment.id)}
                    {@const augDomId = `mh-${match.game_id}-aug-${augIndex}-${augment.id.replace(/[^a-zA-Z0-9_-]/g, '_')}`}
                    <img
                      id={augDomId}
                      src={augment.icon_url}
                      alt={augment.name}
                      class="h-8 w-8 rounded-sm border border-purple-500/60 object-cover cursor-help"
                    />
                    <GameEntityPopover
                      targetId={augDomId}
                      name={augment.name}
                      iconUrl={augment.icon_url}
                      description={augment.tooltip}
                      rarity={augment.rarity}
                      placement="top"
                    />
                  {/each}
                </div>
              {:else}
                <div class="mt-1.5 text-[10px] uppercase tracking-wider text-gray-600">No augment data</div>
              {/if}
            </div>
          </div>
        </article>
      {/each}
    </div>
    <div class="flex shrink-0 items-center justify-between pt-3 text-xs text-gray-400">
      <span>Showing {(currentPage - 1) * pageSize + 1}-{Math.min(currentPage * pageSize, matches.length)} of {matches.length} matches</span>
      <div class="flex items-center gap-2">
        <button
          type="button"
          disabled={currentPage === 1}
          onclick={() => (currentPage = Math.max(1, currentPage - 1))}
          class="rounded-lg border border-gray-700 bg-gray-900 px-2.5 py-1 transition-colors hover:bg-gray-800 disabled:cursor-not-allowed disabled:opacity-40"
        >
          Previous
        </button>
        <span class="min-w-16 text-center">Page {currentPage} of {totalPages}</span>
        <button
          type="button"
          disabled={currentPage === totalPages}
          onclick={() => (currentPage = Math.min(totalPages, currentPage + 1))}
          class="rounded-lg border border-gray-700 bg-gray-900 px-2.5 py-1 transition-colors hover:bg-gray-800 disabled:cursor-not-allowed disabled:opacity-40"
        >
          Next
        </button>
      </div>
    </div>
  {/if}
</div>
