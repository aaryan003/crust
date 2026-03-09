//! Organization endpoints
//!
//! Endpoints for managing organizations and organization members

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::middleware::RequireAuth, routes::ApiResponse, AppState};

/// Organization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

/// Organization member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: String,
}

/// Create organization request
#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
}

/// POST /api/v1/orgs — Create organization
pub async fn create_organization(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Organization>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    if req.name.len() < 3 || req.name.len() > 64 {
        return Err(ApiResponse::error("ORG_NAME_INVALID", "Organization name must be 3-64 characters", StatusCode::BAD_REQUEST));
    }
    if !req.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(ApiResponse::error("ORG_NAME_INVALID", "Organization name can only contain alphanumeric characters, dashes, and underscores", StatusCode::BAD_REQUEST));
    }

    // Insert org + add creator as owner (in a transaction)
    let mut tx = state.db.pool.begin().await
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to start transaction", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org_row = sqlx::query!(
        r#"INSERT INTO organizations (name, display_name, description, owner_id)
           VALUES ($1, $2, $3, $4)
           RETURNING id, name, display_name, description, owner_id, created_at, updated_at"#,
        req.name, req.display_name, req.description, user_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiResponse::error("ORG_ALREADY_EXISTS", "Organization name already taken", StatusCode::CONFLICT)
        } else {
            tracing::error!(error = %e, "Failed to create org");
            ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create organization", StatusCode::INTERNAL_SERVER_ERROR)
        }
    })?;

    // Add creator as owner member
    sqlx::query!(
        "INSERT INTO org_members (org_id, user_id, role) VALUES ($1, $2, 'owner')",
        org_row.id, user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to add org owner member");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to add owner to organization", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    tx.commit().await
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Transaction commit failed", StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(Organization {
        id: org_row.id,
        name: org_row.name,
        display_name: org_row.display_name,
        description: org_row.description,
        owner_id: org_row.owner_id,
        created_at: org_row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: org_row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// GET /api/v1/orgs/:org — Get organization
pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    Path(org_name): Path<String>,
) -> Result<(StatusCode, Json<ApiResponse<Organization>>), (StatusCode, Json<ApiResponse<()>>)> {
    let row = sqlx::query!(
        "SELECT id, name, display_name, description, owner_id, created_at, updated_at FROM organizations WHERE name = $1",
        org_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch org");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    match row {
        Some(r) => Ok((StatusCode::OK, Json(ApiResponse::success(Organization {
            id: r.id,
            name: r.name,
            display_name: r.display_name,
            description: r.description,
            owner_id: r.owner_id,
            created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            updated_at: r.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        })))),
        None => Err(ApiResponse::error("ORG_NOT_FOUND", "Organization not found", StatusCode::NOT_FOUND)),
    }
}

/// GET /api/v1/orgs/:org/members — List organization members
pub async fn list_organization_members(
    State(state): State<Arc<AppState>>,
    Path(org_name): Path<String>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<OrganizationMember>>>), (StatusCode, Json<ApiResponse<()>>)> {
    let org = sqlx::query_scalar!(
        "SELECT id FROM organizations WHERE name = $1",
        org_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org_id = match org {
        Some(id) => id,
        None => return Err(ApiResponse::error("ORG_NOT_FOUND", "Organization not found", StatusCode::NOT_FOUND)),
    };

    let rows = sqlx::query!(
        "SELECT id, org_id, user_id, role, created_at FROM org_members WHERE org_id = $1 ORDER BY created_at",
        org_id
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to list org members");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let members = rows.into_iter().map(|r| OrganizationMember {
        id: r.id,
        org_id: r.org_id,
        user_id: r.user_id,
        role: r.role,
        created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }).collect();

    Ok((StatusCode::OK, Json(ApiResponse::success(members))))
}

/// POST /api/v1/orgs/:org/members/:username — Add organization member
pub async fn add_organization_member(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((org_name, username)): Path<(String, String)>,
) -> Result<(StatusCode, Json<ApiResponse<OrganizationMember>>), (StatusCode, Json<ApiResponse<()>>)> {
    let requester_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    // Fetch org
    let org = sqlx::query!(
        "SELECT id, owner_id FROM organizations WHERE name = $1",
        org_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org = match org {
        Some(o) => o,
        None => return Err(ApiResponse::error("ORG_NOT_FOUND", "Organization not found", StatusCode::NOT_FOUND)),
    };

    // Only org owner can add members
    if org.owner_id != requester_id {
        return Err(ApiResponse::error("ORG_PERMISSION_DENIED", "Only the organization owner can add members", StatusCode::FORBIDDEN));
    }

    // Look up user to add
    let target = sqlx::query!(
        "SELECT id FROM users WHERE username = $1 AND is_active = true",
        username
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let target_id = match target {
        Some(u) => u.id,
        None => return Err(ApiResponse::error("USER_NOT_FOUND", "User not found", StatusCode::NOT_FOUND)),
    };

    let row = sqlx::query!(
        r#"INSERT INTO org_members (org_id, user_id, role)
           VALUES ($1, $2, 'member')
           ON CONFLICT (org_id, user_id) DO UPDATE SET role = 'member'
           RETURNING id, org_id, user_id, role, created_at"#,
        org.id, target_id
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to add org member");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to add member", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(OrganizationMember {
        id: row.id,
        org_id: row.org_id,
        user_id: row.user_id,
        role: row.role,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// DELETE /api/v1/orgs/:org/members/:username — Remove organization member
pub async fn remove_organization_member(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((org_name, username)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ApiResponse<()>>)> {
    let requester_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org = sqlx::query!(
        "SELECT id, owner_id FROM organizations WHERE name = $1",
        org_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org = match org {
        Some(o) => o,
        None => return Err(ApiResponse::error("ORG_NOT_FOUND", "Organization not found", StatusCode::NOT_FOUND)),
    };

    if org.owner_id != requester_id {
        return Err(ApiResponse::error("ORG_PERMISSION_DENIED", "Only the organization owner can remove members", StatusCode::FORBIDDEN));
    }

    let result = sqlx::query!(
        r#"DELETE FROM org_members om
           USING users u
           WHERE om.user_id = u.id AND om.org_id = $1 AND u.username = $2"#,
        org.id, username
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to remove org member");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to remove member", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiResponse::error("USER_NOT_FOUND", "User is not a member of this organization", StatusCode::NOT_FOUND));
    }

    Ok(StatusCode::NO_CONTENT)
}


