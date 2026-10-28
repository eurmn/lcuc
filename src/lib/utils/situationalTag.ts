/**
 * Situational Tag helper
 * Maps Mayhem champ-select situational item tags to CommunityDragon stat icons.
 */

const STAT_ICONS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/statsicon";

export interface SituationalTagInfo {
  tag: string;
  iconUrl?: string;
  borderClass?: string;
  title: string;
}

export function getSituationalTagInfo(tag: string | null | undefined): SituationalTagInfo | null {
  if (!tag) return null;
  const clean = tag.trim();
  if (!clean) return null;

  const lower = clean.toLowerCase();

  // Synergy tags remain text badges
  if (lower.startsWith("synergy:")) {
    return { tag: clean, title: clean };
  }

  // Anti-Heal (Grievous Wounds)
  if (lower.includes("anti-heal") || lower.includes("grievous")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scalehpregen.png`,
      borderClass: "border-rose-500/80 shadow-rose-950/50",
      title: clean,
    };
  }

  // Anti-Shield
  if (lower.includes("anti-shield") || lower.includes("shield")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scalehealshield.png`,
      borderClass: "border-cyan-400/80 shadow-cyan-950/50",
      title: clean,
    };
  }

  // Armor Penetration
  if (lower.includes("armor pen")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scaleapen.png`,
      borderClass: "border-amber-500/80 shadow-amber-950/50",
      title: clean,
    };
  }

  // Magic Penetration
  if (lower.includes("magic pen")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scalempen.png`,
      borderClass: "border-purple-400/80 shadow-purple-950/50",
      title: clean,
    };
  }

  // Magic Resist
  if (lower.includes("magic resist") || lower === "mr") {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scalemr.png`,
      borderClass: "border-indigo-400/80 shadow-indigo-950/50",
      title: clean,
    };
  }

  // Armor / Anti-Burst
  if (lower.includes("armor") || lower.includes("anti-burst")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scalearmor.png`,
      borderClass: "border-yellow-400/80 shadow-yellow-950/50",
      title: clean,
    };
  }

  // Cleanse / QSS / Tenacity
  if (lower.includes("cleanse") || lower.includes("qss") || lower.includes("tenacity")) {
    return {
      tag: clean,
      iconUrl: `${STAT_ICONS_BASE}/scaletenacity.png`,
      borderClass: "border-emerald-400/80 shadow-emerald-950/50",
      title: clean,
    };
  }

  // Fallback to text badge for unmapped tags
  return { tag: clean, title: clean };
}
