use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummonerProfile {
    pub riot_id: String,
    pub display_name: String,
    pub summoner_level: Option<i64>,
    pub profile_icon_id: Option<i64>,
    pub profile_icon_asset_url: Option<String>,
    pub profile_border_asset_url: Option<String>,
    pub profile_border_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChallengeTierProfile {
    pub challenge_tier: Option<String>,
    pub challenge_tier_asset_url: String,
}
