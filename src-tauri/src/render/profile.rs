use crate::models::SummonerProfile;

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub fn render_profile_card(profile: Option<&SummonerProfile>, error: Option<&str>) -> String {
    if let Some(err) = error {
        let template = r##"<div id="profile-card-container" class="w-full rounded-2xl border border-red-500/30 bg-gray-900/80 p-3.5 text-xs text-red-300">
    <p>Profile unavailable: {{error}}</p>
</div>"##;
        return template.replace("{{error}}", &escape_html(err));
    }

    let prof = match profile {
        Some(p) => p,
        None => {
            return r##"<div id="profile-card-container" class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-3.5 text-xs text-gray-500">
    No profile data
</div>"##.to_string();
        }
    };

    let name = escape_html(&prof.riot_id);
    let level_text = prof
        .summoner_level
        .map(|lvl| format!("Level {}", lvl))
        .unwrap_or_default();

    let icon_html = if let Some(ref icon_url) = prof.profile_icon_asset_url {
        let img_tpl = r##"<img src="{{icon_url}}" alt="{{name}}" class="h-full w-full object-cover">"##;
        img_tpl
            .replace("{{icon_url}}", &escape_html(icon_url))
            .replace("{{name}}", &name)
    } else {
        r##"<div class="h-full w-full bg-gray-800"></div>"##.to_string()
    };

    let border_html = if let Some(ref border_url) = prof.profile_border_asset_url {
        let border_tpl = r##"<img src="{{border_url}}" alt="border" class="pointer-events-none absolute inset-0 h-full w-full object-contain scale-110">"##;
        border_tpl.replace("{{border_url}}", &escape_html(border_url))
    } else {
        "".to_string()
    };

    let card_template = r##"<div id="profile-card-container" class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-3.5">
    <div class="flex items-center gap-3.5">
        <div class="relative h-12 w-12 shrink-0 overflow-hidden rounded-full border border-gray-700 bg-gray-800">
            {{icon_html}}
            {{border_html}}
        </div>
        <div class="min-w-0 flex-1">
            <p class="text-[11px] font-semibold uppercase tracking-wider text-blue-400">Summoner</p>
            <p class="truncate text-sm font-bold text-white">{{name}}</p>
            <div class="mt-0.5 flex items-center gap-2">
                <div
                    id="profile-challenge-tier-container"
                    class="flex items-center"
                    hx-get="command:ui_profile_challenge_tier"
                    hx-trigger="load"
                    hx-swap="outerHTML"
                >
                    <div class="h-4 w-4 shrink-0 rounded-full bg-gray-800 animate-pulse"></div>
                </div>
                <p class="text-xs text-gray-400">{{level_text}}</p>
            </div>
        </div>
    </div>
</div>"##;

    card_template
        .replace("{{icon_html}}", &icon_html)
        .replace("{{border_html}}", &border_html)
        .replace("{{name}}", &name)
        .replace("{{level_text}}", &escape_html(&level_text))
}

pub fn render_profile_challenge_tier_icon(asset_url: &str, challenge_tier: Option<&str>) -> String {
    let title = challenge_tier
        .map(|t| format!("Challenge tier: {}", t))
        .unwrap_or_else(|| "Challenge tier".to_string());

    let template = r##"<div id="profile-challenge-tier-container" class="flex items-center" title="{{title}}">
    <img src="{{asset_url}}" alt="{{title}}" class="h-4 w-4 shrink-0 object-contain">
</div>"##;

    template
        .replace("{{title}}", &escape_html(&title))
        .replace("{{asset_url}}", &escape_html(asset_url))
}
