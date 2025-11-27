import { onSyncComplete, onSyncProgress, selectFolder, syncVault } from "@/api/tauri";
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
import { useSettingsStore } from "@/stores/settingsStore";
import { useSyncStore } from "@/stores/syncStore";
import { Settings as SettingsType } from "@/types";
import { invoke } from "@tauri-apps/api/tauri";
import {
  AlertCircle,
  Brain,
  CheckCircle,
  Database,
  FolderOpen,
  Loader2,
  RefreshCw,
  Save,
  Server,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

export function Settings() {
  const { settings, setSettings } = useSettingsStore();
  const { status, setStatus } = useSyncStore();
  const [isSaving, setIsSaving] = useState(false);
  const [localSettings, setLocalSettings] = useState<SettingsType>(settings);
  const [artifactCount, setArtifactCount] = useState(0);

  // Load settings from backend on mount
  useEffect(() => {
    invoke<SettingsType>("get_settings")
      .then((backendSettings) => {
        setLocalSettings(backendSettings);
        setSettings(backendSettings);
      })
      .catch(console.error);

    // Get artifact count
    invoke<unknown[]>("get_artifacts")
      .then((artifacts) => setArtifactCount(artifacts.length))
      .catch(console.error);
  }, [setSettings]);

  // Set up event listeners
  useEffect(() => {
    let unsubProgress: (() => void) | undefined;
    let unsubComplete: (() => void) | undefined;

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
      // Refresh artifact count
      invoke<unknown[]>("get_artifacts")
        .then((artifacts) => setArtifactCount(artifacts.length))
        .catch(console.error);
      toast.success("Sync completed successfully!");
    }).then((unsub) => {
      unsubComplete = unsub;
    });

    return () => {
      unsubProgress?.();
      unsubComplete?.();
    };
  }, [setStatus]);

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
                    {artifactCount} documents indexed
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
