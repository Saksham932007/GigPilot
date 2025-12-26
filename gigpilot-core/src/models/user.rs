use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User model representing a user in the system.
/// 
/// This struct maps to the `users` table in the database and includes
/// authentication and profile information.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// User's email address (unique)
    pub email: String,
    
    /// Bcrypt hashed password
    #[serde(skip_serializing)]
    pub password_hash: String,
    
    /// User's full name
    pub full_name: Option<String>,
    
    /// Timestamp when the user was created
    pub created_at: DateTime<Utc>,
    
    /// Timestamp when the user was last updated
    pub updated_at: DateTime<Utc>,
    
    /// Timestamp of the user's last login
    pub last_login_at: Option<DateTime<Utc>>,
    
    /// Whether the user account is active
    pub is_active: bool,
}

/// User creation request (without password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub full_name: Option<String>,
}

/// User update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    pub full_name: Option<String>,
    pub is_active: Option<bool>,
}

/// User response (public representation, excludes sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login_at: user.last_login_at,
            is_active: user.is_active,
        }
    }
}

