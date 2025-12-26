use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a Postgres connection pool using `DATABASE_URL` environment variable.
///
/// Returns a `sqlx::PgPool` or an error if the environment is not configured or the pool cannot be created.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}
