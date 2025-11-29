import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getSyncStatus, syncVault, getArtifacts, deleteArtifact, syncOutline } from "../api/tauri";

export const syncKeys = {
  all: ["sync"] as const,
  status: () => [...syncKeys.all, "status"] as const,
  artifacts: () => [...syncKeys.all, "artifacts"] as const,
};

export function useSyncStatus() {
  return useQuery({
    queryKey: syncKeys.status(),
    queryFn: getSyncStatus,
    refetchInterval: (query) => {
      // Refetch every second while syncing
      return query.state.data?.isRunning ? 1000 : false;
    },
  });
}

export function useArtifacts() {
  return useQuery({
    queryKey: syncKeys.artifacts(),
    queryFn: getArtifacts,
  });
}

export function useSyncVault() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (vaultPath: string) => syncVault(vaultPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: syncKeys.all });
    },
  });
}

export function useDeleteArtifact() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => deleteArtifact(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: syncKeys.artifacts() });
    },
  });
}

export function useSyncOutline() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => syncOutline(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: syncKeys.all });
    },
  });
}

