<script lang="ts">
  import type { MayhemStageItem } from "../../types/champ_select";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import SituationalTagBadge from "../common/SituationalTagBadge.svelte";

  let {
    stages = [],
  }: {
    stages?: MayhemStageItem[];
  } = $props();

  const stageColors = [
    { border: "border-sky-500/40", bg: "bg-sky-950/20", text: "text-sky-300", badge: "bg-sky-500/20 text-sky-200" },
    { border: "border-amber-500/40", bg: "bg-amber-950/20", text: "text-amber-300", badge: "bg-amber-500/20 text-amber-200" },
    { border: "border-purple-500/40", bg: "bg-purple-950/20", text: "text-purple-300", badge: "bg-purple-500/20 text-purple-200" },
    { border: "border-emerald-500/40", bg: "bg-emerald-950/20", text: "text-emerald-300", badge: "bg-emerald-500/20 text-emerald-200" },
  ];
</script>

{#if stages.length > 0}
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-1.5 text-xs font-semibold text-gray-300 uppercase tracking-wider">
        <span class="icon-[heroicons--arrow-trending-up] text-purple-400"></span>
        Stage-Based Build Pathing
      </div>
      <span class="text-[11px] text-gray-400">Breakpoint Synergy: Lv 3 → Lv 7 → Lv 11 → Lv 15+</span>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2.5">
      {#each stages as st, idx}
        {@const color = stageColors[idx % stageColors.length]}
        <div class="flex flex-col justify-between p-2.5 rounded-xl border {color.border} {color.bg} backdrop-blur-sm shadow-sm transition hover:border-opacity-80">
          <!-- Header -->
          <div class="flex items-start justify-between gap-1.5 mb-2">
            <div>
              <div class="text-xs font-bold {color.text} leading-tight">{st.title}</div>
              <div class="text-[10px] text-gray-400 font-mono mt-0.5">{st.timing}</div>
            </div>
            <span class="px-1.5 py-0.5 rounded text-[10px] font-bold font-mono {color.badge}">
              S{st.stage}
            </span>
          </div>

          <!-- Items row -->
          <div class="flex flex-wrap items-center gap-2 pt-1 border-t border-gray-800/60">
            {#if st.items && st.items.length > 0}
              {#each st.items as it}
                {@const targetId = `stage-it-${st.stage}-${it.id}`}
                <div class="relative group">
                  <div
                    id={targetId}
                    class="relative flex flex-col items-center p-1 rounded-lg border border-gray-700/60 bg-gray-900/80 hover:border-gray-500 transition cursor-help"
                  >
                    {#if it.icon_url}
                      <img src={it.icon_url} alt={it.name} class="w-8 h-8 rounded-md object-cover" />
                    {:else}
                      <div class="w-8 h-8 rounded-md bg-gray-800 flex items-center justify-center text-[10px] text-gray-400">
                        {it.name.slice(0, 2)}
                      </div>
                    {/if}
                    <span class="text-[9px] font-mono text-amber-300 mt-0.5">{it.price_total}g</span>

                    <SituationalTagBadge tag={it.situational_tag} />
                  </div>

                  <GameEntityPopover
                    {targetId}
                    name={it.name}
                    iconUrl={it.icon_url}
                    description={it.description}
                    itemClass={it.item_class}
                    priceTotal={it.price_total}
                    games={it.games}
                    placement="top"
                  />
                </div>
              {/each}
            {:else}
              <div class="text-[11px] text-gray-400 italic py-1">No items specified</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
