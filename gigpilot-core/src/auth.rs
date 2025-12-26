use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::Deserialize;
use std::env;
use uuid::Uuid;

/// Container for the authenticated user's id stored in request extensions.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub Uuid);

/// Claims expected inside the JWT for authenticated users.
#[derive(Debug, Deserialize)]
pub struct Claims {
    /// Subject - should be the user's UUID as a string.
    pub sub: String,
    pub exp: usize,
}

/// Middleware to validate a Bearer JWT in the `Authorization` header.
///
/// On success the request is forwarded; on failure a `401` is returned.
pub async fn jwt_middleware<B>(mut req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = req.headers().get("authorization");
    let token = match auth_header.and_then(|v| v.to_str().ok()) {
        Some(s) if s.starts_with("Bearer ") => &s[7..],
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let decoded = match decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::HS256)) {
        Ok(c) => c.claims,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // Parse subject as UUID and attach to request extensions for downstream handlers.
    let user_id = match Uuid::parse_str(&decoded.sub) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    req.extensions_mut().insert(CurrentUser(user_id));

    Ok(next.run(req).await)
}
