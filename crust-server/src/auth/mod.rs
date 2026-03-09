//! Authentication module
//! Handles user registration, login, JWT tokens, and auth middleware

pub mod handlers;
pub mod middleware;
pub mod token;

use serde::{Deserialize, Serialize};

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Register request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// User response (excludes password_hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Auth response (user + token)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
    pub expires_at: String,
}
