//! Outline Wiki API client for fetching documents.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutlineError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Missing API key")]
    MissingApiKey,
}

pub type OutlineResult<T> = Result<T, OutlineError>;

/// Outline document metadata from documents.list
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineDocument {
    pub id: String,
    pub title: String,
    pub url_id: String,
    #[serde(default)]
    pub text: String,
    pub updated_at: String,
    #[serde(default)]
    pub archived_at: Option<String>,
}

/// Response wrapper for Outline API
#[derive(Debug, Deserialize)]
pub struct OutlineListResponse {
    pub data: Vec<OutlineDocument>,
    pub pagination: Option<OutlinePagination>,
}

#[derive(Debug, Deserialize)]
pub struct OutlineDocumentResponse {
    pub data: OutlineDocument,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlinePagination {
    pub offset: usize,
    pub limit: usize,
}

/// Request body for documents.list
#[derive(Debug, Serialize)]
pub struct ListDocumentsRequest {
    pub offset: usize,
    pub limit: usize,
}

/// Request body for documents.info
#[derive(Debug, Serialize)]
pub struct GetDocumentRequest {
    pub id: String,
}

/// Outline API client
pub struct OutlineClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl OutlineClient {
    /// Create a new Outline client
    pub fn new(base_url: String, api_key: String) -> OutlineResult<Self> {
        if api_key.is_empty() {
            return Err(OutlineError::MissingApiKey);
        }

        Ok(Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// List all documents with pagination
    pub async fn list_documents(&self, offset: usize, limit: usize) -> OutlineResult<OutlineListResponse> {
        let url = format!("{}/documents.list", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&ListDocumentsRequest { offset, limit })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OutlineError::Api(format!("HTTP {}: {}", status, error_text)));
        }

        let result: OutlineListResponse = response.json().await?;
        Ok(result)
    }

    /// Fetch all documents (handles pagination automatically)
    pub async fn list_all_documents(&self) -> OutlineResult<Vec<OutlineDocument>> {
        let mut all_documents = Vec::new();
        let mut offset = 0;
        let limit = 100; // Max per page

        loop {
            let response = self.list_documents(offset, limit).await?;
            let count = response.data.len();
            all_documents.extend(response.data);

            if count < limit {
                break;
            }
            offset += limit;
        }

        // Filter out archived documents
        all_documents.retain(|doc| doc.archived_at.is_none());

        Ok(all_documents)
    }

    /// Get a single document with full content
    pub async fn get_document(&self, id: &str) -> OutlineResult<OutlineDocument> {
        let url = format!("{}/documents.info", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&GetDocumentRequest { id: id.to_string() })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OutlineError::Api(format!("HTTP {}: {}", status, error_text)));
        }

        let result: OutlineDocumentResponse = response.json().await?;
        Ok(result.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_requires_api_key() {
        let result = OutlineClient::new("https://example.com".to_string(), "".to_string());
        assert!(matches!(result, Err(OutlineError::MissingApiKey)));
    }

    #[test]
    fn test_client_creation() {
        let result = OutlineClient::new(
            "https://app.getoutline.com/api".to_string(),
            "test_key".to_string(),
        );
        assert!(result.is_ok());
    }
}

