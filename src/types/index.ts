// Metabrain Type Definitions

export interface ChatMessage {
  id: number;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
  sources?: SourceCitation[];
}

export interface SourceCitation {
  path: string;
  title: string;
  chunk: string;
  similarity: number;
}

export interface Artifact {
  id: string;
  path: string;
  lastModified: number;
  contentHash: string;
  indexedAt: number;
}

export interface Settings {
  vaultPath: string;
  ollamaEndpoint: string;
  ollamaModel: string;
  embeddingModel: string;
  outlineApiKey: string;
  outlineBaseUrl: string;
}

export interface SyncStatus {
  isRunning: boolean;
  totalFiles: number;
  processedFiles: number;
  lastSyncAt: number | null;
  error: string | null;
}

export interface EmbeddingChunk {
  id: string;
  artifactId: string;
  chunkIndex: number;
  content: string;
}

// Default settings
export const DEFAULT_SETTINGS: Settings = {
  vaultPath: "",
  ollamaEndpoint: "http://localhost:11434",
  ollamaModel: "llama3.2",
  embeddingModel: "nomic-embed-text",
  outlineApiKey: "",
  outlineBaseUrl: "https://app.getoutline.com/api",
};

