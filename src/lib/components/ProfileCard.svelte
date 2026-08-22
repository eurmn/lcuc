<script lang="ts">
  import { onMount } from "svelte";
  import { getChallengeTierProfile, getSummonerProfile } from "../api/profile";
  import type { ChallengeTierProfile, SummonerProfile } from "../types/profile";

  let profile = $state<SummonerProfile | null>(null);
  let challengeTier = $state<ChallengeTierProfile | null>(null);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);

  export async function refreshProfile() {
    try {
      loading = true;
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

      if (c.status === "fulfilled") {
        challengeTier = c.value;
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    refreshProfile();
  });
</script>

{#if loading}
  <div class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-3.5">
    <div class="flex animate-pulse items-center gap-3">
      <div class="h-12 w-12 shrink-0 rounded-full border border-gray-700 bg-gray-800"></div>
      <div class="min-w-0 flex-1 space-y-1.5">
        <div class="h-2.5 w-16 rounded-full bg-gray-800"></div>
        <div class="h-3.5 w-28 rounded-full bg-gray-700"></div>
        <div class="h-2.5 w-20 rounded-full bg-gray-800"></div>
      </div>
    </div>
  </div>
{:else if profile}
  <div class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-3.5">
    <div class="flex items-center gap-3.5">
      <div class="relative h-12 w-12 shrink-0 overflow-hidden rounded-full border border-gray-700 bg-gray-800">
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
      <div class="min-w-0 flex-1">
        <p class="text-[11px] font-semibold uppercase tracking-wider text-blue-400">Summoner</p>
        <p class="truncate text-sm font-bold text-white">
          {profile.riot_id || profile.display_name}
        </p>
        <div class="mt-0.5 flex items-center gap-2">
          {#if challengeTier?.challenge_tier_asset_url}
            <div
              class="flex items-center"
              title={challengeTier.challenge_tier
                ? `Challenge tier: ${challengeTier.challenge_tier}`
                : "Challenge tier"}
            >
              <img
                src={challengeTier.challenge_tier_asset_url}
                alt="Challenge tier"
                class="h-4 w-4 shrink-0 object-contain"
              />
            </div>
          {/if}
          {#if profile.summoner_level}
            <p class="text-xs text-gray-400">Level {profile.summoner_level}</p>
          {/if}
        </div>
      </div>
    </div>
  </div>
{:else}
  <div class="w-full rounded-2xl border border-gray-800/80 bg-gray-900/60 p-3.5 text-xs text-gray-400">
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-amber-500/60"></span>
      <p class="font-medium text-gray-300">{errorMessage || "Client Offline"}</p>
    </div>
    <p class="mt-1 text-[11px] text-gray-500">Waiting for League Client to connect...</p>
  </div>
{/if}
