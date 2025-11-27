use crate::db::{Database, Embedding};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Database error: {0}")]
    Database(#[from] crate::db::DbError),
    #[error("No embeddings found")]
    NoEmbeddings,
}

pub type VectorResult<T> = Result<T, VectorError>;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub embedding: Embedding,
    pub similarity: f32,
}

pub struct VectorStore {
    db: Arc<Database>,
}

impl VectorStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Search for similar embeddings using cosine similarity
    pub fn search(&self, query_embedding: &[f32], limit: usize) -> VectorResult<Vec<SearchResult>> {
        let embeddings = self.db.get_all_embeddings()?;
        
        if embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<SearchResult> = embeddings
            .into_iter()
            .map(|emb| {
                let similarity = cosine_similarity(query_embedding, &emb.embedding);
                SearchResult {
                    embedding: emb,
                    similarity,
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N results
        results.truncate(limit);

        Ok(results)
    }

    /// Insert a new embedding
    pub fn insert(&self, embedding: &Embedding) -> VectorResult<()> {
        self.db.insert_embedding(embedding)?;
        Ok(())
    }

    /// Delete embeddings for an artifact
    pub fn delete_by_artifact(&self, artifact_id: &str) -> VectorResult<()> {
        self.db.delete_embeddings_by_artifact(artifact_id)?;
        Ok(())
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.0001);
    }
}

