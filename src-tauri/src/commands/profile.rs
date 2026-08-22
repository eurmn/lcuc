//! # Profile & Summoner Tauri Commands
//!
//! Exposes summoner identity, level, avatar, and challenge tier endpoints to Svelte.

use crate::models::{ChallengeTierProfile, SummonerProfile};
use crate::services::summoner::{
    get_current_challenge_tier_profile, get_current_summoner_profile,
};

/// Direct JSON API command returning the full typed [`SummonerProfile`].
#[tauri::command]
pub async fn api_summoner_profile() -> Result<SummonerProfile, String> {
    get_current_summoner_profile().await
}

/// Direct JSON API command returning the full typed [`ChallengeTierProfile`].
#[tauri::command]
pub async fn api_profile_challenge_tier() -> Result<ChallengeTierProfile, String> {
    Ok(get_current_challenge_tier_profile().await)
}

