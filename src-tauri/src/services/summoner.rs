//! # Summoner Profile & Regalia Service
//!
//! Handles retrieval of the active player's summoner identity, Riot ID,
//! ranked crests/wings, prestige border assets, and challenge tier metadata from LCU.
//!
//! All remote CommunityDragon assets are cached in-disk and served as client-ready local assets.

use crate::models::{ChallengeTierProfile, SummonerProfile};
use crate::services::asset_cache::get_cached_asset_data_uri;
use crate::services::lcu_client::LcuClient;
use serde_json::Value;

const CDRAGON_BASE: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";
const CHALLENGE_TOKEN_BASE: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/challenges-shared";

/// Queries the local League client for the current summoner profile, icon, and regalia borders.
///
/// Assets are fetched server-side from CommunityDragon and cached in-disk.
pub async fn get_current_summoner_profile() -> Result<SummonerProfile, String> {
    let client = LcuClient::connect()?;

    let summoner: Value = client.get_json("/lol-summoner/v1/current-summoner").await?;
    let regalia: Option<Value> = client
        .get_json("/lol-regalia/v2/current-summoner/regalia")
        .await
        .ok();

    let display_name = summoner
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let game_name = summoner
        .get("gameName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tag_line = summoner
        .get("tagLine")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let riot_id = if !game_name.is_empty() && !tag_line.is_empty() {
        format!("{}#{}", game_name, tag_line)
    } else if !display_name.is_empty() {
        display_name.clone()
    } else {
        "Summoner".to_string()
    };

    let summoner_level = summoner.get("summonerLevel").and_then(|v| v.as_i64());
    let profile_icon_id = summoner.get("profileIconId").and_then(|v| v.as_i64());

    let profile_icon_asset_url = if let Some(id) = profile_icon_id {
        let remote_url = format!("{}/v1/profile-icons/{}.jpg", CDRAGON_BASE, id);
        Some(get_cached_asset_data_uri(&remote_url).await)
    } else {
        None
    };

    let mut profile_border_asset_url: Option<String> = None;
    let mut profile_border_kind: Option<String> = None;

    if let Some(reg) = regalia {
        let crest_type = reg
            .get("crestType")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let selected_crest = reg.get("selectedPrestigeCrest").and_then(|v| v.as_i64());

        if crest_type == "ranked" {
            if let Some(tier) = reg
                .get("highestRankedEntry")
                .and_then(|e| e.get("tier"))
                .and_then(|t| t.as_str())
            {
                let tier_lower = tier.to_lowercase();
                let remote_url = format!(
                    "{}/v1/profile-borders/ranked-wings/{}.png",
                    CDRAGON_BASE, tier_lower
                );
                profile_border_asset_url = Some(get_cached_asset_data_uri(&remote_url).await);
                profile_border_kind = Some("ranked".to_string());
            }
        } else if let Some(crest_id) = selected_crest {
            if crest_id > 0 {
                let remote_url = format!(
                    "{}/v1/profile-borders/theme-borders/{}.png",
                    CDRAGON_BASE, crest_id
                );
                profile_border_asset_url = Some(get_cached_asset_data_uri(&remote_url).await);
                profile_border_kind = Some("theme".to_string());
            }
        }
    }

    Ok(SummonerProfile {
        riot_id,
        display_name,
        summoner_level,
        profile_icon_id,
        profile_icon_asset_url,
        profile_border_asset_url,
        profile_border_kind,
    })
}

/// Queries the local League client for the player's overall challenge level and resolves the challenge token icon.
///
/// Uses the requested CDragon path:
/// `https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/challenges-shared/icon-token-(rank).png`
pub async fn get_current_challenge_tier_profile() -> ChallengeTierProfile {
    let challenge_tier = match LcuClient::connect() {
        Ok(client) => match client
            .get_json::<Value>("/lol-challenges/v1/summary-player-data/local-player")
            .await
        {
            Ok(summary) => summary
                .get("overallChallengeLevel")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    };

    let rank_name = challenge_tier
        .as_deref()
        .map(|s| s.to_lowercase())
        .filter(|s| !s.is_empty() && s != "none")
        .unwrap_or_else(|| "unranked".to_string());

    let remote_url = format!("{}/icon-token-{}.png", CHALLENGE_TOKEN_BASE, rank_name);
    let challenge_tier_asset_url = get_cached_asset_data_uri(&remote_url).await;

    ChallengeTierProfile {
        challenge_tier,
        challenge_tier_asset_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_summoner_profile() {
        let profile = get_current_summoner_profile().await;
        println!("Live summoner profile: {:?}", profile);
        assert!(profile.is_ok(), "Expected valid summoner profile from running League client");
    }
}
