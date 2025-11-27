use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Lock error")]
    Lock,
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub last_modified: i64,
    pub content_hash: String,
    pub indexed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub id: String,
    pub artifact_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub vault_path: String,
    pub ollama_endpoint: String,
    pub ollama_model: String,
    pub embedding_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vault_path: String::new(),
            ollama_endpoint: "http://localhost:11434".to_string(),
            ollama_model: "llama3.2".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
        }
    }
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> DbResult<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();
        let db_path = app_data_dir.join("metamind.db");
        let conn = Connection::open(db_path)?;
        
        let db = Self {
            conn: Mutex::new(conn),
        };
        
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        
        // Create artifacts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                last_modified INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                indexed_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create embeddings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                id TEXT PRIMARY KEY,
                artifact_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL,
                FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create index on artifact_id for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embeddings_artifact_id ON embeddings(artifact_id)",
            [],
        )?;

        // Create chat_messages table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    // === Artifact Methods ===

    pub fn upsert_artifact(&self, artifact: &Artifact) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute(
            "INSERT INTO artifacts (id, path, last_modified, content_hash, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                last_modified = excluded.last_modified,
                content_hash = excluded.content_hash,
                indexed_at = excluded.indexed_at",
            params![
                artifact.id,
                artifact.path,
                artifact.last_modified,
                artifact.content_hash,
                artifact.indexed_at
            ],
        )?;
        Ok(())
    }

    pub fn get_artifact_by_path(&self, path: &str) -> DbResult<Option<Artifact>> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut stmt = conn.prepare(
            "SELECT id, path, last_modified, content_hash, indexed_at FROM artifacts WHERE path = ?1"
        )?;
        
        let result = stmt.query_row([path], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                path: row.get(1)?,
                last_modified: row.get(2)?,
                content_hash: row.get(3)?,
                indexed_at: row.get(4)?,
            })
        });

        match result {
            Ok(artifact) => Ok(Some(artifact)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    pub fn get_all_artifacts(&self) -> DbResult<Vec<Artifact>> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut stmt = conn.prepare(
            "SELECT id, path, last_modified, content_hash, indexed_at FROM artifacts"
        )?;
        
        let artifacts = stmt.query_map([], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                path: row.get(1)?,
                last_modified: row.get(2)?,
                content_hash: row.get(3)?,
                indexed_at: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(artifacts)
    }

    pub fn delete_artifact(&self, id: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute("DELETE FROM artifacts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn delete_artifact_by_path(&self, path: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        // First delete embeddings
        conn.execute(
            "DELETE FROM embeddings WHERE artifact_id IN (SELECT id FROM artifacts WHERE path = ?1)",
            [path],
        )?;
        // Then delete artifact
        conn.execute("DELETE FROM artifacts WHERE path = ?1", [path])?;
        Ok(())
    }

    // === Embedding Methods ===

    pub fn insert_embedding(&self, embedding: &Embedding) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let embedding_bytes = embedding_to_bytes(&embedding.embedding);
        
        conn.execute(
            "INSERT INTO embeddings (id, artifact_id, chunk_index, content, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                embedding.id,
                embedding.artifact_id,
                embedding.chunk_index,
                embedding.content,
                embedding_bytes
            ],
        )?;
        Ok(())
    }

    pub fn delete_embeddings_by_artifact(&self, artifact_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute(
            "DELETE FROM embeddings WHERE artifact_id = ?1",
            [artifact_id],
        )?;
        Ok(())
    }

    pub fn get_all_embeddings(&self) -> DbResult<Vec<Embedding>> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut stmt = conn.prepare(
            "SELECT id, artifact_id, chunk_index, content, embedding FROM embeddings"
        )?;
        
        let embeddings = stmt.query_map([], |row| {
            let embedding_bytes: Vec<u8> = row.get(4)?;
            Ok(Embedding {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                embedding: bytes_to_embedding(&embedding_bytes),
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(embeddings)
    }

    // === Chat Message Methods ===

    pub fn insert_chat_message(&self, role: &str, content: &str) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        conn.execute(
            "INSERT INTO chat_messages (role, content, timestamp) VALUES (?1, ?2, ?3)",
            params![role, content, timestamp],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    pub fn get_chat_history(&self) -> DbResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut stmt = conn.prepare(
            "SELECT id, role, content, timestamp FROM chat_messages ORDER BY timestamp ASC"
        )?;
        
        let messages = stmt.query_map([], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(messages)
    }

    pub fn clear_chat_history(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute("DELETE FROM chat_messages", [])?;
        Ok(())
    }

    // === Settings Methods ===

    pub fn get_settings(&self) -> DbResult<Settings> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        
        let mut settings = Settings::default();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows.flatten() {
            match row.0.as_str() {
                "vault_path" => settings.vault_path = row.1,
                "ollama_endpoint" => settings.ollama_endpoint = row.1,
                "ollama_model" => settings.ollama_model = row.1,
                "embedding_model" => settings.embedding_model = row.1,
                _ => {}
            }
        }
        
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        
        let pairs = [
            ("vault_path", &settings.vault_path),
            ("ollama_endpoint", &settings.ollama_endpoint),
            ("ollama_model", &settings.ollama_model),
            ("embedding_model", &settings.embedding_model),
        ];

        for (key, value) in pairs {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        
        Ok(())
    }
}

// Helper functions to convert embeddings to/from bytes
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}

