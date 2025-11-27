import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getChatHistory, clearChat, sendMessage } from "../api/tauri";

export const chatKeys = {
  all: ["chat"] as const,
  history: () => [...chatKeys.all, "history"] as const,
};

export function useChatHistory() {
  return useQuery({
    queryKey: chatKeys.history(),
    queryFn: getChatHistory,
  });
}

export function useSendMessage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (query: string) => sendMessage(query),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: chatKeys.history() });
    },
  });
}

export function useClearChat() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clearChat,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: chatKeys.history() });
    },
  });
}

