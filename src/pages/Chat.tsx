import { onStreamChunk } from "@/api/tauri";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import { ChatMessage } from "@/types";
import { invoke } from "@tauri-apps/api/tauri";
import {
  Bot,
  ChevronDown,
  FileText,
  Loader2,
  Send,
  Sparkles,
  Trash2,
  User,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import Markdown from 'react-markdown';
import { toast } from "sonner";

export function Chat() {
  const {
    messages,
    setMessages,
    addMessage,
    clearMessages,
    isStreaming,
    setStreaming,
    streamingContent,
    appendStreamingContent,
    clearStreamingContent,
  } = useChatStore();

  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Load chat history on mount
  useEffect(() => {
    invoke<ChatMessage[]>("get_chat_history")
      .then((history) => {
        setMessages(history);
      })
      .catch(console.error);
  }, [setMessages]);

  // Set up streaming listener
  useEffect(() => {
    let unsubscribe: (() => void) | undefined;

    onStreamChunk((payload) => {
      if (payload.done) {
        setStreaming(false);
        // Refresh chat history to get the final message
        invoke<ChatMessage[]>("get_chat_history")
          .then((history) => {
            setMessages(history);
            clearStreamingContent();
          })
          .catch(console.error);
      } else {
        appendStreamingContent(payload.content);
      }
    }).then((unsub) => {
      unsubscribe = unsub;
    });

    return () => {
      unsubscribe?.();
    };
  }, [setStreaming, appendStreamingContent, clearStreamingContent, setMessages]);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, streamingContent]);

  const handleSend = async () => {
    const trimmedInput = input.trim();
    if (!trimmedInput || isStreaming) return;

    setInput("");
    setStreaming(true);
    clearStreamingContent();

    // Optimistically add user message
    const userMessage: ChatMessage = {
      id: Date.now(),
      role: "user",
      content: trimmedInput,
      timestamp: Math.floor(Date.now() / 1000),
    };
    addMessage(userMessage);

    try {
      await invoke("send_message", { query: trimmedInput });
    } catch (error) {
      console.error("Failed to send message:", error);
      toast.error("Failed to send message. Make sure Ollama is running.");
      setStreaming(false);
    }
  };

  const handleClear = async () => {
    try {
      await invoke("clear_chat");
      clearMessages();
      toast.success("Chat history cleared");
    } catch (error) {
      console.error("Failed to clear chat:", error);
      toast.error("Failed to clear chat");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
            <Sparkles className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h1 className="font-semibold">Metamind</h1>
            <p className="text-xs text-muted-foreground">
              Ask questions about your knowledge base
            </p>
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleClear}
          disabled={messages.length === 0}
        >
          <Trash2 className="h-4 w-4" />
          Clear
        </Button>
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 px-6 max-h-[75vh]" ref={scrollRef}>
        <div className="py-6 space-y-6">
          {messages.length === 0 && !isStreaming ? (
            <EmptyState />
          ) : (
            <>
              {messages.map((message) => (
                <MessageBubble key={message.id} message={message} />
              ))}
              {isStreaming && streamingContent && (
                <MessageBubble
                  message={{
                    id: -1,
                    role: "assistant",
                    content: streamingContent,
                    timestamp: Date.now() / 1000,
                  }}
                  isStreaming
                />
              )}
              {isStreaming && !streamingContent && (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span className="text-sm">Thinking...</span>
                </div>
              )}
            </>
          )}
        </div>
      </ScrollArea>

      {/* Input */}
      <div className="border-t p-4">
        <div className="flex gap-2">
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask a question about your notes..."
            className="min-h-[60px] max-h-[200px] resize-none"
            disabled={isStreaming}
          />
          <Button
            onClick={handleSend}
            disabled={!input.trim() || isStreaming}
            size="icon"
            className="h-[60px] w-[60px] shrink-0"
          >
            {isStreaming ? (
              <Loader2 className="h-5 w-5 animate-spin" />
            ) : (
              <Send className="h-5 w-5" />
            )}
          </Button>
        </div>
        <p className="mt-2 text-xs text-muted-foreground text-center">
          Press Enter to send, Shift+Enter for new line
        </p>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-primary/10 mb-4">
        <Sparkles className="h-8 w-8 text-primary" />
      </div>
      <h2 className="text-xl font-semibold mb-2">Welcome to Metamind</h2>
      <p className="text-muted-foreground max-w-md mb-6">
        Your personal AI assistant for exploring your knowledge base. Ask questions
        about your Obsidian notes and get intelligent answers.
      </p>
      <div className="grid gap-2 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4" />
          <span>Make sure you've synced your vault in Settings</span>
        </div>
        <div className="flex items-center gap-2">
          <Bot className="h-4 w-4" />
          <span>Ensure Ollama is running locally</span>
        </div>
      </div>
    </div>
  );
}

interface MessageBubbleProps {
  message: ChatMessage;
  isStreaming?: boolean;
}

function MessageBubble({ message, isStreaming }: MessageBubbleProps) {
  const isUser = message.role === "user";

  return (
    <div
      className={cn(
        "flex gap-3",
        isUser ? "flex-row-reverse" : "flex-row"
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          "flex h-8 w-8 shrink-0 items-center justify-center rounded-lg",
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-muted text-muted-foreground"
        )}
      >
        {isUser ? <User className="h-4 w-4" /> : <Bot className="h-4 w-4" />}
      </div>

      {/* Content */}
      <div
        className={cn(
          "flex flex-col gap-1 max-w-[80%]",
          isUser ? "items-end" : "items-start"
        )}
      >
        <Card
          className={cn(
            "py-3",
            isUser ? "bg-primary text-primary-foreground" : "bg-muted"
          )}
        >
          <CardContent className="p-0 px-4">
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <MessageContent content={message.content} />
            </div>
            {isStreaming && (
              <span className="inline-block w-2 h-4 bg-current animate-pulse ml-1" />
            )}
          </CardContent>
        </Card>

        {/* Timestamp */}
        <span className="text-xs text-muted-foreground">
          {formatTimestamp(message.timestamp)}
        </span>

        {/* Sources (for assistant messages) */}
        {!isUser && message.sources && message.sources.length > 0 && (
          <SourcesCitation sources={message.sources} />
        )}
      </div>
    </div>
  );
}

function MessageContent({ content }: { content: string }) {
  return (
    <Markdown>{content}</Markdown>
  );
}

interface Source {
  path: string;
  title: string;
  chunk: string;
  similarity: number;
}

function SourcesCitation({ sources }: { sources: Source[] }) {
  return (
    <Collapsible className="w-full mt-2">
      <CollapsibleTrigger asChild>
        <Button variant="ghost" size="sm" className="h-auto py-1 px-2">
          <FileText className="h-3 w-3 mr-1" />
          <span className="text-xs">{sources.length} sources</span>
          <ChevronDown className="h-3 w-3 ml-1" />
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="mt-2 space-y-2">
          {sources.map((source, i) => (
            <Card key={i} className="py-2">
              <CardContent className="p-0 px-3">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-medium truncate">
                    {source.title || source.path}
                  </span>
                  <Badge variant="secondary" className="text-xs">
                    {Math.round(source.similarity * 100)}%
                  </Badge>
                </div>
                <p className="text-xs text-muted-foreground line-clamp-2">
                  {source.chunk}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  // Less than a minute
  if (diff < 60000) {
    return "Just now";
  }

  // Less than an hour
  if (diff < 3600000) {
    const minutes = Math.floor(diff / 60000);
    return `${minutes}m ago`;
  }

  // Same day
  if (date.toDateString() === now.toDateString()) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  // Different day
  return date.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
