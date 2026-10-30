<script lang="ts">
  import type { ChampSelectSession } from "../../types/champ_select";
  import type { ChampionSummary } from "../../types/champions";

  let {
    session,
    allChampions = [],
    selectedChampionId = 0,
    onSelectChampion,
    onSwapChampion,
  }: {
    session: ChampSelectSession | null;
    allChampions: ChampionSummary[];
    selectedChampionId: number;
    onSelectChampion: (championId: number) => void;
    onSwapChampion: (championId: number) => void;
  } = $props();

  function getChamp(id: number) {
    return allChampions.find((c) => c.id === id);
  }
</script>

<div class="space-y-3">
  <!-- Allied Team Roster -->
  <div class="rounded-xl border border-gray-800 bg-gray-900/60 p-3.5 backdrop-blur-sm">
    <div class="flex items-center justify-between mb-2.5">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400 flex items-center gap-1.5">
        <span class="icon-[heroicons--user-group] h-4 w-4 text-purple-400"></span>
        Allied Team ({session?.myTeam?.length ?? 0}/5)
      </h2>
      <span class="text-[11px] text-gray-500">Click to inspect or swap</span>
    </div>
    <div class="grid grid-cols-5 gap-2">
      {#each session?.myTeam ?? [] as member (member.cellId)}
        {@const isMe = member.cellId === session?.localPlayerCellId}
        {@const champCatalog = getChamp(member.championId)}
        {@const isInspected = selectedChampionId === member.championId}
        <button
          type="button"
          title={isMe ? "Your current pick" : "Click to request swap / inspect"}
          onclick={() => {
            if (member.championId <= 0) return;
            if (isMe) {
              onSelectChampion(member.championId);
            } else {
              onSwapChampion(member.championId);
            }
          }}
          class="relative flex flex-col items-center justify-center p-2 rounded-xl border transition-all cursor-pointer {isInspected
            ? 'border-purple-500 bg-purple-950/40 shadow-[0_0_12px_rgba(168,85,247,0.3)]'
            : isMe
            ? 'border-cyan-500/70 bg-cyan-950/30'
            : 'border-gray-800 bg-gray-900/80 hover:border-gray-700'}"
        >
          {#if isMe}
            <span class="absolute -top-2 left-1/2 -translate-x-1/2 rounded bg-cyan-600 px-1 py-0.2 text-[8px] font-bold uppercase tracking-wider text-white">
              YOU
            </span>
          {/if}
          <div class="relative h-11 w-11 rounded-lg overflow-hidden border border-gray-700 bg-gray-950 mb-1">
            {#if champCatalog?.square_icon_url}
              <img
                src={champCatalog.square_icon_url}
                alt="Champion"
                class="h-full w-full object-cover"
              />
            {:else}
              <div class="flex h-full w-full items-center justify-center text-gray-600">
                <span class="icon-[heroicons--user] h-6 w-6"></span>
              </div>
            {/if}
          </div>
          <span class="text-[11px] font-semibold text-gray-200 truncate max-w-full">
            {champCatalog?.name || (member.championId > 0 ? `Champ #${member.championId}` : "Picking...")}
          </span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Bench / Available Champions -->
  {#if (session?.benchChampions?.length ?? 0) > 0}
    <div class="rounded-xl border border-gray-800 bg-gray-900/60 p-3.5 backdrop-blur-sm">
      <div class="flex items-center justify-between mb-2.5">
        <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400 flex items-center gap-1.5">
          <span class="icon-[heroicons--arrow-path-rounded-square] h-4 w-4 text-cyan-400"></span>
          Available Bench ({session?.benchChampions?.length ?? 0})
        </h2>
        <span class="text-[11px] text-gray-500">Click to swap champion</span>
      </div>
      <div class="flex flex-wrap gap-2">
        {#each session?.benchChampions ?? [] as bench (bench.championId)}
          {@const champCatalog = getChamp(bench.championId)}
          {@const isInspected = selectedChampionId === bench.championId}
          <button
            type="button"
            title="Click to swap this champion from bench"
            onclick={() => onSwapChampion(bench.championId)}
            class="flex items-center gap-2 p-1.5 pr-2.5 rounded-lg border transition-all cursor-pointer {isInspected
              ? 'border-purple-500 bg-purple-950/40 shadow-sm'
              : 'border-gray-800 bg-gray-900/80 hover:border-gray-700'}"
          >
            <img
              src={champCatalog?.square_icon_url || `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/${bench.championId}.png`}
              alt="Bench"
              class="h-7 w-7 rounded-md object-cover bg-gray-950 border border-gray-700"
            />
            <span class="text-[11px] font-semibold text-gray-200 leading-tight">
              {champCatalog?.name || `Champ #${bench.championId}`}
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>
