/**
 * TypeScript definitions for Rune Builder & Telemetry Providers (Coachless.gg).
 */

export type LeagueTier = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7;

export const LEAGUE_TIERS_MAP: Record<LeagueTier, string> = {
  0: "Iron",
  1: "Bronze",
  2: "Silver",
  3: "Gold",
  4: "Platinum",
  5: "Emerald",
  6: "Diamond",
  7: "Master+",
};

export type LeagueRole = 0 | 1 | 2 | 3 | 4 | 5;

export const LEAGUE_ROLES_MAP: Record<LeagueRole, { label: string; icon: string }> = {
  0: { label: "Top", icon: "top" },
  1: { label: "Jungle", icon: "jungle" },
  2: { label: "Mid", icon: "mid" },
  3: { label: "ADC", icon: "bot" },
  4: { label: "Support", icon: "support" },
  5: { label: "All Roles", icon: "all" },
};

export interface PatchFilter {
  major: number;
  patch: number;
  patch_additions: number;
}

export interface BuilderQuery {
  champion_id: number;
  role?: number;
  league_tiers?: number[];
  patch?: PatchFilter;
  keystone_id?: number | null;
  primary_style_id?: number | null;
  sub_style_id?: number | null;
  provider?: string | null;
}

export interface RuneEffectMetric {
  effect_type: number;
  value: string;
}

export interface EnrichedKeystoneOption {
  id: number;
  name: string;
  icon_url: string;
  style_id: number;
  style_name: string;
  style_icon_url: string;
  wpa: number;
  occurrence: number;
  pick_rate_pct: number;
  effects: RuneEffectMetric[];
  selected: boolean;
  long_desc?: string;
}

export interface EnrichedRuneOption {
  id: number;
  name: string;
  icon_url: string;
  wpa: number;
  occurrence: number;
  pick_rate_pct: number;
  effects: RuneEffectMetric[];
  selected: boolean;
  long_desc?: string;
}

export interface TreeRowStats {
  row_index: number;
  label: string;
  options: EnrichedRuneOption[];
}

export interface TreePerksStats {
  style_id: number;
  style_name: string;
  icon_url: string;
  rows: TreeRowStats[];
}

export interface SecondaryTreeOption {
  style_id: number;
  style_name: string;
  icon_url: string;
  playcount_pct: number;
  is_recommended: boolean;
}

export interface StatShardOption {
  id: number;
  name: string;
  icon_url: string;
  shard_type: "offense" | "flex" | "defense";
  wpa: number;
  occurrence: number;
  pick_rate_pct: number;
  selected: boolean;
  long_desc?: string;
}

export interface StatShardsStats {
  offense: StatShardOption[];
  flex: StatShardOption[];
  defense: StatShardOption[];
}

export interface RuneBuilderOverview {
  champion_id: number;
  role: number;
  league_tiers: number[];
  provider: string;
  keystones: EnrichedKeystoneOption[];
  selected_keystone_id: number | null;
  selected_primary_style_id: number | null;
  selected_sub_style_id: number | null;
  primary_tree: TreePerksStats | null;
  secondary_tree_candidates: SecondaryTreeOption[];
  secondary_tree: TreePerksStats | null;
  stat_shards: StatShardsStats;
  recommended_perk_ids: number[];
}
