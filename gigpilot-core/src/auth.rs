use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, warn};
use uuid::Uuid;

/// JWT claims structure for authentication tokens.
/// 
/// Contains the user ID and expiration timestamp for JWT validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID from the database
    pub sub: String, // Subject (user ID)
    
    /// Expiration timestamp (Unix timestamp)
    pub exp: usize,
    
    /// Issued at timestamp
    pub iat: usize,
}

/// Authentication error types.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("Token expired")]
    ExpiredToken,
    
    #[error("Missing authorization header")]
    MissingHeader,
    
    #[error("Invalid authorization format")]
    InvalidFormat,
    
    #[error("JWT secret not configured")]
    MissingSecret,
}

/// JWT authentication middleware.
/// 
/// Validates JWT tokens from the Authorization header and extracts
/// user information for downstream handlers.
pub struct Auth;

impl Auth {
    /// Creates a JWT token for a user.
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - The UUID of the user
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<String, AuthError>` containing the JWT token
    /// or an error if token creation fails.
    /// 
    /// # Errors
    /// 
    /// Returns `AuthError::MissingSecret` if JWT_SECRET is not configured.
    pub fn create_token(user_id: Uuid) -> Result<String, AuthError> {
        let secret = env::var("JWT_SECRET")
            .map_err(|_| AuthError::MissingSecret)?;
        
        let expiration_hours: usize = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .unwrap_or(24);
        
        let now = chrono::Utc::now();
        let exp = (now + chrono::Duration::hours(expiration_hours as i64))
            .timestamp() as usize;
        let iat = now.timestamp() as usize;
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp,
            iat,
        };
        
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        
        encode(&Header::default(), &claims, &encoding_key)
            .map_err(|_| AuthError::InvalidToken)
    }

    /// Validates a JWT token and extracts the user ID.
    /// 
    /// # Arguments
    /// 
    /// * `token` - The JWT token string
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<Uuid, AuthError>` containing the user ID
    /// or an error if validation fails.
    /// 
    /// # Errors
    /// 
    /// Returns various `AuthError` variants for different failure scenarios.
    pub fn validate_token(token: &str) -> Result<Uuid, AuthError> {
        let secret = env::var("JWT_SECRET")
            .map_err(|_| AuthError::MissingSecret)?;
        
        let decoding_key = DecodingKey::from_secret(secret.as_ref());
        let validation = Validation::default();
        
        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AuthError::ExpiredToken
                    }
                    _ => {
                        warn!("Token validation failed: {}", e);
                        AuthError::InvalidToken
                    }
                }
            })?;
        
        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| AuthError::InvalidToken)?;
        
        Ok(user_id)
    }

    /// Extracts the bearer token from the Authorization header.
    /// 
    /// # Arguments
    /// 
    /// * `auth_header` - The Authorization header value
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<String, AuthError>` containing the token
    /// or an error if extraction fails.
    fn extract_bearer_token(auth_header: &str) -> Result<String, AuthError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidFormat);
        }
        
        let token = auth_header[7..].trim();
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }
        
        Ok(token.to_string())
    }
}

/// Axum middleware for JWT authentication.
/// 
/// This middleware validates JWT tokens from the Authorization header
/// and adds the user ID to request extensions for downstream handlers.
/// 
/// # Usage
/// 
/// Add this middleware to routes that require authentication:
/// ```rust
/// .route_layer(middleware::from_fn(auth_middleware))
/// ```
pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("Missing authorization header");
            StatusCode::UNAUTHORIZED
        })?;
    
    let token = Auth::extract_bearer_token(auth_header)
        .map_err(|e| {
            error!("Failed to extract token: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;
    
    let user_id = Auth::validate_token(&token)
        .map_err(|e| {
            error!("Token validation failed: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;
    
    // Add user_id to request extensions for downstream handlers
    request.extensions_mut().insert(user_id);
    
    Ok(next.run(request).await)
}

/// Extracts the authenticated user ID from request extensions.
/// 
/// This is a convenience function for handlers to get the user ID
/// that was set by the auth middleware.
/// 
/// # Arguments
/// 
/// * `request` - The Axum request
/// 
/// # Returns
/// 
/// Returns `Some(Uuid)` if user is authenticated, `None` otherwise.
pub fn get_current_user_id(request: &Request) -> Option<Uuid> {
    request.extensions().get::<Uuid>().copied()
}

