import { invoke } from "@tauri-apps/api/core";
import type {
  ChampSelectSession,
  ExportBuildPayload,
  ExportBuildResult,
  MayhemAugmentsQuery,
  MayhemAugmentsResponse,
  MayhemChampionBuildSummary,
} from "../types/champ_select";

export async function getChampSelectSession(): Promise<ChampSelectSession | null> {
  return invoke<ChampSelectSession | null>("api_champ_select_session");
}

export async function getMayhemChampionBuild(
  championId: number
): Promise<MayhemChampionBuildSummary> {
  return invoke<MayhemChampionBuildSummary>("api_mayhem_champion_build", {
    championId,
  });
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
