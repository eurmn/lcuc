import { invoke } from "@tauri-apps/api/core";

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    console.error(`[IPC Error] ${cmd}:`, error);
    throw typeof error === "string" ? error : JSON.stringify(error);
  }
}
