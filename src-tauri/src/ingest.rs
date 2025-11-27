use crate::db::{Artifact, Database, Embedding};
use crate::embedding::EmbeddingClient;
use crate::parser::MarkdownParser;
use crate::watcher::scan_directory;
use crate::SyncStatus;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Database error: {0}")]
    Database(#[from] crate::db::DbError),
    #[error("Parser error: {0}")]
    Parser(#[from] crate::parser::ParseError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] crate::embedding::EmbeddingError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type IngestResult<T> = Result<T, IngestError>;

pub struct IngestEngine {
    db: Arc<Database>,
    parser: MarkdownParser,
    embedding_client: EmbeddingClient,
    status: SyncStatus,
}

impl IngestEngine {
    pub fn new(db: Arc<Database>, ollama_endpoint: String, embedding_model: String) -> Self {
        Self {
            db,
            parser: MarkdownParser::new(),
            embedding_client: EmbeddingClient::new(ollama_endpoint, embedding_model),
            status: SyncStatus::default(),
        }
    }

    pub fn get_status(&self) -> SyncStatus {
        self.status.clone()
    }

    pub async fn sync_vault(
        &mut self,
        vault_path: &str,
        app_handle: &tauri::AppHandle,
    ) -> IngestResult<SyncStatus> {
        let path = Path::new(vault_path);
        
        if !path.exists() || !path.is_dir() {
            self.status.error = Some("Invalid vault path".to_string());
            return Ok(self.status.clone());
        }

        self.status.is_running = true;
        self.status.error = None;
        self.status.processed_files = 0;
        
        // Scan for all markdown files
        let files = scan_directory(path);
        self.status.total_files = files.len();
        
        // Emit initial progress
        let _ = app_handle.emit_all("sync-progress", serde_json::json!({
            "processed": 0,
            "total": self.status.total_files,
            "currentFile": ""
        }));

        // Process each file
        for file_path in files {
            let file_name = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            // Emit progress
            let _ = app_handle.emit_all("sync-progress", serde_json::json!({
                "processed": self.status.processed_files,
                "total": self.status.total_files,
                "currentFile": file_name
            }));

            if let Err(e) = self.process_file(&file_path).await {
                log::warn!("Failed to process file {:?}: {}", file_path, e);
                // Continue with other files
            }
            
            self.status.processed_files += 1;
        }

        self.status.is_running = false;
        self.status.last_sync_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        );

        // Emit completion
        let _ = app_handle.emit_all("sync-complete", &self.status);

        Ok(self.status.clone())
    }

    pub async fn process_file(&mut self, path: &Path) -> IngestResult<()> {
        let path_str = path.to_string_lossy().to_string();
        log::info!("Processing file {:?}", path_str);
        // Parse the markdown file
        let parsed = self.parser.parse_file(path)?;
        
        // Check if file has changed
        if let Some(existing) = self.db.get_artifact_by_path(&path_str)? {
            if existing.content_hash == parsed.content_hash {
                // File hasn't changed, skip
                return Ok(());
            }
            // File has changed, delete old embeddings
            self.db.delete_embeddings_by_artifact(&existing.id)?;
        }
        
        // Get file metadata
        let metadata = std::fs::metadata(path)?;
        let last_modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        // Create artifact ID
        let artifact_id = Uuid::new_v4().to_string();
        
        // Create and store artifact
        let artifact = Artifact {
            id: artifact_id.clone(),
            path: path_str,
            last_modified,
            content_hash: parsed.content_hash,
            indexed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        self.db.upsert_artifact(&artifact)?;
        
        // Process each chunk
        for (chunk_index, chunk_content) in parsed.chunks.iter().enumerate() {
            // Generate embedding
            let embedding_vec = self.embedding_client.embed(chunk_content).await?;
            
            // Create embedding record
            let embedding = Embedding {
                id: format!("{}#{}", artifact_id, chunk_index),
                artifact_id: artifact_id.clone(),
                chunk_index: chunk_index as i32,
                content: chunk_content.clone(),
                embedding: embedding_vec,
            };
            
            self.db.insert_embedding(&embedding)?;
        }
        
        Ok(())
    }

    pub async fn remove_file(&mut self, path: &Path) -> IngestResult<()> {
        let path_str = path.to_string_lossy().to_string();
        self.db.delete_artifact_by_path(&path_str)?;
        Ok(())
    }
}

