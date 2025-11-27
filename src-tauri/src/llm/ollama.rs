use super::{LLMError, LLMProvider, LLMResult, StreamCallback};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
    done: bool,
}

pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }

    fn generate_url(&self) -> String {
        format!("{}/api/generate", self.endpoint)
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn generate(&self, prompt: &str) -> LLMResult<String> {
        let request = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self.client
            .post(&self.generate_url())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(error_text));
        }

        let gen_response: GenerateResponse = response.json().await?;
        Ok(gen_response.response)
    }

    async fn generate_stream(&self, prompt: &str, on_chunk: StreamCallback) -> LLMResult<String> {
        let request = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: true,
        };

        let response = self.client
            .post(&self.generate_url())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(error_text));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LLMError::Stream(e.to_string()))?;
            
            // Append chunk to buffer
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            
            // Process complete JSON lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                
                if line.trim().is_empty() {
                    continue;
                }
                
                if let Ok(gen_response) = serde_json::from_str::<GenerateResponse>(&line) {
                    if !gen_response.response.is_empty() {
                        on_chunk(&gen_response.response);
                        full_response.push_str(&gen_response.response);
                    }
                    
                    if gen_response.done {
                        return Ok(full_response);
                    }
                }
            }
        }

        Ok(full_response)
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
