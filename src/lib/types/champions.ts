/**
 * Champion metadata TypeScript definitions.
 */

export const ALL_CHAMPIONS_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-fill-blue-hover.png";

export interface ChampionSummary {
  id: number;
  name: string;
  alias: string;
  square_icon_url: string;
  roles: string[];
  has_saved_runes: boolean;
}

export const ALL_CHAMPIONS_OPTION: ChampionSummary = {
  id: 0,
  name: "All Champions",
  alias: "All",
  square_icon_url: ALL_CHAMPIONS_ICON_URL,
  roles: ["all"],
  has_saved_runes: false,
};
