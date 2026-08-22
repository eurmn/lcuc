export interface SummonerProfile {
  riot_id: string;
  display_name: string;
  summoner_level: number | null;
  profile_icon_id: number | null;
  profile_icon_asset_url: string | null;
  profile_border_asset_url: string | null;
  profile_border_kind: string | null;
}

export interface ChallengeTierProfile {
  challenge_tier: string | null;
  challenge_tier_asset_url: string;
}
