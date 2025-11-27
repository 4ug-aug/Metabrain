use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Ollama error: {0}")]
    Ollama(String),
}

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

pub struct EmbeddingClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl EmbeddingClient {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }

    pub async fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.endpoint);
        
        let request = EmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Ollama(error_text));
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        Ok(embedding_response.embedding)
    }

    pub async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        
        for text in texts {
            let embedding = self.embed(text).await?;
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }
}

