import { create } from "zustand";
import { SyncStatus } from "../types";

interface SyncState {
  status: SyncStatus;
  setStatus: (status: Partial<SyncStatus>) => void;
  resetStatus: () => void;
}

const DEFAULT_SYNC_STATUS: SyncStatus = {
  isRunning: false,
  totalFiles: 0,
  processedFiles: 0,
  lastSyncAt: null,
  error: null,
};

export const useSyncStore = create<SyncState>((set) => ({
  status: DEFAULT_SYNC_STATUS,

  setStatus: (newStatus) =>
    set((state) => ({
      status: { ...state.status, ...newStatus },
    })),

  resetStatus: () => set({ status: DEFAULT_SYNC_STATUS }),
}));

