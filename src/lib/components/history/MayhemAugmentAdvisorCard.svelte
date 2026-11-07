<script lang="ts">
  import type {
    AugmentCardRecommendation,
    RerollEvRecommendation,
    AugmentRecommendation,
    LiveGameState,
    MayhemBuildItem,
  } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";

  let {
    championId = 0,
    championName = "",
    evaluatedCards = [],
    rerollEv = null,
    recommendedAugmentsByTier = {},
    liveGameState = null,
    commandLineReroll = false,
    activeAugmentSlot = 1,
    knownItemMap = new Map<number, MayhemBuildItem>(),
    onSelectAugment,
    onToggleCommandLineReroll,
    onSyncLiveInventory,
    onSlotChange,
  }: {
    championId?: number;
    championName?: string;
    evaluatedCards?: AugmentCardRecommendation[];
    rerollEv?: RerollEvRecommendation | null;
    recommendedAugmentsByTier?: Record<string, AugmentRecommendation[]>;
    liveGameState?: LiveGameState | null;
    commandLineReroll?: boolean;
    activeAugmentSlot?: number;
    knownItemMap?: Map<number, MayhemBuildItem>;
    onSelectAugment?: (augId: string, name: string) => void;
    onToggleCommandLineReroll?: () => void;
    onSyncLiveInventory?: () => void;
    onSlotChange?: (slot: number) => void;
  } = $props();

  let activeTierTab = $state<"Silver" | "Gold" | "Prismatic">("Gold");

  function formatPct(val: number | undefined): string {
    if (val === undefined || Number.isNaN(val)) return "0.0%";
    const pct = val > 1 ? val : val * 100;
    return `${pct.toFixed(1)}%`;
  }

  function formatWpa(val: number | undefined): string {
    if (val === undefined || Number.isNaN(val)) return "+0.0%";
    const pct = Math.abs(val) <= 1 ? val * 100 : val;
    return `${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%`;
  }

  function getTierBorderClass(tier: string | undefined): string {
    const t = (tier || "").toLowerCase();
    if (t === "prismatic") return "border-[#e78fff] shadow-[0_0_15px_rgba(242,194,255,0.25)]";
    if (t === "gold") return "border-[#e3c49f] shadow-[0_0_10px_rgba(227,196,159,0.2)]";
    return "border-[#a3b9bd]";
  }

  function getItemIconUrl(itemId: number): string {
    return `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${itemId}.png`.toLowerCase();
  }

  const isAugmentRound = $derived(
    Boolean(liveGameState?.is_augment_checkpoint ?? liveGameState?.isAugmentCheckpoint)
  );

  const liveAugmentSlot = $derived(
    liveGameState?.augment_slot ?? liveGameState?.augmentSlot ?? null
  );

  const displaySlot = $derived(liveAugmentSlot ?? activeAugmentSlot);

  const checkpointLabel = $derived.by(() => {
    switch (displaySlot) {
      case 1:
        return "Level 3 Augment Draft (Slot 1)";
      case 2:
        return "Level 7 Augment Draft (Slot 2)";
      case 3:
        return "Level 11 Augment Draft (Slot 3)";
      case 4:
        return "Level 15 Augment Draft (Slot 4)";
      default:
        return `Augment Slot ${displaySlot}`;
    }
  });

  const tierAugments = $derived(
    recommendedAugmentsByTier[activeTierTab] ?? []
  );
</script>

<div class="rounded-xl border border-gray-800 bg-gray-900/80 p-4 shadow-xl backdrop-blur-sm space-y-3.5">
  <!-- Header: Live Checkpoint, Reroll Toggle, Slot Selector -->
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-gray-800 pb-2.5">
    <div class="flex items-center gap-2">
      <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-purple-500/10 text-purple-400 border border-purple-500/30">
        <span class="icon-[heroicons--sparkles] text-sm"></span>
      </div>
      <div>
        <h3 class="text-xs font-bold uppercase tracking-wider text-gray-200 flex items-center gap-2">
          3-Card Augment Draft & Reroll Advisor
          {#if isAugmentRound}
            <span class="rounded bg-emerald-950/80 border border-emerald-600/70 px-1.5 py-0.2 text-[9px] font-bold text-emerald-300 animate-pulse">
              LIVE CHECKPOINT ACTIVE
            </span>
          {/if}
        </h3>
        <p class="text-[10px] text-gray-400">
          {checkpointLabel} · Bayesian score conditioned on active items & synergies
        </p>
      </div>
    </div>

    <!-- Controls: Command Line Reroll Toggle & Slot Selector -->
    <div class="flex items-center gap-2">
      <!-- Slot Switcher Buttons -->
      <div class="flex items-center rounded-lg bg-gray-950/60 p-0.5 border border-gray-800 text-[10px] font-mono">
        {#each [1, 2, 3, 4] as s}
          <button
            type="button"
            onclick={() => onSlotChange?.(s)}
            class="px-2 py-0.5 rounded transition-colors {displaySlot === s
              ? 'bg-amber-500/20 text-amber-300 font-bold border border-amber-500/50'
              : 'text-gray-400 hover:text-gray-200'}"
          >
            S{s}
          </button>
        {/each}
      </div>

      <!-- Command Line Headliner Toggle -->
      <button
        type="button"
        onclick={() => onToggleCommandLineReroll?.()}
        class="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] font-medium transition-colors border {commandLineReroll
          ? 'bg-purple-950/70 border-purple-500 text-purple-200 shadow-sm'
          : 'bg-gray-800/40 border-gray-700 text-gray-400 hover:text-gray-200'}"
        title="Toggle Command Line Headliner (+1 Tier Reroll Upgrade)"
      >
        <span class="icon-[heroicons--command-line] text-xs {commandLineReroll ? 'text-purple-400' : 'text-gray-500'}"></span>
        <span>Command Line</span>
      </button>

      <!-- Sync Live Items Button -->
      {#if onSyncLiveInventory && liveGameState?.is_in_game}
        <button
          type="button"
          onclick={onSyncLiveInventory}
          class="inline-flex items-center gap-1 rounded-lg bg-cyan-950/50 border border-cyan-600/60 px-2 py-1 text-[10px] font-semibold text-cyan-300 hover:bg-cyan-900/60 transition-colors"
          title="Auto-sync live client inventory items into simulator"
        >
          <span class="icon-[heroicons--arrow-path] text-xs"></span>
          <span>Sync Live ({liveGameState.items?.length ?? 0})</span>
        </button>
      {/if}
    </div>
  </div>

  <!-- 3-Card Evaluated Picks (if cards are offered) -->
  {#if evaluatedCards && evaluatedCards.length > 0}
    <div class="space-y-2">
      <!-- Reroll EV Banner -->
      {#if rerollEv}
        <div class="flex items-center justify-between rounded-lg bg-gray-950/70 border border-gray-800 px-3 py-1.5 text-xs">
          <div class="flex items-center gap-2">
            <span class="icon-[heroicons--arrow-path-rounded-square] text-amber-400 text-sm"></span>
            <span class="text-gray-300 font-medium text-[11px]">
              Reroll EV ({rerollEv.target_tier ?? rerollEv.targetTier} Pool):
              <strong class="font-mono text-amber-300">{formatWpa(rerollEv.ev_reroll ?? rerollEv.evReroll)}</strong>
            </span>
          </div>
          <span class="text-[10px] font-mono text-gray-400">
            {rerollEv.pool_size ?? rerollEv.poolSize} unseen candidates
          </span>
        </div>
      {/if}

      <!-- 3-Card Cards Grid -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        {#each evaluatedCards as card, cIdx (`card-${card.augment_id ?? card.augmentId}-${cIdx}`)}
          {@const isBest = (card.verdict === "best_pick")}
          {@const isReroll = (card.verdict === "reroll")}
          {@const tier = card.tier || "Gold"}
          {@const partnerId = card.lookahead_partner_id ?? card.lookaheadPartnerId}
          {@const partnerKind = card.lookahead_partner_kind ?? card.lookaheadPartnerKind}
          {@const synLift = card.lookahead_synergy_lift ?? card.lookaheadSynergyLift}

          <div
            class="relative flex flex-col justify-between rounded-xl border-2 p-3 bg-gray-950/60 transition-all hover:bg-gray-950/90 {getTierBorderClass(tier)} {isBest
              ? 'ring-2 ring-emerald-500/50 bg-emerald-950/10'
              : ''}"
          >
            <!-- Card Header: Icon + Name + Verdict Badge -->
            <div>
              <div class="flex items-start justify-between gap-2 mb-2">
                <div class="flex items-center gap-2">
                  <AugmentIcon
                    src={card.icon_url ?? card.iconUrl}
                    name={card.name}
                    rarity={tier}
                    class="h-10 w-10 shrink-0"
                    roundedClass="rounded-lg"
                  />
                  <div>
                    <h4 class="text-xs font-bold text-gray-100 leading-tight">
                      {card.name}
                    </h4>
                    <span class="text-[10px] font-mono text-gray-400">
                      Tier: {tier}
                    </span>
                  </div>
                </div>

                <!-- Verdict Badge -->
                {#if isBest}
                  <span class="rounded bg-emerald-950/90 border border-emerald-500 px-1.5 py-0.5 text-[9px] font-bold text-emerald-300 shadow-sm shrink-0">
                    BEST PICK
                  </span>
                {:else if isReroll}
                  <span class="rounded bg-rose-950/80 border border-rose-500/70 px-1.5 py-0.5 text-[9px] font-bold text-rose-300 shadow-sm shrink-0">
                    REROLL
                  </span>
                {:else}
                  <span class="rounded bg-gray-800 border border-gray-700 px-1.5 py-0.5 text-[9px] font-semibold text-gray-300 shrink-0">
                    HOLD
                  </span>
                {/if}
              </div>

              <!-- Metrics Row: Score, Kit Lift, LCB -->
              <div class="grid grid-cols-3 gap-1.5 rounded-lg bg-gray-900/60 border border-gray-800/80 p-2 text-center my-2 text-[10px]">
                <div>
                  <div class="text-[9px] text-gray-400">Score</div>
                  <div class="font-mono font-bold {(card.score ?? 0) >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                    {formatWpa(card.score)}
                  </div>
                </div>
                <div>
                  <div class="text-[9px] text-gray-400">Kit Lift</div>
                  <div class="font-mono font-bold text-cyan-300">
                    {formatWpa(card.kit_lift ?? card.kitLift)}
                  </div>
                </div>
                <div>
                  <div class="text-[9px] text-gray-400">Grade</div>
                  <div class="font-mono font-bold text-amber-300">
                    {card.confidence_grade ?? card.confidenceGrade ?? "B"}
                  </div>
                </div>
              </div>

              <!-- Tooltip excerpt -->
              {#if card.tooltip}
                <p class="text-[10px] text-gray-400 line-clamp-2 italic mb-2">
                  {@html card.tooltip.replace(/<[^>]*>?/gm, '')}
                </p>
              {/if}

              <!-- Active Combos with Slotted Items/Augments -->
              {#if card.active_combos && card.active_combos.length > 0}
                <div class="flex flex-wrap gap-1 mb-2">
                  {#each card.active_combos as combo}
                    <span class="rounded bg-purple-950/70 border border-purple-600/50 px-1.5 py-0.2 text-[9px] font-mono text-purple-200">
                      +{formatPct(combo.synergy_lift ?? combo.synergyLift)} with build
                    </span>
                  {/each}
                </div>
              {/if}

              <!-- Lookahead Combo Callout -->
              {#if partnerId && synLift}
                <div class="flex items-center gap-1.5 rounded bg-amber-950/30 border border-amber-800/40 p-1.5 text-[10px] text-amber-300">
                  <span class="icon-[heroicons--key] text-xs text-amber-400 shrink-0"></span>
                  <span class="truncate">
                    Unlocks {partnerKind === 'item' ? 'Item' : 'Aug'} #{partnerId} (+{formatPct(synLift)} Δsyn)
                  </span>
                  {#if partnerKind === 'item'}
                    <img
                      src={getItemIconUrl(partnerId)}
                      alt="Unlock item"
                      class="h-4 w-4 rounded shrink-0 border border-amber-600/50"
                    />
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Action Button: Pick Card -->
            <button
              type="button"
              onclick={() => onSelectAugment?.(card.augment_id ?? card.augmentId, card.name)}
              class="mt-3 w-full rounded-lg py-1 text-xs font-bold transition-colors border {isBest
                ? 'bg-emerald-600 hover:bg-emerald-500 text-white border-emerald-400 shadow-md'
                : 'bg-gray-800/60 hover:bg-gray-800 text-gray-200 border-gray-700'}"
            >
              {isBest ? 'Select Best Pick' : 'Equip into Slot'}
            </button>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <!-- Opening Level 3 Tier Guide when no 3 cards are drafted yet -->
    <div class="space-y-2.5">
      <div class="flex items-center justify-between border-b border-gray-800/80 pb-2">
        <span class="text-[11px] font-bold text-gray-300">
          Top Recommended Augments for {championName || 'Selected Champion'}
        </span>

        <!-- Tier Tabs -->
        <div class="flex items-center gap-1 text-[10px]">
          {#each ["Silver", "Gold", "Prismatic"] as t}
            <button
              type="button"
              onclick={() => (activeTierTab = t as "Silver" | "Gold" | "Prismatic")}
              class="px-2 py-0.5 rounded font-bold transition-colors border {activeTierTab === t
                ? 'bg-amber-500/20 text-amber-300 border-amber-500/60'
                : 'text-gray-400 hover:text-gray-200 border-transparent'}"
            >
              {t}
            </button>
          {/each}
        </div>
      </div>

      <!-- Tier Guide Items Grid -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
        {#each tierAugments as aug (aug.augment_id ?? aug.augmentId)}
          {@const pId = aug.lookahead_partner_id ?? aug.lookaheadPartnerId}
          {@const pKind = aug.lookahead_partner_kind ?? aug.lookaheadPartnerKind}
          {@const pSyn = aug.lookahead_synergy_lift ?? aug.lookaheadSynergyLift}

          <div
            role="button"
            tabindex="0"
            onclick={() => onSelectAugment?.(aug.augment_id ?? aug.augmentId, aug.name)}
            onkeydown={(e) => { if (e.key === 'Enter') onSelectAugment?.(aug.augment_id ?? aug.augmentId, aug.name); }}
            class="flex items-center justify-between gap-2 p-2 rounded-lg border border-gray-800 bg-gray-950/40 hover:border-gray-700 hover:bg-gray-950/80 transition-all cursor-pointer group"
          >
            <div class="flex items-center gap-2 min-w-0">
              <AugmentIcon
                src={aug.icon_url ?? aug.iconUrl}
                name={aug.name}
                rarity={aug.tier}
                class="h-8 w-8 shrink-0"
                roundedClass="rounded-lg"
              />
              <div class="min-w-0">
                <div class="text-[11px] font-bold text-gray-200 truncate group-hover:text-amber-300">
                  {aug.name}
                </div>
                <div class="flex items-center gap-1.5 text-[9px] font-mono">
                  <span class="text-emerald-400 font-bold">{formatWpa(aug.wpa)}</span>
                  <span class="text-cyan-300">Kit: +{formatPct(aug.kit_lift ?? aug.kitLift)}</span>
                </div>
              </div>
            </div>

            <!-- Grade & Lookahead Indicator -->
            <div class="flex flex-col items-end shrink-0">
              <span class="rounded bg-gray-800 border border-gray-700 px-1.5 py-0.2 text-[9px] font-mono font-bold text-amber-300">
                {aug.confidence_grade ?? aug.confidenceGrade ?? "A"}
              </span>
              {#if pId && pSyn}
                <span class="text-[8px] font-mono text-purple-300 mt-0.5">
                  +{formatPct(pSyn)} syn
                </span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
