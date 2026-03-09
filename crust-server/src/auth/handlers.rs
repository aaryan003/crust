//! Auth endpoint handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

use uuid::Uuid;

use super::{middleware::RequireAuth, token::generate_token, LoginRequest, RegisterRequest};
use crate::AppState;

/// Helper to hash password with argon2
fn hash_password(password: &str) -> Result<String, String> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};

    let salt = SaltString::generate(rand::thread_rng());
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Password hashing failed: {}", e))
}

/// Helper to verify password
#[allow(dead_code)]
fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| format!("Invalid password hash: {}", e))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// POST /api/v1/auth/register
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    // Validate inputs
    if req.username.is_empty() || req.email.is_empty() || req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "data": null,
                "error": {
                    "code": "VALIDATE_REQUIRED_FIELD",
                    "message": "Required field missing",
                },
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            })),
        )
            .into_response();
    }

    // Validate email format (simple check)
    if !req.email.contains('@') || !req.email.contains('.') {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "data": null,
                "error": {
                    "code": "VALIDATE_INVALID_EMAIL",
                    "message": "Email format invalid",
                },
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            })),
        )
            .into_response();
    }

    // Validate password strength (min 12 chars)
    if req.password.len() < 12 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "data": null,
                "error": {
                    "code": "VALIDATE_WEAK_PASSWORD",
                    "message": "Password does not meet requirements",
                },
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            })),
        )
            .into_response();
    }

    // Check if registration is allowed
    let allow_registration = std::env::var("ALLOW_REGISTRATION")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !allow_registration {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "data": null,
                "error": {
                    "code": "AUTH_REGISTRATION_DISABLED",
                    "message": "Registration is disabled",
                },
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            })),
        )
            .into_response();
    }

    // Hash password
    let password_hash = match hash_password(&req.password) {
        Ok(hash) => hash,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": {
                        "code": "INTERNAL_ERROR",
                        "message": "Internal server error",
                    },
                    "metadata": {
                        "timestamp": Utc::now().to_rfc3339(),
                        "duration": 0,
                    }
                })),
            )
                .into_response()
        }
    };

    let display_name = if req.display_name.as_deref().unwrap_or("").is_empty() {
        req.username.clone()
    } else {
        req.display_name.clone().unwrap()
    };

    // Insert user into database
    let row = sqlx::query!(
        r#"
        INSERT INTO users (username, email, password_hash, display_name)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, display_name, created_at, updated_at
        "#,
        req.username,
        req.email,
        password_hash,
        display_name
    )
    .fetch_one(&state.db.pool)
    .await;

    let row = match row {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err)) if db_err.constraint().is_some() => {
            let constraint = db_err.constraint().unwrap_or("");
            let (code, message) = if constraint.contains("username") {
                ("AUTH_USERNAME_TAKEN", "Username already taken")
            } else {
                ("AUTH_EMAIL_TAKEN", "Email already registered")
            };
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": code, "message": message },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Register DB error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "INTERNAL_ERROR", "message": "Internal server error" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
    };

    let user_id = row.id.to_string();
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-jwt-secret-change-in-production".to_string());
    let jwt_expiry = std::env::var("JWT_EXPIRY_SECONDS")
        .unwrap_or_else(|_| "86400".to_string())
        .parse::<i64>()
        .unwrap_or(86400);

    let (token, _jti) = match generate_token(&user_id, &row.username, &jwt_secret, jwt_expiry) {
        Ok((t, j)) => (t, j),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "INTERNAL_ERROR", "message": "Internal server error" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response()
        }
    };

    let expires_at = (Utc::now() + chrono::Duration::seconds(jwt_expiry)).to_rfc3339();

    let response = json!({
        "success": true,
        "data": {
            "user": {
                "id": user_id,
                "username": row.username,
                "email": row.email,
                "display_name": row.display_name,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            },
            "token": token,
            "expires_at": expires_at,
        },
        "error": null,
        "metadata": {
            "timestamp": Utc::now().to_rfc3339(),
            "duration": 0,
        }
    });

    (StatusCode::CREATED, Json(response)).into_response()
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // Validate inputs
    if req.username.is_empty() || req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "data": null,
                "error": {
                    "code": "VALIDATE_REQUIRED_FIELD",
                    "message": "Required field missing",
                },
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            })),
        )
            .into_response();
    }

    // Fetch user from database
    let row = sqlx::query!(
        "SELECT id, username, email, display_name, password_hash, created_at, updated_at FROM users WHERE username = $1 AND is_active = true",
        req.username
    )
    .fetch_optional(&state.db.pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "AUTH_INVALID_CREDENTIALS", "message": "Invalid username or password" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Login DB error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "INTERNAL_ERROR", "message": "Internal server error" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
    };

    // Verify password
    match verify_password(&req.password, &row.password_hash) {
        Ok(true) => {}
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "AUTH_INVALID_CREDENTIALS", "message": "Invalid username or password" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
    }

    let user_id = row.id.to_string();
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-jwt-secret-change-in-production".to_string());
    let jwt_expiry = std::env::var("JWT_EXPIRY_SECONDS")
        .unwrap_or_else(|_| "86400".to_string())
        .parse::<i64>()
        .unwrap_or(86400);

    let (token, _jti) = match generate_token(&user_id, &row.username, &jwt_secret, jwt_expiry) {
        Ok((t, j)) => (t, j),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "INTERNAL_ERROR", "message": "Internal server error" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response()
        }
    };

    let expires_at = (Utc::now() + chrono::Duration::seconds(jwt_expiry)).to_rfc3339();

    let response = json!({
        "success": true,
        "data": {
            "user": {
                "id": user_id,
                "username": row.username,
                "email": row.email,
                "display_name": row.display_name,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            },
            "token": token,
            "expires_at": expires_at,
        },
        "error": null,
        "metadata": {
            "timestamp": Utc::now().to_rfc3339(),
            "duration": 0,
        }
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// POST /api/v1/auth/logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
) -> impl IntoResponse {
    let jti = claims.jti.clone();
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return StatusCode::NO_CONTENT.into_response(),
    };
    let exp = chrono::DateTime::from_timestamp(claims.exp, 0)
        .unwrap_or_else(chrono::Utc::now);

    // Insert JTI into revoked_tokens so subsequent requests are rejected
    if let Err(e) = sqlx::query!(
        r#"INSERT INTO revoked_tokens (token_jti, user_id, expires_at)
           VALUES ($1, $2, $3)
           ON CONFLICT (token_jti) DO NOTHING"#,
        jti,
        user_id,
        exp
    )
    .execute(&state.db.pool)
    .await
    {
        tracing::error!(error = %e, "Failed to revoke token");
    }

    tracing::info!("User {} logged out, token {} revoked", claims.username, jti);
    StatusCode::NO_CONTENT.into_response()
}

/// GET /api/v1/auth/me
pub async fn me(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
) -> impl IntoResponse {
    let user_id: uuid::Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "data": null,
                    "error": { "code": "AUTH_INVALID_TOKEN", "message": "Invalid token" },
                    "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
                })),
            )
                .into_response();
        }
    };

    let row = sqlx::query!(
        "SELECT id, username, email, display_name, created_at, updated_at FROM users WHERE id = $1 AND is_active = true",
        user_id
    )
    .fetch_optional(&state.db.pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let response = json!({
                "success": true,
                "data": {
                    "id": r.id.to_string(),
                    "username": r.username,
                    "email": r.email,
                    "display_name": r.display_name,
                    "created_at": r.created_at,
                    "updated_at": r.updated_at,
                },
                "error": null,
                "metadata": {
                    "timestamp": Utc::now().to_rfc3339(),
                    "duration": 0,
                }
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "data": null,
                "error": { "code": "AUTH_USER_NOT_FOUND", "message": "User not found" },
                "metadata": { "timestamp": Utc::now().to_rfc3339(), "duration": 0 }
            })),
        )
            .into_response(),
    }
}
