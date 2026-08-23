import { tauriInvoke } from "./client";
import type {
  RuneSetSummary,
  RuneTreePayload,
  RunesOverviewPayload,
  SaveRuneTreePayload,
  SelectPerkPayload,
} from "../types/runes";

export async function getRunesOverview(): Promise<RunesOverviewPayload> {
  return tauriInvoke<RunesOverviewPayload>("api_runes_overview");
}

export async function getSavedRunesets(): Promise<RuneSetSummary[]> {
  return tauriInvoke<RuneSetSummary[]>("api_runesets");
}

export async function importClientPage(pageId: number, championId?: number | null): Promise<number> {
  return tauriInvoke<number>("api_import_client_page", { pageId, championId: championId ?? null });
}

export async function exportSavedRunePage(runesetId: number): Promise<number> {
  return tauriInvoke<number>("api_export_saved_rune_page", { runesetId });
}

export async function deleteRuneset(runesetId: number): Promise<boolean> {
  return tauriInvoke<boolean>("api_delete_runeset", { runesetId });
}

export async function updateRunesetChampion(runesetId: number, championId: number): Promise<boolean> {
  return tauriInvoke<boolean>("api_update_runeset_champion", { runesetId, championId });
}

export async function getCurrentRuneTree(): Promise<RuneTreePayload> {
  return tauriInvoke<RuneTreePayload>("api_rune_tree_current");
}

export async function getClientRuneTree(pageId: number): Promise<RuneTreePayload> {
  return tauriInvoke<RuneTreePayload>("api_rune_tree_client", { pageId });
}

export async function getSavedRuneTree(runesetId: number): Promise<RuneTreePayload> {
  return tauriInvoke<RuneTreePayload>("api_rune_tree_saved", { runesetId });
}

export async function selectRunePerk(payload: SelectPerkPayload): Promise<RuneTreePayload> {
  return tauriInvoke<RuneTreePayload>("api_rune_tree_select_perk", { payload });
}

export async function saveRuneTree(payload: SaveRuneTreePayload): Promise<number> {
  return tauriInvoke<number>("api_save_rune_tree", { payload });
}

export async function getSelectedRuneKey(): Promise<string | null> {
  return tauriInvoke<string | null>("api_get_selected_rune_key");
}

export async function setSelectedRuneKey(runeKey: string | null): Promise<void> {
  return tauriInvoke<void>("api_set_selected_rune_key", { runeKey });
}
