//! Pull Request endpoints
//! Handles PR creation, listing, reviews, comments, and merging

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::middleware::RequireAuth, routes::ApiResponse, AppState};

/// Pull Request representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub number: i32,
    pub title: String,
    pub description: Option<String>,
    pub author_id: Uuid,
    pub state: String,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    pub base_sha: String,
    pub created_at: String,
    pub updated_at: String,
}

/// PR Review representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRReview {
    pub id: Uuid,
    pub pr_id: Uuid,
    pub user_id: Uuid,
    pub state: String,
    pub body: Option<String>,
    pub created_at: String,
}

/// PR Comment representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRComment {
    pub id: Uuid,
    pub pr_id: Uuid,
    pub author_id: Uuid,
    pub file_path: String,
    pub line_number: i32,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Create PR request
#[derive(Debug, Deserialize)]
pub struct CreatePullRequestRequest {
    pub title: String,
    pub description: Option<String>,
    pub head_ref: String,
    pub base_ref: String,
}

/// Update PR request
#[derive(Debug, Deserialize)]
pub struct UpdatePullRequestRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
}

/// Create review request
#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub state: String,
    pub body: Option<String>,
}

/// Create comment request
#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub file_path: String,
    pub line_number: i32,
    pub body: String,
}

/// List PRs query params
#[derive(Debug, Deserialize)]
pub struct ListPRsQuery {
    pub state: Option<String>,
    pub limit: Option<i64>,
}

/// Merge response
#[derive(Debug, Serialize)]
pub struct MergeResponse {
    pub merged: bool,
    pub merge_commit_sha: String,
    pub message: String,
}

/// Helper: look up repo_id by owner username + repo name
async fn resolve_repo_id(
    pool: &sqlx::PgPool,
    owner: &str,
    repo: &str,
) -> Result<Uuid, (StatusCode, Json<ApiResponse<()>>)> {
    let row = sqlx::query!(
        "SELECT r.id FROM repositories r JOIN users u ON r.owner_id = u.id WHERE u.username = $1 AND r.name = $2",
        owner, repo
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "DB error resolving repo");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    match row {
        Some(r) => Ok(r.id),
        None => Err(ApiResponse::error("REPO_NOT_FOUND", "Repository not found", StatusCode::NOT_FOUND)),
    }
}

/// POST /api/v1/repos/:owner/:repo/pulls
pub async fn create_pull_request(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo)): Path<(String, String)>,
    Json(req): Json<CreatePullRequestRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PullRequest>>), (StatusCode, Json<ApiResponse<()>>)> {
    if req.title.is_empty() || req.head_ref.is_empty() || req.base_ref.is_empty() {
        return Err(ApiResponse::error("VALIDATE_REQUIRED_FIELD", "title, head_ref, and base_ref are required", StatusCode::BAD_REQUEST));
    }
    if req.head_ref == req.base_ref {
        return Err(ApiResponse::error("PR_INVALID_HEAD", "head_ref and base_ref must differ", StatusCode::BAD_REQUEST));
    }

    let author_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;

    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    // Get next PR number for this repo
    let next_num = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(number), 0) + 1 FROM pull_requests WHERE repo_id = $1",
        repo_id
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get next PR number");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?
    .unwrap_or(1);

    // Check for duplicate open PR on same head+base
    let existing = sqlx::query_scalar!(
        "SELECT id FROM pull_requests WHERE repo_id = $1 AND head_ref = $2 AND base_ref = $3 AND state = 'open'",
        repo_id, req.head_ref, req.base_ref
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "DB error checking duplicate PR");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    if existing.is_some() {
        return Err(ApiResponse::error("PR_ALREADY_EXISTS", "An open PR for this head/base pair already exists", StatusCode::CONFLICT));
    }

    let zero_sha = "0".repeat(64);
    let row = sqlx::query!(
        r#"INSERT INTO pull_requests (repo_id, number, title, description, author_id, state, head_ref, head_sha, base_ref, base_sha)
           VALUES ($1, $2, $3, $4, $5, 'open', $6, $7, $8, $9)
           RETURNING id, repo_id, number, title, description, author_id, state,
                     head_ref, head_sha, base_ref, base_sha, created_at, updated_at"#,
        repo_id, next_num, req.title, req.description, author_id,
        req.head_ref, zero_sha, req.base_ref, zero_sha
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create PR");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create pull request", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(PullRequest {
        id: row.id,
        repo_id: row.repo_id,
        number: row.number,
        title: row.title,
        description: row.description,
        author_id: row.author_id,
        state: row.state,
        head_ref: row.head_ref,
        head_sha: row.head_sha,
        base_ref: row.base_ref,
        base_sha: row.base_sha,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// GET /api/v1/repos/:owner/:repo/pulls
pub async fn list_pull_requests(
    State(state): State<Arc<AppState>>,
    _auth: Option<RequireAuth>,
    Path((owner, repo)): Path<(String, String)>,
    Query(q): Query<ListPRsQuery>,
) -> Result<Json<ApiResponse<Vec<PullRequest>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;
    let filter_state = q.state.unwrap_or_else(|| "open".to_string());
    let limit = q.limit.unwrap_or(20).min(100);

    let rows = sqlx::query!(
        r#"SELECT id, repo_id, number, title, description, author_id, state,
                  head_ref, head_sha, base_ref, base_sha, created_at, updated_at
           FROM pull_requests
           WHERE repo_id = $1 AND state = $2
           ORDER BY number DESC
           LIMIT $3"#,
        repo_id, filter_state, limit
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to list PRs");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to list pull requests", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let prs = rows.into_iter().map(|r| PullRequest {
        id: r.id,
        repo_id: r.repo_id,
        number: r.number,
        title: r.title,
        description: r.description,
        author_id: r.author_id,
        state: r.state,
        head_ref: r.head_ref,
        head_sha: r.head_sha,
        base_ref: r.base_ref,
        base_sha: r.base_sha,
        created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: r.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }).collect();

    Ok(Json(ApiResponse::success(prs)))
}

/// GET /api/v1/repos/:owner/:repo/pulls/:number
pub async fn get_pull_request(
    State(state): State<Arc<AppState>>,
    _auth: Option<RequireAuth>,
    Path((owner, repo, number)): Path<(String, String, i32)>,
) -> Result<Json<ApiResponse<PullRequest>>, (StatusCode, Json<ApiResponse<()>>)> {
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    let row = sqlx::query!(
        r#"SELECT id, repo_id, number, title, description, author_id, state,
                  head_ref, head_sha, base_ref, base_sha, created_at, updated_at
           FROM pull_requests WHERE repo_id = $1 AND number = $2"#,
        repo_id, number
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch PR");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    match row {
        Some(r) => Ok(Json(ApiResponse::success(PullRequest {
            id: r.id,
            repo_id: r.repo_id,
            number: r.number,
            title: r.title,
            description: r.description,
            author_id: r.author_id,
            state: r.state,
            head_ref: r.head_ref,
            head_sha: r.head_sha,
            base_ref: r.base_ref,
            base_sha: r.base_sha,
            created_at: r.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            updated_at: r.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }))),
        None => Err(ApiResponse::error("PR_NOT_FOUND", &format!("Pull request #{} not found", number), StatusCode::NOT_FOUND)),
    }
}

/// PATCH /api/v1/repos/:owner/:repo/pulls/:number
pub async fn update_pull_request(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo, number)): Path<(String, String, i32)>,
    Json(req): Json<UpdatePullRequestRequest>,
) -> Result<Json<ApiResponse<PullRequest>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    // Only PR author or repo owner can update
    let pr_check = sqlx::query!(
        "SELECT id, author_id FROM pull_requests WHERE repo_id = $1 AND number = $2",
        repo_id, number
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "DB error fetching PR for update");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let pr = match pr_check {
        Some(p) => p,
        None => return Err(ApiResponse::error("PR_NOT_FOUND", &format!("Pull request #{} not found", number), StatusCode::NOT_FOUND)),
    };

    if pr.author_id != user_id {
        // Check if user is repo owner
        let is_owner = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM repositories r JOIN users u ON r.owner_id = u.id WHERE r.id = $1 AND u.id = $2)",
            repo_id, user_id
        )
        .fetch_one(&state.db.pool)
        .await
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?
        .unwrap_or(false);

        if !is_owner {
            return Err(ApiResponse::error("REPO_PERMISSION_DENIED", "Only the PR author or repo owner can update this PR", StatusCode::FORBIDDEN));
        }
    }

    let row = sqlx::query!(
        r#"UPDATE pull_requests
           SET title       = COALESCE($1, title),
               description = COALESCE($2, description),
               state       = COALESCE($3, state),
               updated_at  = NOW()
           WHERE id = $4
           RETURNING id, repo_id, number, title, description, author_id, state,
                     head_ref, head_sha, base_ref, base_sha, created_at, updated_at"#,
        req.title, req.description, req.state, pr.id
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to update PR");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to update pull request", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok(Json(ApiResponse::success(PullRequest {
        id: row.id,
        repo_id: row.repo_id,
        number: row.number,
        title: row.title,
        description: row.description,
        author_id: row.author_id,
        state: row.state,
        head_ref: row.head_ref,
        head_sha: row.head_sha,
        base_ref: row.base_ref,
        base_sha: row.base_sha,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    })))
}

/// POST /api/v1/repos/:owner/:repo/pulls/:number/reviews
pub async fn create_review(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo, number)): Path<(String, String, i32)>,
    Json(req): Json<CreateReviewRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PRReview>>), (StatusCode, Json<ApiResponse<()>>)> {
    if !["approved", "requested_changes", "commented"].contains(&req.state.as_str()) {
        return Err(ApiResponse::error("VALIDATE_INVALID_ENUM", "state must be 'approved', 'requested_changes', or 'commented'", StatusCode::BAD_REQUEST));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    let pr = sqlx::query_scalar!(
        "SELECT id FROM pull_requests WHERE repo_id = $1 AND number = $2",
        repo_id, number
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let pr_id = match pr {
        Some(id) => id,
        None => return Err(ApiResponse::error("PR_NOT_FOUND", &format!("Pull request #{} not found", number), StatusCode::NOT_FOUND)),
    };

    let row = sqlx::query!(
        r#"INSERT INTO pr_reviews (pr_id, user_id, state, body)
           VALUES ($1, $2, $3, $4)
           RETURNING id, pr_id, user_id, state, body, created_at"#,
        pr_id, user_id, req.state, req.body
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create review");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create review", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(PRReview {
        id: row.id,
        pr_id: row.pr_id,
        user_id: row.user_id,
        state: row.state,
        body: row.body,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// POST /api/v1/repos/:owner/:repo/pulls/:number/comments
pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims }: RequireAuth,
    Path((owner, repo, number)): Path<(String, String, i32)>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PRComment>>), (StatusCode, Json<ApiResponse<()>>)> {
    if req.file_path.is_empty() || req.body.is_empty() {
        return Err(ApiResponse::error("VALIDATE_REQUIRED_FIELD", "file_path and body are required", StatusCode::BAD_REQUEST));
    }

    let author_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Invalid user ID", StatusCode::INTERNAL_SERVER_ERROR))?;
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    let pr = sqlx::query_scalar!(
        "SELECT id FROM pull_requests WHERE repo_id = $1 AND number = $2",
        repo_id, number
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let pr_id = match pr {
        Some(id) => id,
        None => return Err(ApiResponse::error("PR_NOT_FOUND", &format!("Pull request #{} not found", number), StatusCode::NOT_FOUND)),
    };

    let row = sqlx::query!(
        r#"INSERT INTO pr_comments (pr_id, author_id, file_path, line_number, body)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, pr_id, author_id, file_path, line_number, body, created_at, updated_at"#,
        pr_id, author_id, req.file_path, req.line_number, req.body
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create comment");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to create comment", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(PRComment {
        id: row.id,
        pr_id: row.pr_id,
        author_id: row.author_id,
        file_path: row.file_path,
        line_number: row.line_number,
        body: row.body,
        created_at: row.created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: row.updated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))))
}

/// POST /api/v1/repos/:owner/:repo/pulls/:number/merge
pub async fn merge_pull_request(
    State(state): State<Arc<AppState>>,
    RequireAuth { claims: _ }: RequireAuth,
    Path((owner, repo, number)): Path<(String, String, i32)>,
) -> Result<Json<ApiResponse<MergeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let repo_id = resolve_repo_id(&state.db.pool, &owner, &repo).await?;

    let pr = sqlx::query!(
        "SELECT id, state FROM pull_requests WHERE repo_id = $1 AND number = $2",
        repo_id, number
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| ApiResponse::error("SERVER_INTERNAL_ERROR", "Database error", StatusCode::INTERNAL_SERVER_ERROR))?;

    let pr = match pr {
        Some(p) => p,
        None => return Err(ApiResponse::error("PR_NOT_FOUND", &format!("Pull request #{} not found", number), StatusCode::NOT_FOUND)),
    };

    match pr.state.as_str() {
        "merged" => return Err(ApiResponse::error("PR_ALREADY_MERGED", "This pull request has already been merged", StatusCode::CONFLICT)),
        "closed" => return Err(ApiResponse::error("PR_ALREADY_CLOSED", "This pull request is closed", StatusCode::GONE)),
        _ => {}
    }

    // Mark as merged
    let merge_sha = Uuid::new_v4().to_string().replace('-', "");
    sqlx::query!(
        "UPDATE pull_requests SET state = 'merged', updated_at = NOW() WHERE id = $1",
        pr.id
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to merge PR");
        ApiResponse::error("SERVER_INTERNAL_ERROR", "Failed to merge pull request", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok(Json(ApiResponse::success(MergeResponse {
        merged: true,
        merge_commit_sha: merge_sha,
        message: format!("Pull request #{} merged successfully", number),
    })))
}


