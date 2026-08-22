/**
 * Rune Builder & Telemetry Tauri IPC client wrappers.
 */

import { tauriInvoke } from "./client";
import type {
  BuilderQuery,
  EnrichedKeystoneOption,
  RuneBuilderOverview,
  SecondaryTreeOption,
  StatShardsStats,
  TreePerksStats,
} from "../types/rune_builder";

/**
 * Fetches the aggregated builder overview for a champion, role, and tier selection.
 */
export async function getBuilderOverview(query: BuilderQuery): Promise<RuneBuilderOverview> {
  return tauriInvoke<RuneBuilderOverview>("api_rune_builder_get_overview", { query });
}

/**
 * Fetches all viable keystones with pick rates and WPA.
 */
export async function getBuilderKeystones(
  query: BuilderQuery
): Promise<EnrichedKeystoneOption[]> {
  return tauriInvoke<EnrichedKeystoneOption[]>("api_rune_builder_get_keystones", { query });
}

/**
 * Fetches the 3-row tier runes for a specific style tree.
 */
export async function getBuilderTreeRunes(
  mainTree: number,
  treeToLoad: number,
  keystone: number | null,
  query: BuilderQuery
): Promise<TreePerksStats> {
  return tauriInvoke<TreePerksStats>("api_rune_builder_get_tree_runes", {
    mainTree,
    treeToLoad,
    keystone,
    query,
  });
}

/**
 * Fetches secondary tree distribution and popularity.
 */
export async function getBuilderSecondaryPlaycounts(
  tree: number,
  keystone: number | null,
  query: BuilderQuery
): Promise<SecondaryTreeOption[]> {
  return tauriInvoke<SecondaryTreeOption[]>("api_rune_builder_get_secondary_playcounts", {
    tree,
    keystone,
    query,
  });
}

/**
 * Fetches stat shard performance metrics for Offense, Flex, and Defense.
 */
export async function getBuilderShards(
  keystone: number | null,
  query: BuilderQuery
): Promise<StatShardsStats> {
  return tauriInvoke<StatShardsStats>("api_rune_builder_get_shards", {
    keystone,
    query,
  });
}
