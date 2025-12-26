use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Embedding model representing a stored vector embedding.
/// 
/// This struct maps to the `embeddings` table and stores
/// text embeddings for semantic search.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Embedding {
    /// Unique identifier for the embedding
    pub id: Uuid,
    
    /// ID of the user who owns this embedding
    pub user_id: Uuid,
    
    /// Original text content that was embedded
    pub text_content: String,
    
    /// Vector embedding (1536 dimensions for OpenAI ada-002)
    /// Stored as a PostgreSQL vector type
    pub embedding: Vec<f32>,
    
    /// Type of entity this embedding represents
    pub entity_type: String,
    
    /// ID of the related entity (invoice_id, project_id, etc.)
    pub entity_id: Option<Uuid>,
    
    /// Timestamp when the embedding was created
    pub created_at: chrono::DateTime<Utc>,
    
    /// Timestamp when the embedding was last updated
    pub updated_at: chrono::DateTime<Utc>,
}

/// Stores an embedding in the database.
/// 
/// This function:
/// 1. Calls the OpenAI embedding API (mocked) to generate a vector
/// 2. Stores the embedding in the database
/// 
/// # Arguments
/// 
/// * `pool` - PostgreSQL connection pool
/// * `user_id` - ID of the user
/// * `text` - Text content to embed
/// * `entity_type` - Type of entity (e.g., "invoice", "project")
/// * `entity_id` - Optional ID of the related entity
/// 
/// # Returns
/// 
/// Returns the created `Embedding` or an error.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - OpenAI API call fails
/// - Database insertion fails
#[instrument(skip(pool))]
pub async fn store_embedding(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    text: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
) -> Result<Embedding, anyhow::Error> {
    let start_time = std::time::Instant::now();
    
    info!("Generating embedding for text: {}...", &text[..text.len().min(50)]);
    
    // Mock OpenAI embedding API call
    let embedding_vector = generate_embedding_mock(text).await?;
    
    let llm_latency = start_time.elapsed();
    info!("LLM embedding generation took: {:?}", llm_latency);
    
    let db_start = std::time::Instant::now();
    
    // Store embedding in database
    // Convert vector to PostgreSQL vector format: [v1,v2,v3,...]
    let embedding_str = format!("[{}]", embedding_vector.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
    
    // Note: sqlx doesn't have native support for pgvector type
    // We'll use a raw query and manually construct the Embedding
    let embedding_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO embeddings (
            user_id, text_content, embedding, entity_type, entity_id
        ) VALUES (
            $1, $2, $3::vector, $4, $5
        )
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(text)
    .bind(embedding_str)
    .bind(entity_type)
    .bind(entity_id)
    .fetch_one(pool)
    .await?;
    
    // Fetch the created embedding
    let embedding = sqlx::query_as::<_, Embedding>(
        r#"
        SELECT 
            id, user_id, text_content,
            (embedding::text)::float[] as embedding,
            entity_type, entity_id, created_at, updated_at
        FROM embeddings
        WHERE id = $1
        "#,
    )
    .bind(embedding_id)
    .fetch_one(pool)
    .await?;
    
    let db_latency = db_start.elapsed();
    info!("Database insertion took: {:?}", db_latency);
    info!("Total latency - LLM: {:?}, DB: {:?}", llm_latency, db_latency);
    
    Ok(embedding)
}

/// Mock function to generate embeddings using OpenAI API.
/// 
/// In production, this would make an actual HTTP request to OpenAI's
/// embedding API endpoint.
/// 
/// # Arguments
/// 
/// * `text` - Text to embed
/// 
/// # Returns
/// 
/// Returns a 1536-dimensional vector (OpenAI ada-002 format).
async fn generate_embedding_mock(text: &str) -> Result<Vec<f32>, anyhow::Error> {
    // Simulate API call delay
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // In production, this would be:
    // let client = reqwest::Client::new();
    // let response = client
    //     .post("https://api.openai.com/v1/embeddings")
    //     .header("Authorization", format!("Bearer {}", api_key))
    //     .json(&json!({
    //         "model": "text-embedding-ada-002",
    //         "input": text
    //     }))
    //     .send()
    //     .await?;
    // 
    // let data: serde_json::Value = response.json().await?;
    // let embedding = data["data"][0]["embedding"]
    //     .as_array()
    //     .ok_or_else(|| anyhow::anyhow!("Invalid embedding response"))?
    //     .iter()
    //     .map(|v| v.as_f64().unwrap() as f32)
    //     .collect();
    // 
    // Ok(embedding)
    
    // Mock: Generate a deterministic "embedding" based on text hash
    // In reality, this would be a semantic vector from OpenAI
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Generate 1536-dimensional mock vector
    let mut embedding = Vec::with_capacity(1536);
    for i in 0..1536 {
        let value = ((hash as f64 + i as f64) % 1000.0) / 1000.0 - 0.5;
        embedding.push(value as f32);
    }
    
    // Normalize the vector
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut embedding {
            *v /= norm;
        }
    }
    
    Ok(embedding)
}

