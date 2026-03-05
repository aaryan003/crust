//! Team endpoints
//!
//! Endpoints for managing teams and team members

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::middleware::RequireAuth, routes::ApiResponse, AppState};

/// Team metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: String,
}

/// Team repository assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRepoAssignment {
    pub id: Uuid,
    pub team_id: Uuid,
    pub repo_id: Uuid,
    pub permission: String,
    pub created_at: String,
}

/// Create team request
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
}

/// Grant team access request
#[derive(Debug, Deserialize)]
pub struct GrantTeamAccessRequest {
    pub permission: String,
}

/// Helper: resolve org_id by name + verify requester is owner
async fn require_org_owner(
    pool: &sqlx::PgPool,
    org_name: &str,
    requester_id: Uuid,
) -> Result<Uuid, (StatusCode, Json<ApiResponse<()>>)> {
    let org = sqlx::query!(
        "SELECT id, owner_id FROM organizations WHERE name = $1",
        org_name
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    match org {
        Some(o) if o.owner_id == requester_id => Ok(o.id),
        Some(_) => Err(ApiResponse::error("ORG_PERMISSION_DENIED", "Only the organization owner can manage teams", StatusCode::FORBIDDEN)),
        None => Err(ApiResponse::error("ORG_NOT_FOUND", "Organization not found", StatusCode::NOT_FOUND)),
    }
}

/// POST /api/v1/orgs/:org/teams — Create team
pub async fn create_team(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path(org_name): Path<String>,
    Json(req): Json<CreateTeamRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Team>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org_id = require_org_owner(&state.db.pool, &org_name, user_id).await?;

    let row = sqlx::query!(
        r#"INSERT INTO teams (org_id, name, display_name, description)
           VALUES ($1, $2, $3, $4)
           RETURNING id, org_id, name, display_name, description, created_at, updated_at"#,
        org_id, req.name, req.display_name, req.description
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiResponse::error("TEAM_ALREADY_EXISTS", "A team with that name already exists in this organization", StatusCode::CONFLICT)
        } else {
            tracing::error!(error = %e, "Failed to create team");
            ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create team", StatusCode::INTERNAL_SERVER_ERROR)
        }
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(Team {
        id: row.id,
        org_id: row.org_id,
        name: row.name,
        display_name: row.display_name,
        description: row.description,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// GET /api/v1/orgs/:org/teams — List teams in organization
pub async fn list_teams(
    State(state): State<Arc<AppState>>,
    Path(org_name): Path<String>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<Team>>>), (StatusCode, Json<ApiResponse<()>>)> {
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
        "SELECT id, org_id, name, display_name, description, created_at, updated_at FROM teams WHERE org_id = $1 ORDER BY name",
        org_id
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to list teams");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let teams = rows.into_iter().map(|r| Team {
        id: r.id,
        org_id: r.org_id,
        name: r.name,
        display_name: r.display_name,
        description: r.description,
        created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: r.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }).collect();

    Ok((StatusCode::OK, Json(ApiResponse::success(teams))))
}

/// PUT /api/v1/orgs/:org/teams/:team/repos/:owner/:repo — Grant team access to repository
pub async fn grant_team_access(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((org_name, team_name, repo_owner, repo_name)): Path<(String, String, String, String)>,
    Json(req): Json<GrantTeamAccessRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TeamRepoAssignment>>), (StatusCode, Json<ApiResponse<()>>)> {
    if req.permission != "read" && req.permission != "write" {
        return Err(ApiResponse::error("VALIDATE_INVALID_ENUM", "Permission must be 'read' or 'write'", StatusCode::BAD_REQUEST));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org_id = require_org_owner(&state.db.pool, &org_name, user_id).await?;

    // Resolve team
    let team = sqlx::query_scalar!(
        "SELECT id FROM teams WHERE org_id = $1 AND name = $2",
        org_id, team_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let team_id = match team {
        Some(id) => id,
        None => return Err(ApiResponse::error("TEAM_NOT_FOUND", "Team not found", StatusCode::NOT_FOUND)),
    };

    // Resolve repo (repo owner must match the authenticated user for permission check)
    let repo = sqlx::query!(
        "SELECT r.id FROM repositories r JOIN users u ON r.owner_id = u.id WHERE u.username = $1 AND r.name = $2",
        repo_owner, repo_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let repo_id = match repo {
        Some(r) => r.id,
        None => return Err(ApiResponse::error("REPO_NOT_FOUND", "Repository not found", StatusCode::NOT_FOUND)),
    };

    let row = sqlx::query!(
        r#"INSERT INTO team_repos (team_id, repo_id, permission)
           VALUES ($1, $2, $3)
           ON CONFLICT (team_id, repo_id) DO UPDATE SET permission = $3
           RETURNING id, team_id, repo_id, permission, created_at"#,
        team_id, repo_id, req.permission
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to grant team access");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to grant team access", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::OK, Json(ApiResponse::success(TeamRepoAssignment {
        id: row.id,
        team_id: row.team_id,
        repo_id: row.repo_id,
        permission: row.permission,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// POST /api/v1/orgs/:org/teams/:team/members/:username — Add user to team
pub async fn add_team_member(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((org_name, team_name, username)): Path<(String, String, String)>,
) -> Result<(StatusCode, Json<ApiResponse<TeamMember>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    let org_id = require_org_owner(&state.db.pool, &org_name, user_id).await?;

    let team = sqlx::query_scalar!(
        "SELECT id FROM teams WHERE org_id = $1 AND name = $2",
        org_id, team_name
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let team_id = match team {
        Some(id) => id,
        None => return Err(ApiResponse::error("TEAM_NOT_FOUND", "Team not found", StatusCode::NOT_FOUND)),
    };

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
        r#"INSERT INTO team_members (team_id, user_id, role)
           VALUES ($1, $2, 'member')
           ON CONFLICT (team_id, user_id) DO UPDATE SET role = 'member'
           RETURNING id, team_id, user_id, role, created_at"#,
        team_id, target_id
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to add team member");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to add team member", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(TeamMember {
        id: row.id,
        team_id: row.team_id,
        user_id: row.user_id,
        role: row.role,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}


