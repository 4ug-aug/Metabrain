import { onOutlineSyncComplete, onOutlineSyncProgress, onSyncComplete, onSyncProgress, selectFolder, syncVault } from "@/api/tauri";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { useArtifacts, useDeleteArtifact, useSyncOutline } from "@/queries/sync";
import { useSettingsStore } from "@/stores/settingsStore";
import { useSyncStore } from "@/stores/syncStore";
import { Artifact, Settings as SettingsType } from "@/types";
import { invoke } from "@tauri-apps/api/tauri";
import {
  AlertCircle,
  BookOpen,
  Brain,
  CheckCircle,
  Database,
  Eye,
  EyeOff,
  FileText,
  FolderOpen,
  Loader2,
  RefreshCw,
  Save,
  Server,
  Trash2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

export function Settings() {
  const { settings, setSettings } = useSettingsStore();
  const { status, setStatus } = useSyncStore();
  const [isSaving, setIsSaving] = useState(false);
  const [localSettings, setLocalSettings] = useState<SettingsType>(settings);
  const [showApiKey, setShowApiKey] = useState(false);
  
  // Outline sync state
  const [outlineSyncStatus, setOutlineSyncStatus] = useState({
    isRunning: false,
    processed: 0,
    total: 0,
    currentDocument: "",
    lastSyncAt: null as number | null,
    error: null as string | null,
  });
  
  // Use TanStack Query for artifacts
  const { data: artifacts = [], refetch: refetchArtifacts } = useArtifacts();
  const deleteArtifactMutation = useDeleteArtifact();
  const syncOutlineMutation = useSyncOutline();

  // Load settings from backend on mount
  useEffect(() => {
    invoke<SettingsType>("get_settings")
      .then((backendSettings) => {
        setLocalSettings(backendSettings);
        setSettings(backendSettings);
      })
      .catch(console.error);
  }, [setSettings]);

  // Set up event listeners
  useEffect(() => {
    let unsubProgress: (() => void) | undefined;
    let unsubComplete: (() => void) | undefined;
    let unsubOutlineProgress: (() => void) | undefined;
    let unsubOutlineComplete: (() => void) | undefined;

    onSyncProgress((payload) => {
      setStatus({
        isRunning: true,
        processedFiles: payload.processed,
        totalFiles: payload.total,
      });
    }).then((unsub) => {
      unsubProgress = unsub;
    });

    onSyncComplete((payload) => {
      setStatus(payload);
      // Refresh artifacts
      refetchArtifacts();
      toast.success("Sync completed successfully!");
    }).then((unsub) => {
      unsubComplete = unsub;
    });

    onOutlineSyncProgress((payload) => {
      setOutlineSyncStatus((prev) => ({
        ...prev,
        isRunning: true,
        processed: payload.processed,
        total: payload.total,
        currentDocument: payload.currentDocument,
      }));
    }).then((unsub) => {
      unsubOutlineProgress = unsub;
    });

    onOutlineSyncComplete((payload) => {
      setOutlineSyncStatus({
        isRunning: false,
        processed: payload.processedFiles,
        total: payload.totalFiles,
        currentDocument: "",
        lastSyncAt: payload.lastSyncAt,
        error: payload.error,
      });
      refetchArtifacts();
      if (payload.error) {
        toast.error("Outline sync completed with errors");
      } else {
        toast.success("Outline sync completed successfully!");
      }
    }).then((unsub) => {
      unsubOutlineComplete = unsub;
    });

    return () => {
      unsubProgress?.();
      unsubComplete?.();
      unsubOutlineProgress?.();
      unsubOutlineComplete?.();
    };
  }, [setStatus, refetchArtifacts]);

  const handleSelectFolder = async () => {
    try {
      const selected = await selectFolder();
      if (selected) {
        setLocalSettings((prev) => ({ ...prev, vaultPath: selected }));
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
      toast.error("Failed to select folder");
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await invoke("save_settings", { settings: localSettings });
      setSettings(localSettings);
      toast.success("Settings saved successfully!");
    } catch (error) {
      console.error("Failed to save settings:", error);
      toast.error("Failed to save settings");
    } finally {
      setIsSaving(false);
    }
  };

  const handleSync = async () => {
    if (!localSettings.vaultPath) {
      toast.error("Please select a vault path first");
      return;
    }

    try {
      setStatus({ isRunning: true, error: null });
      await syncVault(localSettings.vaultPath);
    } catch (error) {
      console.error("Failed to sync vault:", error);
      toast.error("Failed to sync vault");
      setStatus({ isRunning: false, error: String(error) });
    }
  };

  const handleDeleteArtifact = async (artifact: Artifact) => {
    try {
      await deleteArtifactMutation.mutateAsync(artifact.id);
      toast.success(`Removed "${getFileName(artifact.path)}"`);
    } catch (error) {
      console.error("Failed to delete artifact:", error);
      toast.error("Failed to remove document");
    }
  };

  const handleSyncOutline = async () => {
    if (!localSettings.outlineApiKey) {
      toast.error("Please enter your Outline API key first");
      return;
    }

    try {
      setOutlineSyncStatus((prev) => ({ ...prev, isRunning: true, error: null }));
      await syncOutlineMutation.mutateAsync();
    } catch (error) {
      console.error("Failed to sync Outline:", error);
      toast.error("Failed to sync Outline");
      setOutlineSyncStatus((prev) => ({ ...prev, isRunning: false, error: String(error) }));
    }
  };

  const hasChanges =
    JSON.stringify(localSettings) !== JSON.stringify(settings);

  return (
    <ScrollArea className="h-full">
      <div className="container max-w-3xl py-8 px-6">
        <div className="space-y-6">
          {/* Header */}
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
            <p className="text-muted-foreground">
              Configure your Metamind knowledge assistant
            </p>
          </div>

          <Separator />

          {/* Vault Configuration */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Database className="h-5 w-5" />
                Knowledge Vault
              </CardTitle>
              <CardDescription>
                Select your Obsidian vault or notes directory to index
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="vault-path">Vault Path</Label>
                <div className="flex gap-2">
                  <Input
                    id="vault-path"
                    value={localSettings.vaultPath}
                    placeholder="Select a folder..."
                    readOnly
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectFolder}>
                    <FolderOpen className="h-4 w-4" />
                    Browse
                  </Button>
                </div>
              </div>

              {/* Sync Status */}
              <div className="rounded-lg border p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {status.isRunning ? (
                      <Loader2 className="h-4 w-4 animate-spin text-primary" />
                    ) : status.lastSyncAt ? (
                      <CheckCircle className="h-4 w-4 text-green-500" />
                    ) : (
                      <AlertCircle className="h-4 w-4 text-muted-foreground" />
                    )}
                    <span className="text-sm font-medium">
                      {status.isRunning
                        ? "Syncing..."
                        : status.lastSyncAt
                        ? "Synced"
                        : "Not synced"}
                    </span>
                  </div>
                  <Badge variant="secondary">
                    {artifacts.length} documents indexed
                  </Badge>
                </div>

                {status.isRunning && status.totalFiles > 0 && (
                  <div className="space-y-2">
                    <Progress
                      value={(status.processedFiles / status.totalFiles) * 100}
                    />
                    <p className="text-xs text-muted-foreground">
                      Processing {status.processedFiles} of {status.totalFiles}{" "}
                      files
                    </p>
                  </div>
                )}

                {status.lastSyncAt && !status.isRunning && (
                  <p className="text-xs text-muted-foreground">
                    Last synced:{" "}
                    {new Date(status.lastSyncAt * 1000).toLocaleString()}
                  </p>
                )}

                {status.error && (
                  <p className="text-xs text-destructive">{status.error}</p>
                )}

                <Button
                  onClick={handleSync}
                  disabled={status.isRunning || !localSettings.vaultPath}
                  className="w-full"
                  variant="outline"
                >
                  {status.isRunning ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Syncing...
                    </>
                  ) : (
                    <>
                      <RefreshCw className="h-4 w-4" />
                      Sync Now
                    </>
                  )}
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Indexed Documents */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <FileText className="h-5 w-5" />
                  Indexed Documents
                </div>
                <Badge variant="secondary">{artifacts.length} files</Badge>
              </CardTitle>
              <CardDescription>
                View and manage documents in your knowledge base
              </CardDescription>
            </CardHeader>
            <CardContent>
              {artifacts.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <FileText className="h-12 w-12 mx-auto mb-3 opacity-50" />
                  <p className="text-sm">No documents indexed yet</p>
                  <p className="text-xs mt-1">Sync your vault to add documents</p>
                </div>
              ) : (
                <ScrollArea className="h-[300px] pr-4">
                  <div className="space-y-2">
                    {artifacts.map((artifact) => (
                      <ArtifactItem
                        key={artifact.id}
                        artifact={artifact}
                        onDelete={handleDeleteArtifact}
                        isDeleting={deleteArtifactMutation.isPending}
                      />
                    ))}
                  </div>
                </ScrollArea>
              )}
            </CardContent>
          </Card>

          {/* Ollama Configuration */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Server className="h-5 w-5" />
                Ollama Connection
              </CardTitle>
              <CardDescription>
                Configure your local Ollama instance for AI processing
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="ollama-endpoint">Ollama Endpoint</Label>
                <Input
                  id="ollama-endpoint"
                  value={localSettings.ollamaEndpoint}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      ollamaEndpoint: e.target.value,
                    }))
                  }
                  placeholder="http://localhost:11434"
                />
                <p className="text-xs text-muted-foreground">
                  The URL where your Ollama server is running
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Model Configuration */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Brain className="h-5 w-5" />
                AI Models
              </CardTitle>
              <CardDescription>
                Choose which models to use for chat and embeddings
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="chat-model">Chat Model</Label>
                <Input
                  id="chat-model"
                  value={localSettings.ollamaModel}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      ollamaModel: e.target.value,
                    }))
                  }
                  placeholder="llama3.2"
                />
                <p className="text-xs text-muted-foreground">
                  The model used for generating responses (e.g., llama3.2,
                  mistral, mixtral)
                </p>
              </div>

              <Separator />

              <div className="space-y-2">
                <Label htmlFor="embedding-model">Embedding Model</Label>
                <Input
                  id="embedding-model"
                  value={localSettings.embeddingModel}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      embeddingModel: e.target.value,
                    }))
                  }
                  placeholder="nomic-embed-text"
                />
                <p className="text-xs text-muted-foreground">
                  The model used for creating vector embeddings of your documents
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Outline Wiki Integration */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BookOpen className="h-5 w-5" />
                Outline Wiki
              </CardTitle>
              <CardDescription>
                Connect to your Outline Wiki to index documentation
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="outline-base-url">Outline API URL</Label>
                <Input
                  id="outline-base-url"
                  value={localSettings.outlineBaseUrl}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      outlineBaseUrl: e.target.value,
                    }))
                  }
                  placeholder="https://app.getoutline.com/api"
                />
                <p className="text-xs text-muted-foreground">
                  The API URL for your Outline instance (use default for cloud-hosted)
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="outline-api-key">API Key</Label>
                <div className="flex gap-2">
                  <Input
                    id="outline-api-key"
                    type={showApiKey ? "text" : "password"}
                    value={localSettings.outlineApiKey}
                    onChange={(e) =>
                      setLocalSettings((prev) => ({
                        ...prev,
                        outlineApiKey: e.target.value,
                      }))
                    }
                    placeholder="Enter your Outline API key"
                    className="flex-1"
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                  >
                    {showApiKey ? (
                      <EyeOff className="h-4 w-4" />
                    ) : (
                      <Eye className="h-4 w-4" />
                    )}
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Create an API key in Outline under Settings → API & Apps
                </p>
              </div>

              {/* Outline Sync Status */}
              <div className="rounded-lg border p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {outlineSyncStatus.isRunning ? (
                      <Loader2 className="h-4 w-4 animate-spin text-primary" />
                    ) : outlineSyncStatus.lastSyncAt ? (
                      <CheckCircle className="h-4 w-4 text-green-500" />
                    ) : (
                      <AlertCircle className="h-4 w-4 text-muted-foreground" />
                    )}
                    <span className="text-sm font-medium">
                      {outlineSyncStatus.isRunning
                        ? "Syncing Outline..."
                        : outlineSyncStatus.lastSyncAt
                        ? "Synced"
                        : "Not synced"}
                    </span>
                  </div>
                  <Badge variant="secondary">
                    {artifacts.filter((a) => a.path.startsWith("outline://")).length} docs
                  </Badge>
                </div>

                {outlineSyncStatus.isRunning && outlineSyncStatus.total > 0 && (
                  <div className="space-y-2">
                    <Progress
                      value={(outlineSyncStatus.processed / outlineSyncStatus.total) * 100}
                    />
                    <p className="text-xs text-muted-foreground">
                      Processing {outlineSyncStatus.processed} of {outlineSyncStatus.total}{" "}
                      documents
                      {outlineSyncStatus.currentDocument && (
                        <span className="block truncate mt-1">
                          Current: {outlineSyncStatus.currentDocument}
                        </span>
                      )}
                    </p>
                  </div>
                )}

                {outlineSyncStatus.lastSyncAt && !outlineSyncStatus.isRunning && (
                  <p className="text-xs text-muted-foreground">
                    Last synced:{" "}
                    {new Date(outlineSyncStatus.lastSyncAt * 1000).toLocaleString()}
                  </p>
                )}

                {outlineSyncStatus.error && (
                  <p className="text-xs text-destructive">{outlineSyncStatus.error}</p>
                )}

                <Button
                  onClick={handleSyncOutline}
                  disabled={outlineSyncStatus.isRunning || !localSettings.outlineApiKey}
                  className="w-full"
                  variant="outline"
                >
                  {outlineSyncStatus.isRunning ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Syncing Outline...
                    </>
                  ) : (
                    <>
                      <RefreshCw className="h-4 w-4" />
                      Sync Outline
                    </>
                  )}
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Save Button */}
          <div className="flex justify-end gap-2">
            {hasChanges && (
              <Badge variant="outline" className="mr-auto">
                Unsaved changes
              </Badge>
            )}
            <Button onClick={handleSave} disabled={isSaving || !hasChanges}>
              {isSaving ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  <Save className="h-4 w-4" />
                  Save Settings
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}

// Helper to extract filename from path
function getFileName(path: string): string {
  return path.split("/").pop() || path.split("\\").pop() || path;
}

// Helper to format timestamp
function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

// Artifact item component
interface ArtifactItemProps {
  artifact: Artifact;
  onDelete: (artifact: Artifact) => void;
  isDeleting: boolean;
}

function ArtifactItem({ artifact, onDelete, isDeleting }: ArtifactItemProps) {
  const fileName = getFileName(artifact.path);

  return (
    <div className="flex items-start gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors">
      <FileText className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
      <div className="flex-1 min-w-0">
        <p className="font-medium text-sm truncate">{fileName}</p>
        <p className="text-xs text-muted-foreground truncate">{artifact.path}</p>
        <p className="text-xs text-muted-foreground mt-1">
          Indexed: {formatDate(artifact.indexedAt)}
        </p>
      </div>
      <AlertDialog>
        <AlertDialogTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            className="shrink-0 text-muted-foreground hover:text-destructive"
            disabled={isDeleting}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove document?</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove "{fileName}" from your knowledge base. The original
              file will not be deleted. You can re-sync to add it back.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => onDelete(artifact)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
