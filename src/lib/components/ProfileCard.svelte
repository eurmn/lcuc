<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getChallengeTierProfile, getSummonerProfile } from "../api/profile";
  import type { ChallengeTierProfile, SummonerProfile } from "../types/profile";

  let profile = $state<SummonerProfile | null>(null);
  let challengeTier = $state<ChallengeTierProfile | null>(null);

  let unlistenSummoner: UnlistenFn | null = null;
  let unlistenConnected: UnlistenFn | null = null;
  let unlistenDisconnected: UnlistenFn | null = null;

  export async function refreshProfile() {
    try {
      const [p, c] = await Promise.allSettled([
        getSummonerProfile(),
        getChallengeTierProfile(),
      ]);

      if (p.status === "fulfilled") {
        profile = p.value;
      } else {
        profile = null;
      }

      if (c.status === "fulfilled" && p.status === "fulfilled") {
        challengeTier = c.value;
      } else {
        challengeTier = null;
      }
    } catch {
      profile = null;
      challengeTier = null;
    }
  }

  onMount(async () => {
    await refreshProfile();

    try {
      unlistenSummoner = await listen("lcu:summoner-changed", () => {
        refreshProfile();
      });
      unlistenConnected = await listen("lcu:connected", () => {
        refreshProfile();
      });
      unlistenDisconnected = await listen("lcu:disconnected", () => {
        profile = null;
        challengeTier = null;
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

<div
  class="flex h-[3.25rem] shrink-0 items-center gap-2.5 rounded-2xl border border-gray-800 bg-gray-900 px-3 py-1.5 shadow-md select-none w-64 max-w-[309px]"
>
  <!-- Avatar (Blank grey circle placeholder when not connected) -->
  <div
    class="relative h-9 w-9 shrink-0 overflow-hidden rounded-full border border-gray-700 bg-gray-800 shadow-inner"
  >
    {#if profile?.profile_icon_asset_url}
      <img
        src={profile.profile_icon_asset_url}
        alt={profile.riot_id || profile.display_name}
        class="h-full w-full object-cover"
      />
    {:else}
      <!-- Blank grey circle placeholder -->
      <div class="h-full w-full bg-gray-800"></div>
    {/if}
  </div>

  <!-- Text & Info Container -->
  <div class="min-w-0 flex-1 overflow-hidden whitespace-nowrap">
    <!-- SUMMONER Label (Always Kept) -->
    <p class="text-[10px] font-semibold uppercase tracking-wider text-blue-400 leading-tight">
      Summoner
    </p>

    {#if profile}
      <!-- Username -->
      <p class="truncate text-xs font-bold text-white leading-tight">
        {profile.riot_id || profile.display_name}
      </p>
      <!-- Sub-info: Challenge Tier badge + Level -->
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
    {:else}
      <!-- Disconnected State Username -->
      <p class="truncate text-xs font-bold text-gray-400 leading-tight">
        Disconnected
      </p>
      <!-- Placeholder items of matching dimensions -->
      <div class="mt-0.5 flex items-center gap-1.5 overflow-hidden whitespace-nowrap">
        <div class="h-3.5 w-3.5 shrink-0 rounded bg-gray-800/80"></div>
        <p class="text-[11px] text-gray-500 leading-tight">Lvl ---</p>
      </div>
    {/if}
  </div>
</div>
