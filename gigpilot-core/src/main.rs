//! GigPilot core HTTP server (Axum)
//!
//! This crate provides the HTTP entrypoint, router and middleware for the GigPilot backend.

mod auth;
mod db;
mod models;
mod sync;

use axum::{routing::{get, post}, Router, http::StatusCode, response::Json};
use std::net::SocketAddr;
use tracing_subscriber;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://gigpilot:gigpilotpass@localhost/gigpilot".to_string());
    let pool = db::create_pool(&database_url).await?;

    // Sync subrouter
    let sync_router = Router::new()
        .route("/pull", post(sync::pull_handler))
        .route("/push", post(sync::push_handler));

    // Basic health route and nest sync router under /sync
    let app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, Json(json!({ "status": "ok" }))) }))
        .nest("/sync", sync_router)
        // apply JWT middleware to protected scope example
        .route_layer(axum::middleware::from_fn(auth::jwt_middleware))
        .layer(axum::extract::Extension(pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    // Use pool to avoid unused variable warning in simple scaffold
    drop(pool);

    Ok(())
}
