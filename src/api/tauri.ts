import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/dialog";
import { ChatMessage, Settings, SyncStatus, Artifact } from "../types";

// Settings Commands
export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

// Chat Commands
export async function sendMessage(query: string): Promise<void> {
  return invoke("send_message", { query });
}

export async function getChatHistory(): Promise<ChatMessage[]> {
  return invoke<ChatMessage[]>("get_chat_history");
}

export async function clearChat(): Promise<void> {
  return invoke("clear_chat");
}

// Sync Commands
export async function syncVault(vaultPath: string): Promise<SyncStatus> {
  return invoke<SyncStatus>("sync_vault", { vaultPath });
}

export async function getSyncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>("get_sync_status");
}

export async function getArtifacts(): Promise<Artifact[]> {
  return invoke<Artifact[]>("get_artifacts");
}

export async function deleteArtifact(id: string): Promise<void> {
  return invoke("delete_artifact", { id });
}

// Dialog Commands
export async function selectFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Obsidian Vault",
  });
  return selected as string | null;
}

// Event Listeners
export type StreamChunkPayload = {
  content: string;
  done: boolean;
};

export type SyncProgressPayload = {
  processed: number;
  total: number;
  currentFile: string;
};

export function onStreamChunk(
  callback: (payload: StreamChunkPayload) => void
): Promise<() => void> {
  return listen<StreamChunkPayload>("stream-chunk", (event) => {
    callback(event.payload);
  });
}

export function onSyncProgress(
  callback: (payload: SyncProgressPayload) => void
): Promise<() => void> {
  return listen<SyncProgressPayload>("sync-progress", (event) => {
    callback(event.payload);
  });
}

export function onSyncComplete(
  callback: (payload: SyncStatus) => void
): Promise<() => void> {
  return listen<SyncStatus>("sync-complete", (event) => {
    callback(event.payload);
  });
}

export function onSyncError(
  callback: (payload: { error: string }) => void
): Promise<() => void> {
  return listen<{ error: string }>("sync-error", (event) => {
    callback(event.payload);
  });
}

