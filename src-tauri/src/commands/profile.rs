//! # Profile & Summoner Tauri Commands
//!
//! Exposes summoner identity, level, avatar, and challenge tier endpoints to HTMX.

use crate::models::SummonerProfile;
use crate::render::{render_profile_card, render_profile_challenge_tier_icon};
use crate::services::summoner::{
    get_current_challenge_tier_profile, get_current_summoner_profile,
};

/// HTMX command returning the summoner profile header card partial.
///
/// Triggered on page load via `hx-get="command:ui_profile_card"`.
#[tauri::command]
pub async fn ui_profile_card() -> String {
    match get_current_summoner_profile().await {
        Ok(profile) => render_profile_card(Some(&profile), None),
        Err(err) => render_profile_card(None, Some(&err)),
    }
}

/// HTMX command returning the challenge tier icon partial.
///
/// Triggered on page load via `hx-get="command:ui_profile_challenge_tier"`.
#[tauri::command]
pub async fn ui_profile_challenge_tier() -> String {
    let tier_prof = get_current_challenge_tier_profile().await;
    render_profile_challenge_tier_icon(
        &tier_prof.challenge_tier_asset_url,
        tier_prof.challenge_tier.as_deref(),
    )
}

/// Direct JSON API command returning the full typed [`SummonerProfile`].
#[tauri::command]
pub async fn api_summoner_profile() -> Result<SummonerProfile, String> {
    get_current_summoner_profile().await
}
