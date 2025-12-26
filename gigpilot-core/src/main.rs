use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::Json,
    routing::get,
    Router,
};
use dotenv::dotenv;
use gigpilot_core::db::Database;
use sqlx::PgPool;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod auth;
mod db;
mod models;

/// Application state containing shared resources.
/// 
/// This struct holds the database connection pool and other
/// shared state that needs to be accessible to route handlers.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool
    pub db: PgPool,
}

/// Health check endpoint.
/// 
/// Returns a simple JSON response indicating the server is running.
/// Useful for monitoring and load balancer health checks.
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "gigpilot-core",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Database health check endpoint.
/// 
/// Verifies that the database connection is working by executing
/// a simple query.
async fn db_health_check(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database health check failed: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "database": "connected"
    })))
}

/// Creates the main application router.
/// 
/// Sets up all routes and middleware for the GigPilot API.
/// 
/// # Arguments
/// 
/// * `state` - The application state containing database pool
/// 
/// # Returns
/// 
/// Returns a configured Axum Router.
fn create_router(state: AppState) -> Router {
    Router::new()
        // Public routes
        .route("/health", get(health_check))
        .route("/health/db", get(db_health_check))
        
        // Protected routes will be added here
        // .route("/api/invoices", get(list_invoices))
        // .route_layer(middleware::from_fn(auth::auth_middleware))
        
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(LevelFilter::INFO.into());
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
    
    info!("Starting GigPilot Core Server...");
    
    // Initialize database connection pool
    let db_pool = Database::new().await?;
    
    // Create application state
    let app_state = AppState {
        db: db_pool,
    };
    
    // Create router
    let app = create_router(app_state);
    
    // Get server configuration
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .map_err(|_| anyhow::anyhow!("Invalid SERVER_PORT"))?;
    
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}:{}: {}", host, port, e))?;
    
    info!("Server listening on {}:{}", host, port);
    
    // Start the server
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    
    Ok(())
}

