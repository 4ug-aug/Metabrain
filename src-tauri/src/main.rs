// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod embedding;
mod ingest;
mod llm;
mod parser;
mod rag;
mod vector;
mod watcher;

use db::{Database, ChatMessage, Settings, Artifact};
use ingest::IngestEngine;
use rag::RagEngine;
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex as TokioMutex;
use serde::{Deserialize, Serialize};

// Application state
pub struct AppState {
    pub db: Arc<Database>,
    pub ingest_engine: Arc<TokioMutex<Option<IngestEngine>>>,
    pub rag_engine: Arc<TokioMutex<RagEngine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub is_running: bool,
    pub total_files: usize,
    pub processed_files: usize,
    pub last_sync_at: Option<i64>,
    pub error: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            total_files: 0,
            processed_files: 0,
            last_sync_at: None,
            error: None,
        }
    }
}

// === Settings Commands ===

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state.db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    // Save settings to database
    state.db.save_settings(&settings).map_err(|e| e.to_string())?;
    
    // Update RAG engine with new settings
    let mut rag_engine = state.rag_engine.lock().await;
    rag_engine.update_settings(
        state.db.clone(),
        settings.ollama_endpoint.clone(),
        settings.ollama_model.clone(),
        settings.embedding_model.clone(),
    );
    
    // Also update ingest engine if it exists
    let mut ingest_engine_guard = state.ingest_engine.lock().await;
    if ingest_engine_guard.is_some() {
        let engine = IngestEngine::new(
            state.db.clone(),
            settings.ollama_endpoint,
            settings.embedding_model,
        );
        *ingest_engine_guard = Some(engine);
    }
    
    Ok(())
}

// === Chat Commands ===

#[tauri::command]
async fn get_chat_history(state: State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
    state.db.get_chat_history().map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_chat(state: State<'_, AppState>) -> Result<(), String> {
    state.db.clear_chat_history().map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_message(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    query: String,
) -> Result<(), String> {
    // Get chat history BEFORE adding the new message
    let chat_history = state.db.get_chat_history().map_err(|e| e.to_string())?;
    
    // Save user message
    state.db.insert_chat_message("user", &query).map_err(|e| e.to_string())?;
    
    // Process through RAG engine with chat context
    let rag_engine = state.rag_engine.lock().await;
    
    match rag_engine.query(&query, &chat_history, &app_handle).await {
        Ok(response) => {
            // Save assistant response
            state.db.insert_chat_message("assistant", &response).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Error: {}", e);
            state.db.insert_chat_message("assistant", &error_msg).ok();
            Err(e.to_string())
        }
    }
}

// === Sync Commands ===

#[tauri::command]
async fn sync_vault(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    vault_path: String,
) -> Result<SyncStatus, String> {
    let mut ingest_engine_guard = state.ingest_engine.lock().await;
    
    // Create or get ingest engine
    if ingest_engine_guard.is_none() {
        let settings = state.db.get_settings().map_err(|e| e.to_string())?;
        let engine = IngestEngine::new(
            state.db.clone(),
            settings.ollama_endpoint,
            settings.embedding_model,
        );
        *ingest_engine_guard = Some(engine);
    }
    
    let engine = ingest_engine_guard.as_mut().unwrap();
    
    // Run sync
    match engine.sync_vault(&vault_path, &app_handle).await {
        Ok(status) => Ok(status),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn get_sync_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    let ingest_engine_guard = state.ingest_engine.lock().await;
    
    if let Some(engine) = ingest_engine_guard.as_ref() {
        Ok(engine.get_status())
    } else {
        Ok(SyncStatus::default())
    }
}

#[tauri::command]
async fn get_artifacts(state: State<'_, AppState>) -> Result<Vec<Artifact>, String> {
    state.db.get_all_artifacts().map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_artifact(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Delete embeddings first (foreign key constraint)
    state.db.delete_embeddings_by_artifact(&id).map_err(|e| e.to_string())?;
    // Delete the artifact
    state.db.delete_artifact(&id).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    env_logger::init();
    
    tauri::Builder::default()
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app.path_resolver()
                .app_data_dir()
                .expect("Failed to get app data directory");
            
            // Initialize database
            let db = Arc::new(
                Database::new(app_data_dir.clone())
                    .expect("Failed to initialize database")
            );
            
            // Get settings for RAG engine initialization
            let settings = db.get_settings().unwrap_or_default();
            
            // Initialize RAG engine
            let rag_engine = RagEngine::new(
                db.clone(),
                settings.ollama_endpoint,
                settings.ollama_model,
                settings.embedding_model,
            );
            
            // Create app state
            let state = AppState {
                db,
                ingest_engine: Arc::new(TokioMutex::new(None)),
                rag_engine: Arc::new(TokioMutex::new(rag_engine)),
            };
            
            app.manage(state);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_chat_history,
            clear_chat,
            send_message,
            sync_vault,
            get_sync_status,
            get_artifacts,
            delete_artifact,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
