use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::middleware::RequireAuth, AppState};

pub mod objects;
pub mod orgs;
pub mod prs;
pub mod teams;

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub default_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Create repository request
#[derive(Debug, Deserialize)]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub default_branch: Option<String>,
}

/// Update repository request
#[derive(Debug, Deserialize)]
pub struct UpdateRepositoryRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub default_branch: Option<String>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    pub timestamp: String,
    pub duration: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata {
                timestamp: now,
                duration: 0,
                request_id: None,
            },
        }
    }
}

impl ApiResponse<()> {
    pub fn error(code: &str, message: &str, status_code: StatusCode) -> (StatusCode, Json<Self>) {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        (
            status_code,
            Json(Self {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: code.to_string(),
                    message: message.to_string(),
                    field: None,
                }),
                metadata: ResponseMetadata {
                    timestamp: now,
                    duration: 0,
                    request_id: None,
                },
            }),
        )
    }

    pub fn error_with_field(
        code: &str,
        message: &str,
        field: &str,
        status_code: StatusCode,
    ) -> (StatusCode, Json<Self>) {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        (
            status_code,
            Json(Self {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: code.to_string(),
                    message: message.to_string(),
                    field: Some(field.to_string()),
                }),
                metadata: ResponseMetadata {
                    timestamp: now,
                    duration: 0,
                    request_id: None,
                },
            }),
        )
    }
}

/// POST /api/v1/repos — Create a new repository
pub async fn create_repository(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Json(req): Json<CreateRepositoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Repository>>), (StatusCode, Json<ApiResponse<()>>)> {
    // Validate required fields
    if req.name.is_empty() || req.display_name.is_empty() {
        return Err(ApiResponse::error_with_field(
            "VALIDATE_REQUIRED_FIELD",
            "Repository name is required",
            "name",
            StatusCode::BAD_REQUEST,
        ));
    }

    // Validate repository name (alphanumeric, dash, underscore, 3-64 chars)
    if !is_valid_repo_name(&req.name) {
        return Err(ApiResponse::error_with_field(
            "REPO_NAME_INVALID",
            "Repository name must be 3-64 characters, containing only alphanumeric characters, dashes, and underscores",
            "name",
            StatusCode::BAD_REQUEST,
        ));
    }

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        ApiResponse::error(
            "SERVER_INTERNAL_ERROR",
            "Invalid user ID in token",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    let is_public = req.is_public.unwrap_or(false);
    let default_branch = req.default_branch.clone().unwrap_or_else(|| "main".to_string());

    let row = sqlx::query!(
        r#"INSERT INTO repositories (owner_id, name, display_name, description, is_public, default_branch)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, owner_id, name, display_name, description, is_public, default_branch, created_at, updated_at"#,
        user_id,
        req.name,
        req.display_name,
        req.description,
        is_public,
        default_branch
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiResponse::error("REPO_ALREADY_EXISTS", "Repository name already exists for this user", StatusCode::CONFLICT)
        } else {
            tracing::error!(error = %e, "Failed to create repository");
            ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create repository", StatusCode::INTERNAL_SERVER_ERROR)
        }
    })?;

    let repo = Repository {
        id: row.id,
        owner_id: row.owner_id,
        name: row.name,
        display_name: row.display_name,
        description: row.description,
        is_public: row.is_public,
        default_branch: row.default_branch,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(repo))))
}

/// GET /api/v1/repos/:owner/:repo — Get repository metadata
pub async fn get_repository(
    State(state): State<Arc<AppState>>,
    auth: Option<RequireAuth>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<(StatusCode, Json<ApiResponse<Repository>>), (StatusCode, Json<ApiResponse<()>>)> {
    let row = sqlx::query!(
        r#"SELECT r.id, r.owner_id, r.name, r.display_name, r.description,
                  r.is_public, r.default_branch, r.created_at, r.updated_at
           FROM repositories r
           JOIN users u ON r.owner_id = u.id
           WHERE u.username = $1 AND r.name = $2"#,
        owner,
        repo
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch repository");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to fetch repository", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    match row {
        Some(r) => {
            // Private repos require authentication
            if !r.is_public && auth.is_none() {
                return Err(ApiResponse::error(
                    "AUTH_REQUIRED",
                    "Authentication required to access this repository",
                    StatusCode::UNAUTHORIZED,
                ));
            }
            Ok((StatusCode::OK, Json(ApiResponse::success(Repository {
                id: r.id,
                owner_id: r.owner_id,
                name: r.name,
                display_name: r.display_name,
                description: r.description,
                is_public: r.is_public,
                default_branch: r.default_branch,
                created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                updated_at: r.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            }))))
        }
        None => Err(ApiResponse::error("REPO_NOT_FOUND", "Repository not found", StatusCode::NOT_FOUND)),
    }
}

/// PATCH /api/v1/repos/:owner/:repo — Update repository (owner only)
pub async fn update_repository(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo)): Path<(String, String)>,
    Json(req): Json<UpdateRepositoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Repository>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID in token", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    // Verify ownership and fetch repo
    let existing = sqlx::query!(
        r#"SELECT r.id FROM repositories r
           JOIN users u ON r.owner_id = u.id
           WHERE u.username = $1 AND r.name = $2 AND r.owner_id = $3"#,
        owner, repo, user_id
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch repository for update");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to fetch repository", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let repo_id = match existing {
        Some(r) => r.id,
        None => return Err(ApiResponse::error("REPO_NOT_FOUND", "Repository not found", StatusCode::NOT_FOUND)),
    };

    let row = sqlx::query!(
        r#"UPDATE repositories
           SET display_name = COALESCE($1, display_name),
               description  = COALESCE($2, description),
               is_public    = COALESCE($3, is_public),
               default_branch = COALESCE($4, default_branch),
               updated_at   = NOW()
           WHERE id = $5
           RETURNING id, owner_id, name, display_name, description, is_public, default_branch, created_at, updated_at"#,
        req.display_name,
        req.description,
        req.is_public,
        req.default_branch,
        repo_id
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to update repository");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to update repository", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::OK, Json(ApiResponse::success(Repository {
        id: row.id,
        owner_id: row.owner_id,
        name: row.name,
        display_name: row.display_name,
        description: row.description,
        is_public: row.is_public,
        default_branch: row.default_branch,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// DELETE /api/v1/repos/:owner/:repo — Delete repository (owner only)
pub async fn delete_repository(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID in token", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let result = sqlx::query!(
        r#"DELETE FROM repositories r
           USING users u
           WHERE r.owner_id = u.id AND u.username = $1 AND r.name = $2 AND r.owner_id = $3"#,
        owner, repo, user_id
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to delete repository");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to delete repository", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiResponse::error("REPO_NOT_FOUND", "Repository not found", StatusCode::NOT_FOUND));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Helper function to validate repository name
/// Valid: alphanumeric, dash, underscore, 3-64 chars, lowercase
pub fn is_valid_repo_name(name: &str) -> bool {
    if name.len() < 3 || name.len() > 64 {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_repo_names() {
        assert!(is_valid_repo_name("my-repo"));
        assert!(is_valid_repo_name("my_repo"));
        assert!(is_valid_repo_name("my-repo-123"));
        assert!(is_valid_repo_name("abc"));
    }

    #[test]
    fn test_invalid_repo_names() {
        assert!(!is_valid_repo_name("ab")); // too short
        assert!(!is_valid_repo_name("My-Repo")); // uppercase
        assert!(!is_valid_repo_name("my repo")); // space
        assert!(!is_valid_repo_name("my.repo")); // dot
        assert!(!is_valid_repo_name("a".repeat(65).as_str())); // too long
    }
}
