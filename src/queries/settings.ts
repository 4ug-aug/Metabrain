import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getSettings, saveSettings } from "../api/tauri";
import { Settings } from "../types";

export const settingsKeys = {
  all: ["settings"] as const,
  detail: () => [...settingsKeys.all, "detail"] as const,
};

export function useSettings() {
  return useQuery({
    queryKey: settingsKeys.detail(),
    queryFn: getSettings,
    staleTime: Infinity,
  });
}

export function useSaveSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: Settings) => saveSettings(settings),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
    },
  });
}

