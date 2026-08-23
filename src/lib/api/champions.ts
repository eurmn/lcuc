/**
 * Champions Tauri IPC client wrappers.
 */

import { tauriInvoke } from "./client";
import type { ChampionSummary } from "../types/champions";

/**
 * Fetches all champions sorted alphabetically.
 */
export async function getChampions(): Promise<ChampionSummary[]> {
  return tauriInvoke<ChampionSummary[]>("api_get_champions");
}

/**
 * Fetches the roster of champions that currently have saved runes.
 */
export async function getChampionRoster(): Promise<ChampionSummary[]> {
  return tauriInvoke<ChampionSummary[]>("api_get_champion_roster");
}

/**
 * Attempts to detect a champion name within a given string (e.g. rune page name).
 */
export async function detectChampionFromName(name: string): Promise<ChampionSummary | null> {
  return tauriInvoke<ChampionSummary | null>("api_detect_champion_from_name", { name });
}

/**
 * Toggles a champion's favorite status in SQLite.
 */
export async function toggleFavoriteChampion(championId: number): Promise<boolean> {
  return tauriInvoke<boolean>("api_toggle_favorite_champion", { championId });
}

/**
 * Retrieves the last selected champion ID persisted in SQLite.
 */
export async function getSelectedChampionId(): Promise<number | null> {
  return tauriInvoke<number | null>("api_get_selected_champion_id");
}

/**
 * Persists the last selected champion ID in SQLite.
 */
export async function setSelectedChampionId(championId: number | null): Promise<void> {
  return tauriInvoke<void>("api_set_selected_champion_id", { championId });
}

