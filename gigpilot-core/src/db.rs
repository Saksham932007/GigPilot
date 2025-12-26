use anyhow::Result;
use sqlx::{PgPool, Pool, Postgres};
use std::env;
use tracing::info;

/// Database connection pool helper.
/// 
/// Manages PostgreSQL connection pooling using sqlx for efficient
/// database access across the application.
pub struct Database;

impl Database {
    /// Creates a new PostgreSQL connection pool from the DATABASE_URL environment variable.
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<PgPool>` containing the connection pool or an error
    /// if the connection could not be established.
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The `DATABASE_URL` environment variable is not set
    /// - The database connection cannot be established
    /// - The connection pool cannot be created
    pub async fn new() -> Result<PgPool> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set"))?;

        info!("Connecting to database...");
        
        let pool = PgPool::connect(&database_url).await?;
        
        // Test the connection
        sqlx::query("SELECT 1")
            .execute(&pool)
            .await?;
        
        info!("Database connection established successfully");
        
        Ok(pool)
    }

    /// Gets a reference to the database pool from the application state.
    /// 
    /// This is a convenience method for extracting the pool from Axum's
    /// application state.
    /// 
    /// # Arguments
    /// 
    /// * `pool` - A reference to the PostgreSQL connection pool
    /// 
    /// # Returns
    /// 
    /// Returns a reference to the same pool (for consistency with future extensions)
    pub fn get_pool(pool: &Pool<Postgres>) -> &Pool<Postgres> {
        pool
    }
}

