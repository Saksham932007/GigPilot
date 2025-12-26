use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::Json,
};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::get_current_user_id;
use crate::sync::types::{PullRequest, PullResponse, PushRequest, PushResponse};
use crate::sync::{get_changes, push_changes};

/// Pull sync endpoint handler.
/// 
/// Handles GET requests to `/sync/pull` for retrieving changes
/// from the server after a given timestamp.
pub async fn pull_handler(
    State(state): State<super::super::AppState>,
    request: Request,
    Query(query): Query<PullRequest>,
) -> Result<Json<PullResponse>, StatusCode> {
    let user_id = get_current_user_id(&request)
        .ok_or_else(|| {
            error!("No user ID in request extensions");
            StatusCode::UNAUTHORIZED
        })?;
    
    info!("Pull sync request from user: {}", user_id);
    
    let response = get_changes(&state.db, user_id, query)
        .await
        .map_err(|e| {
            error!("Pull sync failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(response))
}

/// Push sync endpoint handler.
/// 
/// Handles POST requests to `/sync/push` for applying changes
/// from the client to the server.
pub async fn push_handler(
    State(state): State<super::super::AppState>,
    request: Request,
    Json(push_request): Json<PushRequest>,
) -> Result<Json<PushResponse>, StatusCode> {
    let user_id = get_current_user_id(&request)
        .ok_or_else(|| {
            error!("No user ID in request extensions");
            StatusCode::UNAUTHORIZED
        })?;
    
    info!("Push sync request from user: {} with {} changes", user_id, push_request.changes.len());
    
    let response = push_changes(&state.db, user_id, push_request)
        .await
        .map_err(|e| {
            error!("Push sync failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(response))
}

