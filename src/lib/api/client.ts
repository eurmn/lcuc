import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    logger.error("frontend.ipc", `IPC command failed: ${cmd}`, error);
    throw typeof error === "string" ? error : JSON.stringify(error);
  }
}
