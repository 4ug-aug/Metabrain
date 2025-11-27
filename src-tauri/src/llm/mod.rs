pub mod ollama;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Streaming error: {0}")]
    Stream(String),
}

pub type LLMResult<T> = Result<T, LLMError>;

/// Callback type for streaming chunks
pub type StreamCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Trait defining the interface for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a response for the given prompt
    async fn generate(&self, prompt: &str) -> LLMResult<String>;
    
    /// Generate a streaming response, calling the callback for each chunk
    async fn generate_stream(&self, prompt: &str, on_chunk: StreamCallback) -> LLMResult<String>;
    
    /// Get the model name
    fn model_name(&self) -> &str;
}

/// Factory function to create an LLM provider based on configuration
pub fn create_provider(provider_type: &str, endpoint: &str, model: &str) -> Box<dyn LLMProvider> {
    match provider_type {
        "ollama" | _ => Box::new(ollama::OllamaProvider::new(endpoint.to_string(), model.to_string())),
    }
}
