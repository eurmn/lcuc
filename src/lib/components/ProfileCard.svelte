<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { MotionDiv, type MotionTransition } from "@humanspeak/svelte-motion";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getChallengeTierProfile, getSummonerProfile } from "../api/profile";
  import type { ChallengeTierProfile, SummonerProfile } from "../types/profile";

  let {
    isOpen = true,
    onToggleCollapse,
  }: {
    isOpen?: boolean;
    onToggleCollapse?: () => void;
  } = $props();

  let profile = $state<SummonerProfile | null>(null);
  let challengeTier = $state<ChallengeTierProfile | null>(null);
  let loading = $state(false);
  let errorMessage = $state<string | null>(null);

  let unlistenSummoner: UnlistenFn | null = null;
  let unlistenConnected: UnlistenFn | null = null;
  let unlistenDisconnected: UnlistenFn | null = null;

  let layoutTransition: MotionTransition = $derived({
    x: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    width: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    y: { duration: 0.35, ease: "linear" },
    height: { duration: 0.35, ease: "linear" },
    opacity: { duration: 0.35, ease: "linear" },
  });

  export async function refreshProfile(showLoading = false) {
    try {
      if (showLoading) {
        loading = true;
      }
      errorMessage = null;
      const [p, c] = await Promise.allSettled([
        getSummonerProfile(),
        getChallengeTierProfile(),
      ]);

      if (p.status === "fulfilled") {
        profile = p.value;
      } else {
        errorMessage = typeof p.reason === "string" ? p.reason : "Client Offline";
        profile = null;
      }

      if (c.status === "fulfilled" && p.status === "fulfilled") {
        challengeTier = c.value;
      } else {
        challengeTier = null;
      }
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await refreshProfile(false);

    try {
      unlistenSummoner = await listen("lcu:summoner-changed", () => {
        refreshProfile(false);
      });
      unlistenConnected = await listen("lcu:connected", () => {
        refreshProfile(false);
      });
      unlistenDisconnected = await listen("lcu:disconnected", () => {
        profile = null;
        challengeTier = null;
        errorMessage = "Client Offline";
        loading = false;
      });
    } catch (err) {
      console.error("[ProfileCard] Failed to attach Tauri event listeners:", err);
    }
  });

  onDestroy(() => {
    if (unlistenSummoner) unlistenSummoner();
    if (unlistenConnected) unlistenConnected();
    if (unlistenDisconnected) unlistenDisconnected();
  });
</script>

{#if loading}
  <MotionDiv layout="position" transition={layoutTransition} class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-2.5 flex items-center justify-between gap-2.5 shadow-md overflow-hidden min-w-0">
    <div class="flex items-center gap-3 min-w-0 flex-1 overflow-hidden animate-pulse">
      <div class="h-10 w-10 shrink-0 rounded-full border border-gray-700 bg-gray-800"></div>
      <div class="min-w-0 flex-1 space-y-1.5 overflow-hidden whitespace-nowrap">
        <div class="h-2.5 w-16 rounded-full bg-gray-800"></div>
        <div class="h-3.5 w-28 rounded-full bg-gray-700"></div>
      </div>
    </div>
    {#if onToggleCollapse}
      <button
        type="button"
        onclick={onToggleCollapse}
        title={isOpen ? "Collapse sidebar" : "Expand sidebar"}
        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-gray-800 bg-gray-950/80 text-gray-400 hover:border-gray-700 hover:bg-gray-800 hover:text-white transition-all cursor-pointer shadow-sm"
      >
        <span class="{isOpen ? 'icon-[heroicons--chevron-double-left]' : 'icon-[heroicons--chevron-double-right]'} h-3.5 w-3.5"></span>
      </button>
    {/if}
  </MotionDiv>
{:else if profile}
  <MotionDiv layout="position" transition={layoutTransition} class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-2.5 flex items-center justify-between gap-2.5 shadow-md overflow-hidden min-w-0">
    <div class="flex items-center gap-3 min-w-0 flex-1 overflow-hidden">
      <!-- Avatar (Always visible on left) -->
      <div class="relative h-10 w-10 shrink-0 overflow-hidden rounded-full border border-gray-700 bg-gray-800 shadow-inner">
        {#if profile.profile_icon_asset_url}
          <img
            src={profile.profile_icon_asset_url}
            alt={profile.riot_id || profile.display_name}
            class="h-full w-full object-cover"
          />
        {:else}
          <div class="h-full w-full bg-gray-800"></div>
        {/if}
      </div>

      <!-- Text Container (Eaten by overflow as card width shrinks) -->
      <div class="min-w-0 flex-1 overflow-hidden whitespace-nowrap">
        <p class="text-[10px] font-semibold uppercase tracking-wider text-blue-400 leading-tight">Summoner</p>
        <p class="truncate text-sm font-bold text-white leading-tight">
          {profile.riot_id || profile.display_name}
        </p>
        <div class="mt-0.5 flex items-center gap-1.5 overflow-hidden whitespace-nowrap">
          {#if challengeTier?.challenge_tier_asset_url}
            <img
              src={challengeTier.challenge_tier_asset_url}
              alt="Challenge tier"
              class="h-3.5 w-3.5 shrink-0 object-contain"
            />
          {/if}
          {#if profile.summoner_level}
            <p class="text-[11px] text-gray-400 leading-tight">Lvl {profile.summoner_level}</p>
          {/if}
        </div>
      </div>
    </div>

    <!-- Toggle Button (Pinned on right) -->
    {#if onToggleCollapse}
      <button
        type="button"
        onclick={onToggleCollapse}
        title={isOpen ? "Collapse sidebar" : "Expand sidebar"}
        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-gray-800 bg-gray-950/80 text-gray-400 hover:border-gray-700 hover:bg-gray-800 hover:text-white transition-all cursor-pointer shadow-sm"
      >
        <span class="{isOpen ? 'icon-[heroicons--chevron-double-left]' : 'icon-[heroicons--chevron-double-right]'} h-3.5 w-3.5"></span>
      </button>
    {/if}
  </MotionDiv>
{:else}
  <MotionDiv layout="position" transition={layoutTransition} class="w-full rounded-2xl border border-gray-800/80 bg-gray-900/60 p-2.5 flex items-center justify-between gap-2.5 text-xs text-gray-400 shadow-md overflow-hidden min-w-0">
    <div class="min-w-0 flex-1 overflow-hidden whitespace-nowrap">
      <div class="flex items-center gap-2">
        <span class="h-2 w-2 shrink-0 rounded-full bg-amber-500/60"></span>
        <p class="font-medium text-gray-300 truncate">{errorMessage || "Client Offline"}</p>
      </div>
      <p class="mt-1 text-[11px] text-gray-500 truncate">Waiting for League Client to connect...</p>
    </div>

    {#if onToggleCollapse}
      <button
        type="button"
        onclick={onToggleCollapse}
        title={isOpen ? "Collapse sidebar" : "Expand sidebar"}
        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-gray-800 bg-gray-950/80 text-gray-400 hover:border-gray-700 hover:bg-gray-800 hover:text-white transition-all cursor-pointer shadow-sm"
      >
        <span class="{isOpen ? 'icon-[heroicons--chevron-double-left]' : 'icon-[heroicons--chevron-double-right]'} h-3.5 w-3.5"></span>
      </button>
    {/if}
  </MotionDiv>
{/if}
