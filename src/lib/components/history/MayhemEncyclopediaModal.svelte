<script lang="ts">
  let {
    isOpen = false,
    onClose,
    onSimulateAugment,
  }: {
    isOpen: boolean;
    onClose: () => void;
    onSimulateAugment?: (augmentName: string) => void;
  } = $props();

  let activeTab = $state<"rules" | "items" | "warps">("rules");

  const buildWarpAugments = [
    {
      name: "Marksmage",
      tier: "Prismatic",
      color: "border-purple-500/50 text-purple-300",
      description: "Basic attacks deal bonus magic damage equal to 75% - 100% AP.",
      warpSummary: "AD Marksmen build AP / On-Hit: Nashor's Tooth, Lich Bane, Rabadon's Deathcap, Guinsoo's.",
    },
    {
      name: "ADAPt",
      tier: "Silver",
      color: "border-gray-400/50 text-gray-300",
      description: "Converts all bonus AD into AP (1 AP per 0.6 bonus AD) + 15% total AP.",
      warpSummary: "Build high-AD items (Infinity Edge, Bloodthirster) which convert into astronomical AP power.",
    },
    {
      name: "EscAPADe",
      tier: "Silver",
      color: "border-gray-400/50 text-gray-300",
      description: "Converts all AP into bonus AD (1 AD per 1.66 AP) + 15% total AD.",
      warpSummary: "Stack high AP items (Rabadon's, Shadowflame) which convert into massive physical AD.",
    },
    {
      name: "Big Shot",
      tier: "Prismatic",
      color: "border-purple-500/50 text-purple-300",
      description: "Converts 75% attack speed into crit damage + 25% bonus AD + 35% crit damage.",
      warpSummary: "Attack speed stacking converts directly into gargantuan single-hit critical strikes.",
    },
    {
      name: "Apex Inventor",
      tier: "Gold",
      color: "border-amber-500/50 text-amber-300",
      description: "Grants 100 Item Haste (50% item cooldown reduction).",
      warpSummary: "Prioritizes active/proc items: Heartsteel, Sundered Sky, Unending Despair, Eclipse, Zhonya's.",
    },
    {
      name: "Vulnerability",
      tier: "Gold",
      color: "border-amber-500/50 text-amber-300",
      description: "Item and damage-over-time effects can critically strike (+20% crit chance).",
      warpSummary: "Liandry's Torment, Blackfire Torch, and Sunfire burn procs critically strike with Infinity Edge.",
    },
    {
      name: "Goliath",
      tier: "Prismatic",
      color: "border-purple-500/50 text-purple-300",
      description: "Grants 35% bonus health, 15% adaptive force, and 50% increased size.",
      warpSummary: "Hyper-scales HP tanks: Heartsteel, Warmog's Armor, Atma's Reckoning, and Jak'Sho.",
    },
    {
      name: "Draw Your Sword",
      tier: "Prismatic",
      color: "border-purple-500/50 text-purple-300",
      description: "Ranged champions become melee, gaining 24% AD, 20% AS, 24% HP, 20% MS, and 20% lifesteal.",
      warpSummary: "Ranged champions unlock melee-only Hydra cleave and bruiser durability items.",
    },
  ];

  const exclusiveItems = [
    {
      name: "Atma's Reckoning",
      price: "3000g",
      stats: "+700 Health, +30 Armor, +30 MR",
      passive: "Intimidate: Gain bonus Attack Damage equal to 2% of bonus health.",
      badge: "Mayhem Legendary",
    },
    {
      name: "Rite of Ruin",
      price: "2800g",
      stats: "+70 AP, +400 Mana, +25 Ability Haste",
      passive: "Casting spells grants shields to nearby allies and bursts damage upon expiration.",
      badge: "Mayhem Legendary",
    },
    {
      name: "Sword of Blossoming Dawn",
      price: "2700g",
      stats: "+40 AP, +20% Attack Speed, +200 Health, +15 AH",
      passive: "Basic attacks heal lowest health nearby ally based on AP and attack speed.",
      badge: "Mayhem Legendary",
    },
    {
      name: "Wooglet's Witchcap",
      price: "Quest Reward",
      stats: "Massive AP + Zhonya's Stasis Active",
      passive: "Combines Rabadon's Deathcap AP amplification with Zhonya's Hourglass 2.5s invulnerability.",
      badge: "Augment Quest",
    },
    {
      name: "Golden Spatula",
      price: "Quest Reward",
      stats: "All-Rounder Capstone (AD, AP, AS, Armor, MR, MS)",
      passive: "Empowers all basic attacks and abilities with hybrid damage and stat multipliers.",
      badge: "Augment Quest",
    },
    {
      name: "Ultra Hydra",
      price: "Quest Reward",
      stats: "Omni-Cleave Physical",
      passive: "Cleave radius and damage dramatically increased, scaling with both AD and maximum Health.",
      badge: "Augment Quest",
    },
  ];

  const itemLockGroups = [
    { group: "Hydra", items: "Ravenous Hydra, Titanic Hydra, Profane Hydra, Stridebreaker (Max 1)" },
    { group: "Lifeline", items: "Sterak's Gage, Maw of Malmortius, Immortal Shieldbow (Max 1)" },
    { group: "Last Whisper", items: "Lord Dominik's Regards, Mortal Reminder, Serylda's Grudge, Terminus (Max 1)" },
    { group: "Crit Modifier", items: "Infinity Edge, Navori Flickerblade, Guinsoo's Rageblade (Max 1)" },
    { group: "Tear of Goddess", items: "Archangel's/Seraph's, Manamune/Muramana, Winter's Approach/Fimbulwinter (Max 1)" },
  ];
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-gray-950/80 backdrop-blur-sm animate-in fade-in duration-150">
    <div class="relative w-full max-w-4xl max-h-[85vh] flex flex-col rounded-2xl border border-gray-800 bg-gray-900 shadow-2xl overflow-hidden">
      <!-- Modal Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-gray-800 bg-gray-950/60">
        <div class="flex items-center gap-2.5">
          <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-purple-500/20 text-purple-300 border border-purple-500/30">
            <span class="icon-[heroicons--book-open] text-lg"></span>
          </div>
          <div>
            <h2 class="text-sm font-bold text-white tracking-wide">ARAM Mayhem Encyclopedia</h2>
            <p class="text-[11px] text-gray-400">Comprehensive mechanics, mode overrides, exclusive items & build warps</p>
          </div>
        </div>

        <button
          onclick={onClose}
          type="button"
          class="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-gray-800 transition"
          aria-label="Close"
        >
          <span class="icon-[heroicons--x-mark] text-xl"></span>
        </button>
      </div>

      <!-- Navigation Tabs -->
      <div class="flex border-b border-gray-800 bg-gray-950/40 px-6 gap-2 pt-2">
        <button
          onclick={() => activeTab = "rules"}
          class="px-4 py-2 text-xs font-semibold rounded-t-lg transition border-b-2 {activeTab === 'rules' ? 'border-purple-400 text-purple-200 bg-gray-800/60' : 'border-transparent text-gray-400 hover:text-gray-200'}"
        >
          Mode Overrides & Rules
        </button>
        <button
          onclick={() => activeTab = "items"}
          class="px-4 py-2 text-xs font-semibold rounded-t-lg transition border-b-2 {activeTab === 'items' ? 'border-purple-400 text-purple-200 bg-gray-800/60' : 'border-transparent text-gray-400 hover:text-gray-200'}"
        >
          Exclusive Items & Locks
        </button>
        <button
          onclick={() => activeTab = "warps"}
          class="px-4 py-2 text-xs font-semibold rounded-t-lg transition border-b-2 {activeTab === 'warps' ? 'border-purple-400 text-purple-200 bg-gray-800/60' : 'border-transparent text-gray-400 hover:text-gray-200'}"
        >
          Build-Warping Augments
        </button>
      </div>

      <!-- Modal Body (Scrollable) -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6 scrollbar-thin text-xs text-gray-300">
        {#if activeTab === "rules"}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Overcap Crit -->
            <div class="p-3.5 rounded-xl border border-gray-800 bg-gray-950/40 space-y-1.5">
              <div class="flex items-center gap-1.5 font-bold text-amber-300">
                <span class="icon-[heroicons--arrow-path-rounded-square]"></span>
                Overcap Critical Strike Conversion
              </div>
              <p class="text-gray-400 leading-relaxed">
                Every 1% Critical Strike chance in excess of 100% automatically converts to <span class="text-amber-200 font-semibold">+0.45 bonus AD</span> or <span class="text-purple-300 font-semibold">+0.75 AP</span> (Adaptive Force).
              </p>
            </div>

            <!-- Attack Speed Cap -->
            <div class="p-3.5 rounded-xl border border-gray-800 bg-gray-950/40 space-y-1.5">
              <div class="flex items-center gap-1.5 font-bold text-sky-300">
                <span class="icon-[heroicons--bolt]"></span>
                5.0 Attack Speed Cap
              </div>
              <p class="text-gray-400 leading-relaxed">
                The standard 2.5 attack speed limit is raised to <span class="text-sky-200 font-semibold">5.0</span>. On-hit and attack speed hyper-scalers (e.g. Lethal Tempo, Big Shot, Deft) can attack at lightning speed.
              </p>
            </div>

            <!-- Combo Breaker -->
            <div class="p-3.5 rounded-xl border border-gray-800 bg-gray-950/40 space-y-1.5">
              <div class="flex items-center gap-1.5 font-bold text-emerald-300">
                <span class="icon-[heroicons--shield-check]"></span>
                Combo Breaker (Anti-Chain CC)
              </div>
              <p class="text-gray-400 leading-relaxed">
                If immobilized for 5 out of the last 7 seconds, you are instantly cleansed of all crowd control and granted <span class="text-emerald-200 font-semibold">3 seconds of total CC immunity</span>.
              </p>
            </div>

            <!-- Free Presence of Mind -->
            <div class="p-3.5 rounded-xl border border-gray-800 bg-gray-950/40 space-y-1.5">
              <div class="flex items-center gap-1.5 font-bold text-indigo-300">
                <span class="icon-[heroicons--sparkles]"></span>
                Permanent Presence of Mind
              </div>
              <p class="text-gray-400 leading-relaxed">
                Runes are entirely disabled in Mayhem, but all champions possess the <span class="text-indigo-200 font-semibold">Presence of Mind</span> effect permanently.
              </p>
            </div>

            <!-- Headliners -->
            <div class="md:col-span-2 p-3.5 rounded-xl border border-purple-500/30 bg-purple-950/20 space-y-2">
              <div class="flex items-center gap-1.5 font-bold text-purple-200">
                <span class="icon-[heroicons--musical-note]"></span>
                Headliner Bands (Match Start Random Event)
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px]">
                <div class="p-2 rounded-lg bg-gray-900/60 border border-gray-800">
                  <span class="font-bold text-amber-300">COMMAND LINE:</span> Hacked Rerolls upgrade offering tier (Silver → Gold, Gold → Prismatic).
                </div>
                <div class="p-2 rounded-lg bg-gray-900/60 border border-gray-800">
                  <span class="font-bold text-rose-300">CAFE CUTIES:</span> Health relics heal 42% missing HP and extra relics spawn at base.
                </div>
                <div class="p-2 rounded-lg bg-gray-900/60 border border-gray-800">
                  <span class="font-bold text-purple-300">HEARTSTEEL:</span> Stickers on augment cards grant bonus anvils, AD/AP, or +1 Level cap.
                </div>
                <div class="p-2 rounded-lg bg-gray-900/60 border border-gray-800">
                  <span class="font-bold text-cyan-300">SPACE GROOVE:</span> Center disco ball explodes for up to 1,000 gold per player.
                </div>
              </div>
            </div>
          </div>
        {:else if activeTab === "items"}
          <div class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              {#each exclusiveItems as it}
                <div class="p-3 rounded-xl border border-gray-800 bg-gray-950/40 flex flex-col justify-between">
                  <div>
                    <div class="flex items-center justify-between mb-1">
                      <span class="font-bold text-white text-xs">{it.name}</span>
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-amber-500/20 text-amber-300 border border-amber-500/30">{it.price}</span>
                    </div>
                    <div class="text-[10px] text-gray-400 font-mono mb-1">{it.stats}</div>
                    <p class="text-[11px] text-gray-300 leading-normal">{it.passive}</p>
                  </div>
                  <div class="mt-2 pt-1.5 border-t border-gray-800/60 flex items-center justify-between text-[10px]">
                    <span class="text-purple-400 font-semibold">{it.badge}</span>
                  </div>
                </div>
              {/each}
            </div>

            <!-- Item Lock Groups -->
            <div class="p-3.5 rounded-xl border border-rose-500/30 bg-rose-950/10 space-y-2">
              <div class="text-xs font-bold text-rose-300 flex items-center gap-1.5">
                <span class="icon-[heroicons--lock-closed]"></span>
                Exclusive Item Locks (Cannot Equip Multiple)
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px]">
                {#each itemLockGroups as lg}
                  <div class="p-2 rounded-lg bg-gray-900/60 border border-gray-800">
                    <span class="font-bold text-gray-200">{lg.group}:</span>
                    <p class="text-gray-400 text-[10px] mt-0.5">{lg.items}</p>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {:else if activeTab === "warps"}
          <div class="space-y-3">
            <p class="text-gray-400 leading-relaxed">
              Equipping these augments alters champion stat-scaling and dynamically re-routes recommended item builds. Click <span class="text-purple-300 font-semibold">Simulate</span> to test how each augment reshapes the build in real time!
            </p>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {#each buildWarpAugments as warp}
                <div class="p-3 rounded-xl border {warp.color} bg-gray-950/40 flex flex-col justify-between">
                  <div>
                    <div class="flex items-center justify-between mb-1">
                      <span class="font-bold text-white text-xs">{warp.name}</span>
                      <span class="text-[10px] font-semibold uppercase">{warp.tier}</span>
                    </div>
                    <p class="text-[11px] text-gray-300 leading-normal mb-2">{warp.description}</p>
                    <div class="p-2 rounded-lg bg-gray-900/80 border border-gray-800/80 text-[11px] text-purple-200">
                      <span class="font-bold text-purple-300">Build Impact:</span> {warp.warpSummary}
                    </div>
                  </div>

                  {#if onSimulateAugment}
                    <div class="mt-3 pt-2 border-t border-gray-800/60 flex justify-end">
                      <button
                        onclick={() => { onSimulateAugment(warp.name); onClose(); }}
                        class="px-2.5 py-1 rounded-lg text-xs font-semibold bg-purple-600 hover:bg-purple-500 text-white transition flex items-center gap-1 shadow"
                      >
                        <span class="icon-[heroicons--arrow-path]"></span>
                        Simulate in Build
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end px-6 py-3 border-t border-gray-800 bg-gray-950/60">
        <button
          onclick={onClose}
          class="px-4 py-1.5 rounded-lg text-xs font-semibold bg-gray-800 hover:bg-gray-700 text-gray-200 transition"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
