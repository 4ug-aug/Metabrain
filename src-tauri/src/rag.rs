use crate::db::{ChatMessage, Database};
use crate::embedding::EmbeddingClient;
use crate::llm::{create_provider, LLMProvider};
use crate::vector::{SearchResult, VectorStore};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::Manager;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RagError {
    #[error("Embedding error: {0}")]
    Embedding(#[from] crate::embedding::EmbeddingError),
    #[error("Vector error: {0}")]
    Vector(#[from] crate::vector::VectorError),
    #[error("LLM error: {0}")]
    Llm(#[from] crate::llm::LLMError),
    #[error("No context found")]
    NoContext,
}

pub type RagResult<T> = Result<T, RagError>;

const SYSTEM_PROMPT: &str = r#"You are Metamind, a helpful AI assistant that answers questions based on the user's personal knowledge base.

Use MAINLY the provided context to answer questions. If the context doesn't contain relevant information, say so clearly but attempt to answer the user's question.

When citing information, mention which note it comes from if possible.

Be concise but thorough in your answers."#;

const QUERY_EXPANSION_PROMPT: &str = r#"Given the following conversation and the latest user query, generate 2-3 alternative search queries that would help find relevant information in a knowledge base. The queries should:
1. Capture the core intent of the question
2. Include relevant synonyms or related terms
3. Consider context from the conversation

Return ONLY the queries, one per line, without numbering or explanations.

Conversation:
{conversation}

Latest Query: {query}

Alternative search queries:"#;

const MAX_CONTEXT_CHUNKS: usize = 5;
const MIN_SIMILARITY_THRESHOLD: f32 = 0.25;
const MAX_CHAT_HISTORY: usize = 10;

pub struct RagEngine {
    vector_store: VectorStore,
    embedding_client: EmbeddingClient,
    llm_provider: Box<dyn LLMProvider>,
}

impl RagEngine {
    pub fn new(
        db: Arc<Database>,
        ollama_endpoint: String,
        llm_model: String,
        embedding_model: String,
    ) -> Self {
        Self {
            vector_store: VectorStore::new(db),
            embedding_client: EmbeddingClient::new(ollama_endpoint.clone(), embedding_model),
            llm_provider: create_provider("ollama", &ollama_endpoint, &llm_model),
        }
    }

    pub fn update_settings(
        &mut self,
        db: Arc<Database>,
        ollama_endpoint: String,
        llm_model: String,
        embedding_model: String,
    ) {
        self.vector_store = VectorStore::new(db);
        self.embedding_client = EmbeddingClient::new(ollama_endpoint.clone(), embedding_model);
        self.llm_provider = create_provider("ollama", &ollama_endpoint, &llm_model);
    }

    /// Main query method with chat context and query expansion
    pub async fn query(
        &self,
        query: &str,
        chat_history: &[ChatMessage],
        app_handle: &tauri::AppHandle,
    ) -> RagResult<String> {
        log::info!("Processing query: {}", query);

        // 1. Expand the query using chat context
        let expanded_queries = self.expand_query(query, chat_history).await?;
        log::info!("Expanded queries: {:?}", expanded_queries);

        // 2. Search with all queries and deduplicate results
        let mut all_results: Vec<SearchResult> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();

        for search_query in &expanded_queries {
            let query_embedding = self.embedding_client.embed(search_query).await?;
            let results = self.vector_store.search(&query_embedding, MAX_CONTEXT_CHUNKS)?;
            
            for result in results {
                if !seen_ids.contains(&result.embedding.id) {
                    seen_ids.insert(result.embedding.id.clone());
                    all_results.push(result);
                }
            }
        }

        // Sort all results by similarity and take top N
        all_results.sort_by(|a, b| {
            b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(MAX_CONTEXT_CHUNKS);

        // Filter by similarity threshold
        let relevant_results: Vec<&SearchResult> = all_results
            .iter()
            .filter(|r| r.similarity >= MIN_SIMILARITY_THRESHOLD)
            .collect();

        log::info!("Found {} relevant chunks", relevant_results.len());

        // 3. Build context from search results
        let kb_context = self.build_context(&relevant_results);

        // 4. Build the full prompt with chat history
        let prompt = self.build_prompt_with_history(query, &kb_context, chat_history);

        // 5. Stream response from LLM
        let app_handle_clone = app_handle.clone();
        let response = self.llm_provider.generate_stream(
            &prompt,
            Box::new(move |chunk| {
                let _ = app_handle_clone.emit_all("stream-chunk", serde_json::json!({
                    "content": chunk,
                    "done": false
                }));
            })
        ).await?;

        // Emit completion
        let _ = app_handle.emit_all("stream-chunk", serde_json::json!({
            "content": "",
            "done": true
        }));

        Ok(response)
    }

    /// Expand the query using the LLM to generate alternative search queries
    async fn expand_query(
        &self,
        query: &str,
        chat_history: &[ChatMessage],
    ) -> RagResult<Vec<String>> {
        // Always include the original query
        let mut queries = vec![query.to_string()];

        // Build conversation context (last few messages)
        let recent_history: Vec<&ChatMessage> = chat_history
            .iter()
            .rev()
            .take(MAX_CHAT_HISTORY)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        // Format conversation for the prompt
        let conversation = if recent_history.is_empty() {
            "No previous conversation.".to_string()
        } else {
            recent_history
                .iter()
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Generate expanded queries
        let expansion_prompt = QUERY_EXPANSION_PROMPT
            .replace("{conversation}", &conversation)
            .replace("{query}", query);

        match self.llm_provider.generate(&expansion_prompt).await {
            Ok(response) => {
                // Parse the response - each line is a query
                for line in response.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() 
                        && trimmed.len() > 3 
                        && trimmed.len() < 500
                        && !trimmed.starts_with('-')
                        && !trimmed.starts_with('*')
                    {
                        // Remove any numbering like "1." or "1)"
                        let cleaned = trimmed
                            .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ' ');
                        if !cleaned.is_empty() {
                            queries.push(cleaned.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Query expansion failed, using original query only: {}", e);
            }
        }

        // Limit total queries to avoid too many API calls
        queries.truncate(4);
        
        Ok(queries)
    }

    fn build_context(&self, results: &[&SearchResult]) -> String {
        if results.is_empty() {
            return "No relevant context found in your knowledge base.".to_string();
        }

        let mut context_parts: Vec<String> = Vec::new();

        for (i, result) in results.iter().enumerate() {
            let source = &result.embedding.artifact_id;
            let content = &result.embedding.content;
            let similarity = result.similarity;

            context_parts.push(format!(
                "[Source {}: {} (relevance: {:.0}%)]\n{}",
                i + 1,
                source,
                similarity * 100.0,
                content
            ));
        }

        context_parts.join("\n\n---\n\n")
    }

    fn build_prompt_with_history(
        &self,
        query: &str,
        kb_context: &str,
        chat_history: &[ChatMessage],
    ) -> String {
        // Include recent chat history for context
        let recent_history: Vec<&ChatMessage> = chat_history
            .iter()
            .rev()
            .take(MAX_CHAT_HISTORY)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let chat_context = if recent_history.is_empty() {
            String::new()
        } else {
            let history_str = recent_history
                .iter()
                .map(|m| {
                    let role_label = if m.role == "user" { "User" } else { "Assistant" };
                    format!("{}: {}", role_label, m.content)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            
            format!("\n\n## Previous Conversation:\n\n{}", history_str)
        };

        format!(
            "{}\n\n## Context from your knowledge base:\n\n{}{}

## Current User Question:

{}

## Your Answer:",
            SYSTEM_PROMPT,
            kb_context,
            chat_context,
            query
        )
    }
}
