/**
 * Game Description Rich Tag & Stat Parser
 *
 * Formats CommunityDragon and LCU rich markup for items, augments, and runes
 * into styled, semantic HTML with official League of Legends stat colors and icons.
 */

const STAT_ICONS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/statsicon";

const GOLD_ICON =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/gameplay/goldcoins.png";

const TRUE_DAMAGE_ICON =
  "https://raw.communitydragon.org/latest/game/assets/ux/floatingtext/enhancedtruedamageicon.png";

interface TagStyleDefinition {
  colorClass: string;
  iconUrl?: string;
  isBlock?: boolean;
}

const TAG_DEFINITIONS: Record<string, TagStyleDefinition> = {
  // Stat tags with icons
  scalead: {
    colorClass: "text-orange-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalead.png`,
  },
  scaleap: {
    colorClass: "text-cyan-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleap.png`,
  },
  scalehealth: {
    colorClass: "text-emerald-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalehealth.png`,
  },
  scalemana: {
    colorClass: "text-sky-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalemana.png`,
  },
  scalelevel: {
    colorClass: "text-amber-200/90 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalelevel.png`,
  },
  speed: {
    colorClass: "text-teal-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalems.png`,
  },
  scalems: {
    colorClass: "text-teal-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalems.png`,
  },
  gold: {
    colorClass: "text-yellow-400 font-bold",
    iconUrl: GOLD_ICON,
  },
  truedamage: {
    colorClass: "text-white font-bold",
    iconUrl: TRUE_DAMAGE_ICON,
  },
  scalearmor: {
    colorClass: "text-yellow-600 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalearmor.png`,
  },
  scalemr: {
    colorClass: "text-indigo-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalemr.png`,
  },
  scaleas: {
    colorClass: "text-amber-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleas.png`,
  },
  scaleah: {
    colorClass: "text-teal-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleah.png`,
  },
  scaleabilityhaste: {
    colorClass: "text-teal-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleah.png`,
  },
  scalebonus: {
    colorClass: "text-amber-300 font-semibold",
  },
  scaleapen: {
    colorClass: "text-rose-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleapen.png`,
  },
  scalempen: {
    colorClass: "text-purple-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalempen.png`,
  },
  scalecrit: {
    colorClass: "text-red-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalecrit.png`,
  },
  scaletenacity: {
    colorClass: "text-emerald-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaletenacity.png`,
  },
  scaleadaptiveforce: {
    colorClass: "text-purple-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scaleadaptiveforce.png`,
  },
  scalehealshield: {
    colorClass: "text-emerald-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalehealshield.png`,
  },
  scalehpregen: {
    colorClass: "text-emerald-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalehpregen.png`,
  },
  scalemanaregen: {
    colorClass: "text-sky-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalemanaregen.png`,
  },
  scalecooldown: {
    colorClass: "text-cyan-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalecooldown.png`,
  },
  scalerange: {
    colorClass: "text-lime-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalerange.png`,
  },
  scalesize: {
    colorClass: "text-orange-300 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/scalesize.png`,
  },
  onhit: {
    colorClass: "text-amber-400 font-semibold",
    iconUrl: `${STAT_ICONS_BASE}/onhit.png`,
  },

  // Damage & Effect types
  physicaldamage: {
    colorClass: "text-orange-400 font-semibold",
  },
  magicdamage: {
    colorClass: "text-cyan-400 font-semibold",
  },
  healing: {
    colorClass: "text-emerald-400 font-semibold",
  },
  shield: {
    colorClass: "text-teal-300 font-semibold",
  },

  // Highlight & Semantic tags without icons
  keywordmajor: {
    colorClass: "text-yellow-300 font-bold",
  },
  statgood: {
    colorClass: "text-emerald-400 font-semibold",
  },
  attention: {
    colorClass: "text-amber-300 font-semibold",
  },
  status: {
    colorClass: "text-purple-300 font-semibold",
  },
  rules: {
    colorClass: "text-gray-400 italic text-[11px]",
    isBlock: true,
  },
  passive: {
    colorClass: "text-amber-200 font-semibold",
  },
  active: {
    colorClass: "text-sky-300 font-semibold",
  },
  stats: {
    colorClass: "text-gray-200 font-medium",
    isBlock: true,
  },
  maintext: {
    colorClass: "text-gray-300",
    isBlock: false,
  },
};

/**
 * Parses raw League longDesc, item description, or augment tooltip strings into styled HTML.
 */
export function parseGameDescription(raw?: string | null): string {
  if (!raw || typeof raw !== "string") {
    return "";
  }

  let html = raw.trim();

  // 1. Standardize line breaks and dividers, collapsing duplicate consecutive <br> tags
  html = html.replace(/<hr\s*\/?>/gi, '<hr class="border-gray-800 my-2" />');
  html = html.replace(/<br\s*\/?>/gi, "<br />");
  html = html.replace(/(<br\s*\/?>\s*){2,}/gi, "<br />");

  // 2. Standard formatting tags (b, strong, i, em, u)
  html = html.replace(/<b\b[^>]*>([\s\S]*?)<\/b>/gi, '<strong class="font-bold text-white">$1</strong>');
  html = html.replace(/<i\b[^>]*>([\s\S]*?)<\/i>/gi, '<em class="italic text-gray-300">$1</em>');

  // 3. Custom <font color="#..."> tags
  html = html.replace(/<font\s+color=['"]?([^'">]+)['"]?>([\s\S]*?)<\/font>/gi, (_match, color, content) => {
    return `<span style="color: ${color}">${content}</span>`;
  });

  // 4. Clean up match runtime tracker lines and parenthetical trackers
  html = html.replace(/\s*\(@f[0-9]+(\*100)?@%?\)/gi, "");
  html = html.replace(/\s*\(@f[0-9]+@\)/gi, "");
  html = html.replace(/<br\s*\/?>\s*<br\s*\/?>\s*(Damage Dealt|Bonus Damage|Healing Done|Shields Generated|Immobilizing Spells Blocked|Hooks Landed|Times Triggered|Takedowns|Damage Redirected|Burn Damage Dealt)[^<]*(@f[0-9]+@|f[0-9]+)[\s\S]*$/gi, "");
  html = html.replace(/<br\s*\/?>\s*(Damage Dealt|Bonus Damage|Healing Done|Shields Generated|Immobilizing Spells Blocked|Hooks Landed|Times Triggered|Takedowns|Damage Redirected|Burn Damage Dealt)[^<]*(@f[0-9]+@|f[0-9]+)[\s\S]*$/gi, "");

  // Remove leftover @Param@ variables cleanly if any weren't substituted
  html = html.replace(/@([a-zA-Z0-9_]+)(\*100)?@/g, (_m, param) => {
    if (/^f[0-9]+$/i.test(param)) return "";
    return `<span class="text-amber-300 font-semibold">${param}</span>`;
  });

  // Clean unresolved {{...}} templates
  html = html.replace(/\{\{[^}]*\}\}/g, "");

  // 5. Transform all game stat tags in multiple passes to support nesting
  let prevHtml = "";
  let passCount = 0;
  while (prevHtml !== html && passCount < 5) {
    prevHtml = html;
    passCount++;

    for (const [tag, def] of Object.entries(TAG_DEFINITIONS)) {
      const regex = new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)<\\/${tag}>`, "gi");

      html = html.replace(regex, (_match, content) => {
        const iconHtml = def.iconUrl
          ? `<img src="${def.iconUrl}" alt="" class="inline-block h-[1em] w-[1em] align-[-0.15em] mr-0.5 object-contain select-none pointer-events-none" />`
          : "";

        if (def.isBlock) {
          return `<div class="${def.colorClass} my-1">${iconHtml}${content}</div>`;
        }

        return `<span class="${def.colorClass}">${iconHtml}${content}</span>`;
      });
    }
  }

  // 6. Clean up known container tags if open (<mainText>, <stats>)
  html = html.replace(/<\/?(mainText|stats)[^>]*>/gi, "");

  // 7. Clean up any lingering unknown XML tags safely while preserving text content
  html = html.replace(/<\/?([a-z0-9_]+)[^>]*>/gi, (fullTag, tagName) => {
    const knownTags = ["strong", "em", "span", "div", "img", "br", "hr", "p", "b", "i"];
    if (knownTags.includes(tagName.toLowerCase())) {
      return fullTag;
    }
    return "";
  });

  // 8. Final pass: collapse any leftover duplicate <br /> tags and trim leading/trailing breaks
  html = html.replace(/(<br\s*\/?>\s*){2,}/gi, "<br />");
  html = html.replace(/^(\s*<br\s*\/?>\s*)+|(\s*<br\s*\/?>\s*)+$/gi, "").trim();

  return html;
}
