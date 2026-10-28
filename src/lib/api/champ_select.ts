import { invoke } from "@tauri-apps/api/core";
import type {
  ChampSelectSession,
  ExportBuildPayload,
  ExportBuildResult,
  LiveGameState,
  MayhemAugmentsQuery,
  MayhemAugmentsResponse,
  MayhemChampionBuildSummary,
  MayhemLiveAdvice,
  SaveMayhemBuildPayload,
  SavedMayhemBuild,
} from "../types/champ_select";

export async function isLcuConnected(): Promise<boolean> {
  return invoke<boolean>("api_is_lcu_connected");
}

export async function getChampSelectSession(): Promise<ChampSelectSession | null> {
  return invoke<ChampSelectSession | null>("api_champ_select_session");
}

export async function getMayhemChampionBuild(
  championId: number,
  augmentIds: string[] = []
): Promise<MayhemChampionBuildSummary> {
  return invoke<MayhemChampionBuildSummary>("api_mayhem_champion_build", {
    championId,
    augmentIds,
  });
}

export async function saveMayhemBuild(
  payload: SaveMayhemBuildPayload
): Promise<SavedMayhemBuild> {
  return invoke<SavedMayhemBuild>("api_save_mayhem_build", { payload });
}

export async function listSavedMayhemBuilds(
  championId?: number
): Promise<SavedMayhemBuild[]> {
  return invoke<SavedMayhemBuild[]>("api_list_saved_mayhem_builds", {
    championId: championId ?? null,
  });
}

export async function deleteSavedMayhemBuild(id: string): Promise<void> {
  return invoke<void>("api_delete_saved_mayhem_build", { id });
}

export async function getMayhemTeamBuilds(
  championIds: number[]
): Promise<Record<number, MayhemChampionBuildSummary>> {
  return invoke<Record<number, MayhemChampionBuildSummary>>("api_mayhem_team_builds", {
    championIds,
  });
}

export async function getMayhemAugments(
  query: MayhemAugmentsQuery
): Promise<MayhemAugmentsResponse> {
  return invoke<MayhemAugmentsResponse>("api_mayhem_augments", {
    query,
  });
}

export async function exportMayhemBuild(
  payload: ExportBuildPayload
): Promise<ExportBuildResult> {
  return invoke<ExportBuildResult>("api_export_mayhem_build", {
    payload,
  });
}

export async function swapChampSelectChampion(championId: number): Promise<boolean> {
  return invoke<boolean>("api_champ_select_swap", { championId });
}

export async function getLiveGameState(): Promise<LiveGameState> {
  return invoke<LiveGameState>("api_get_live_game_state");
}

export async function getLiveBuildAdvice(params: {
  championId: number;
  strategyId?: string;
  currentGold: number;
  inventoryItemIds: number[];
  level?: number;
  gameTime?: number;
  augmentIds?: string[];
}): Promise<MayhemLiveAdvice> {
  return invoke<MayhemLiveAdvice>("api_get_live_build_advice", {
    championId: params.championId,
    strategyId: params.strategyId ?? null,
    currentGold: params.currentGold,
    inventoryItemIds: params.inventoryItemIds,
    level: params.level ?? null,
    gameTime: params.gameTime ?? null,
    augmentIds: params.augmentIds ?? [],
  });
}
