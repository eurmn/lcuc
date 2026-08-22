import { tauriInvoke } from "./client";
import type { ChallengeTierProfile, SummonerProfile } from "../types/profile";

export async function getSummonerProfile(): Promise<SummonerProfile> {
  return tauriInvoke<SummonerProfile>("api_summoner_profile");
}

export async function getChallengeTierProfile(): Promise<ChallengeTierProfile> {
  return tauriInvoke<ChallengeTierProfile>("api_profile_challenge_tier");
}
