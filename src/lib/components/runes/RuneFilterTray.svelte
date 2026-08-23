<script lang="ts">
  import {
    AnimatePresence,
    MotionDiv,
    type MotionTransition,
  } from "@humanspeak/svelte-motion";
  import Divider from "../Divider.svelte";
  import { LEAGUE_ROLES_MAP } from "../../types/rune_builder";

  let {
    isEditing = false,
    selectedRole = 1,
    selectedTiers = [4, 5, 6, 7],
    onSelectRole,
    onToggleTier,
  }: {
    isEditing: boolean;
    selectedRole: number;
    selectedTiers: number[];
    onSelectRole: (role: number) => void;
    onToggleTier: (tier: number) => void;
  } = $props();

  const roleEntries = Object.entries(LEAGUE_ROLES_MAP).map(([key, val]) => ({
    id: parseInt(key, 10),
    label: val.label,
  }));

  const tierEntries: { id: number; label: string }[] = [
    { id: 3, label: "Gold" },
    { id: 4, label: "Platinum" },
    { id: 5, label: "Emerald" },
    { id: 6, label: "Diamond" },
    { id: 7, label: "Master+" },
  ];

  const filterTransition: MotionTransition = {
    height: { duration: 0.5, ease: [0.16, 1, 0.3, 1] },
    opacity: { duration: 0.3, ease: "easeInOut" },
  };
</script>

<AnimatePresence present={isEditing}>
  {#snippet child()}
    <MotionDiv
      key="rune-filters-tray"
      initial={{ opacity: 0, height: 0 }}
      animate={{ opacity: 1, height: 55 }}
      exit={{ opacity: 0, height: 0 }}
      transition={filterTransition}
      class="overflow-hidden"
    >
      <!-- Separate Animated Divider Component -->
      <Divider transition={filterTransition} />

      <!-- Filter Controls Row -->
      <div class="flex flex-wrap items-center justify-between gap-3">
        <!-- Role Tabs -->
        <div class="flex items-center gap-1">
          <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1">Role:</span>
          <div class="inline-flex rounded-xl bg-gray-900 p-0.5 border border-gray-800 shadow-sm">
            {#each roleEntries as role (role.id)}
              <button
                type="button"
                onclick={() => onSelectRole(role.id)}
                class={selectedRole === role.id
                  ? "rounded-lg bg-purple-600 px-2.5 py-1 text-xs font-bold text-white shadow-sm cursor-pointer"
                  : "rounded-lg px-2.5 py-1 text-xs font-semibold text-gray-400 hover:text-white cursor-pointer transition-colors"}
              >
                {role.label}
              </button>
            {/each}
          </div>
        </div>

        <!-- Rank Tier Filter Badges -->
        <div class="flex items-center gap-1">
          <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1">Ranks:</span>
          <div class="flex flex-wrap items-center gap-1">
            {#each tierEntries as tier (tier.id)}
              {@const isChecked = selectedTiers.includes(tier.id)}
              <button
                type="button"
                onclick={() => onToggleTier(tier.id)}
                class={isChecked
                  ? "inline-flex items-center gap-1 rounded-full border border-purple-500/50 bg-purple-500/20 px-2 py-0.5 text-[10px] font-bold text-purple-300 shadow-sm cursor-pointer"
                  : "inline-flex items-center gap-1 rounded-full border border-gray-800 bg-gray-900/60 px-2 py-0.5 text-[10px] font-medium text-gray-400 hover:border-gray-700 hover:text-gray-300 cursor-pointer"}
              >
                {#if isChecked}
                  <span class="icon-[heroicons--check] h-2.5 w-2.5 text-purple-400"></span>
                {/if}
                <span>{tier.label}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>
    </MotionDiv>
  {/snippet}
</AnimatePresence>
