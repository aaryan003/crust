//! JWT validation middleware

use std::sync::Arc;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::AppState;
use super::token::{validate_token, TokenClaims};

/// Extractor for authenticated requests
pub struct RequireAuth {
    pub claims: TokenClaims,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for RequireAuth
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingHeader)?;

        // Parse Bearer token
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken);
        }

        let token = &auth_header[7..]; // Skip "Bearer "

        // Get JWT secret from environment
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default-jwt-secret-change-in-production".to_string());

        // Validate token signature + expiry
        let claims = validate_token(token, &secret).map_err(|_| AuthError::InvalidToken)?;

        // Check revoked_tokens table — reject if JTI was invalidated by logout
        let jti = claims.jti.clone();
        let revoked = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM revoked_tokens WHERE token_jti = $1",
            jti
        )
        .fetch_one(&state.db.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        if revoked > 0 {
            return Err(AuthError::InvalidToken);
        }

        Ok(RequireAuth { claims })
    }
}

/// Auth error types
#[derive(Debug)]
pub enum AuthError {
    MissingHeader,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            AuthError::MissingHeader => (
                StatusCode::UNAUTHORIZED,
                "AUTH_MISSING_HEADER",
                "Authorization header missing",
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "AUTH_TOKEN_INVALID",
                "Token is invalid or malformed",
            ),
        };

        let response = json!({
            "success": false,
            "data": null,
            "error": {
                "code": code,
                "message": message,
            },
            "metadata": {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "duration": 0,
            }
        });

        (status, Json(response)).into_response()
    }
}
