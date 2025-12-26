use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::rag::embeddings::{generate_embedding_mock, Embedding};

/// Search for similar projects/invoices using vector similarity.
/// 
/// This function:
/// 1. Generates an embedding for the query text
/// 2. Searches for similar embeddings using cosine similarity
/// 3. Returns the most similar results
/// 
/// # Arguments
/// 
/// * `pool` - PostgreSQL connection pool
/// * `user_id` - ID of the user
/// * `query` - Search query text
/// * `limit` - Maximum number of results to return
/// 
/// # Returns
/// 
/// Returns a vector of `Embedding` results sorted by similarity.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Embedding generation fails
/// - Database query fails
#[instrument(skip(pool))]
pub async fn search_similar_projects(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    query: &str,
    limit: Option<i64>,
) -> Result<Vec<(Embedding, f32)>, anyhow::Error> {
    let start_time = std::time::Instant::now();
    
    info!("Searching for similar projects with query: {}", query);
    
    // Generate embedding for query
    let query_embedding = generate_embedding_mock(query).await?;
    
    let llm_latency = start_time.elapsed();
    info!("LLM embedding generation took: {:?}", llm_latency);
    
    let db_start = std::time::Instant::now();
    
    // Convert embedding vector to PostgreSQL vector format
    let embedding_str = format!("[{}]", query_embedding.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
    
    // Search using cosine similarity
    let limit = limit.unwrap_or(10);
    
    let results = sqlx::query_as::<_, (Embedding, f32)>(
        r#"
        SELECT 
            id, user_id, text_content,
            embedding::text::float[] as embedding,
            entity_type, entity_id, created_at, updated_at,
            1 - (embedding <=> $2::vector) as similarity
        FROM embeddings
        WHERE user_id = $1
            AND entity_type IN ('invoice', 'project')
        ORDER BY embedding <=> $2::vector
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(embedding_str)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let db_latency = db_start.elapsed();
    info!("Database similarity search took: {:?}", db_latency);
    info!("Total latency - LLM: {:?}, DB: {:?}", llm_latency, db_latency);
    
    info!("Found {} similar results", results.len());
    
    Ok(results)
}

