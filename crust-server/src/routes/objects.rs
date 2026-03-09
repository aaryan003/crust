// Object transport endpoints - handles CRUSTPACK format upload/download
// Implements contracts/crustpack-format.md and contracts/api-contracts.md

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::middleware::RequireAuth,
    permissions::PermissionContext,
    routes::ApiResponse,
    storage::{ObjectStore, PackReader, PackWriter},
    AppState,
};

/// Preflight request - tell server what objects we have and want
#[derive(Debug, Deserialize)]
pub struct RefPreflightRequest {
    /// Object IDs we want from the server
    pub wants: Vec<String>,
    /// Object IDs we already have
    pub haves: Vec<String>,
}

/// Preflight response - acknowledge wants/haves
#[derive(Debug, Serialize)]
pub struct RefPreflightResponse {
    pub wants: Vec<String>,
    pub haves: Vec<String>,
}

/// Upload result
#[derive(Debug, Serialize)]
pub struct ObjectUploadResult {
    pub objects_stored: usize,
    pub conflicts: Vec<String>,
}

/// Ref update request
#[derive(Debug, Deserialize)]
pub struct RefUpdateRequest {
    pub updates: Vec<RefUpdate>,
}

/// Single ref update
#[derive(Debug, Deserialize)]
pub struct RefUpdate {
    pub ref_name: String,
    pub old_sha: String,
    pub new_sha: String,
    #[serde(default)]
    pub force: bool,
}

/// Ref update response
#[derive(Debug, Serialize)]
pub struct RefUpdateResponse {
    pub ref_name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Fetch request - what objects do we want?
#[derive(Debug, Deserialize)]
pub struct ObjectFetchRequest {
    pub wants: Vec<String>,
    #[serde(default)]
    pub haves: Vec<String>,
}

/// Handler for POST /api/v1/repos/:owner/:repo/refs/preflight
///
/// Client tells server what objects it wants and what it already has.
/// Server validates the wants/haves and acknowledges.
pub async fn preflight_handler(
    State(_state): State<Arc<AppState>>,
    Path((_owner, _repo)): Path<(String, String)>,
    RequireAuth { .. }: RequireAuth,
    Json(req): Json<RefPreflightRequest>,
) -> impl IntoResponse {
    let response = RefPreflightResponse {
        wants: req.wants.clone(),
        haves: req.haves.clone(),
    };

    let api_response = ApiResponse {
        success: true,
        data: Some(response),
        error: None,
        metadata: crate::routes::ResponseMetadata {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            duration: 0,
            request_id: None,
        },
    };

    (StatusCode::OK, Json(api_response))
}

/// Handler for POST /api/v1/repos/:owner/:repo/objects/upload
///
/// Client uploads CRUSTPACK-formatted object data.
/// Server deserializes, validates each object, stores to disk.
pub async fn upload_handler(
    State(state): State<Arc<AppState>>,
    Path((owner, repo)): Path<(String, String)>,
    RequireAuth { claims }: RequireAuth,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "AUTH_INVALID_TOKEN".to_string(),
                    message: "Invalid user ID in token".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::UNAUTHORIZED, Json(error_response)).into_response();
        }
    };

    // Look up repo by owner username + repo name, get owner_id and is_public
    let repo_row = sqlx::query!(
        r#"SELECT r.id as repo_id, r.owner_id, r.is_public
           FROM repositories r
           JOIN users u ON r.owner_id = u.id
           WHERE u.username = $1 AND r.name = $2"#,
        owner,
        repo
    )
    .fetch_optional(&state.db.pool)
    .await;

    let (repo_owner_id, repo_is_public) = match repo_row {
        Ok(Some(row)) => (row.owner_id, row.is_public),
        Ok(None) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "REPO_NOT_FOUND".to_string(),
                    message: format!("Repository '{}/{}' not found", owner, repo),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::NOT_FOUND, Json(error_response)).into_response();
        }
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_INTERNAL_ERROR".to_string(),
                    message: "Database error".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    // Check write permission: owner or public repo member
    let perm_ctx = PermissionContext {
        user_id,
        repo_owner_id,
        repo_is_public,
    };

    if !perm_ctx.can_write() {
        let error_response = ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(crate::routes::ApiError {
                code: "REPO_PERMISSION_DENIED".to_string(),
                message: "You don't have write access to this repository".to_string(),
                field: None,
            }),
            metadata: crate::routes::ResponseMetadata {
                timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                duration: 0,
                request_id: None,
            },
        };
        return (StatusCode::FORBIDDEN, Json(error_response)).into_response();
    }

    // Deserialize CRUSTPACK
    let objects = match PackReader::deserialize(&body) {
        Ok(objs) => objs,
        Err(e) => {
            let error_code = if e.to_string().contains("SHA256") {
                "PACK_CHECKSUM_MISMATCH"
            } else {
                "PACK_MALFORMED"
            };

            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: error_code.to_string(),
                    message: format!("Pack deserialization failed: {}", e),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
        }
    };

    // Store objects
    let store = match ObjectStore::new("/data/repos") {
        Ok(s) => s,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_DISK_FULL".to_string(),
                    message: "Failed to initialize object store".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::SERVICE_UNAVAILABLE, Json(error_response)).into_response();
        }
    };

    let mut stored_count = 0;
    let mut conflicts = Vec::new();

    for (object_id, _object_type, data) in objects {
        match store.save_object(&owner, &repo, &data) {
            Ok(_) => stored_count += 1,
            Err(e) => {
                conflicts.push(format!("Object {}: {}", object_id.as_str(), e));
            }
        }
    }

    let response = ObjectUploadResult {
        objects_stored: stored_count,
        conflicts,
    };

    let api_response = ApiResponse {
        success: true,
        data: Some(response),
        error: None,
        metadata: crate::routes::ResponseMetadata {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            duration: 0,
            request_id: None,
        },
    };

    (StatusCode::OK, Json(api_response)).into_response()
}

/// Handler for POST /api/v1/repos/:owner/:repo/objects/fetch
///
/// Client requests objects. Server returns them in CRUSTPACK format.
pub async fn fetch_handler(
    State(_state): State<Arc<AppState>>,
    Path((owner, repo)): Path<(String, String)>,
    RequireAuth { claims }: RequireAuth,
    Json(req): Json<ObjectFetchRequest>,
) -> impl IntoResponse {
    let _user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "AUTH_INVALID_TOKEN".to_string(),
                    message: "Invalid user ID in token".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::UNAUTHORIZED, Json(error_response)).into_response();
        }
    };

    // TODO: Verify read permission on repo
    // For now, allow any authenticated user

    // Load objects from disk
    let store = match ObjectStore::new("/data/repos") {
        Ok(s) => s,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_INTERNAL_ERROR".to_string(),
                    message: "Failed to initialize object store".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let mut writer = PackWriter::new();

    // Use BFS graph traversal: walk commits → trees → blobs
    let reachable = store.collect_reachable_objects(&owner, &repo, &req.wants, &req.haves);
    let found_count = reachable.len();

    for (object_id, obj_type, data) in reachable {
        writer.add_object(object_id, obj_type, data);
    }

    if found_count == 0 {
        let error_response = ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(crate::routes::ApiError {
                code: "PACK_EMPTY".to_string(),
                message: "No objects found for requested IDs".to_string(),
                field: None,
            }),
            metadata: crate::routes::ResponseMetadata {
                timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                duration: 0,
                request_id: None,
            },
        };
        return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
    }

    // Serialize to CRUSTPACK
    match writer.serialize() {
        Ok(pack_bytes) => {
            // Return raw bytes with octet-stream content type
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
                pack_bytes,
            )
                .into_response()
        }
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_INTERNAL_ERROR".to_string(),
                    message: "Failed to serialize pack".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

/// Handler for POST /api/v1/repos/:owner/:repo/refs/update
///
/// Client requests atomic reference update (e.g., fast-forward branch pointer).
/// Server verifies old_sha matches current value, then updates to new_sha.
pub async fn update_refs_handler(
    State(_state): State<Arc<AppState>>,
    Path((owner, repo)): Path<(String, String)>,
    RequireAuth { .. }: RequireAuth,
    Json(req): Json<RefUpdateRequest>,
) -> impl IntoResponse {
    let store = match ObjectStore::new("/data/repos") {
        Ok(s) => s,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_INTERNAL_ERROR".to_string(),
                    message: "Failed to initialize object store".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let mut responses = Vec::new();

    for update in req.updates {
        // ref_name is like "refs/heads/main" — write under that path
        // Extract branch name from ref_name (e.g. "refs/heads/main" → "main")
        let branch_name = update
            .ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&update.ref_name);

        // Non-fast-forward check: if force=false, verify old_sha matches current tip
        if !update.force {
            let null_sha = "0000000000000000000000000000000000000000000000000000000000000000";
            let current_tips = store.list_refs(&owner, &repo, "heads");
            let current_tip = current_tips.get(branch_name).cloned().unwrap_or_default();

            // If the remote has a tip and it doesn't match what the client expected
            if !current_tip.is_empty()
                && current_tip != update.old_sha
                && update.old_sha != null_sha
            {
                responses.push(RefUpdateResponse {
                    ref_name: update.ref_name.clone(),
                    ok: false,
                    error: Some(format!(
                        "PUSH_REJECTED: Non-fast-forward update rejected. Pull first (remote tip: {})",
                        &current_tip[..12]
                    )),
                });
                continue;
            }
        }

        let ok = store
            .write_ref(&owner, &repo, &update.ref_name, &update.new_sha)
            .is_ok();

        responses.push(RefUpdateResponse {
            ref_name: update.ref_name.clone(),
            ok,
            error: if ok {
                None
            } else {
                Some("Failed to write ref".to_string())
            },
        });
    }

    let api_response = ApiResponse {
        success: true,
        data: Some(responses),
        error: None,
        metadata: crate::routes::ResponseMetadata {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            duration: 0,
            request_id: None,
        },
    };

    (StatusCode::OK, Json(api_response)).into_response()
}

/// Refs response - branch and tag listings
#[derive(Debug, Serialize)]
pub struct RefsResponse {
    pub heads: HashMap<String, String>,
    pub tags: HashMap<String, String>,
}

/// Handler for GET /api/v1/repos/:owner/:repo/refs
///
/// Returns all branches (heads) and tags for the repository.
/// Reads from disk: /data/repos/{owner}/{repo}.crust/refs/heads/
pub async fn list_refs_handler(
    State(_state): State<Arc<AppState>>,
    Path((owner, repo)): Path<(String, String)>,
) -> impl IntoResponse {
    let store = match ObjectStore::new("/data/repos") {
        Ok(s) => s,
        Err(_) => {
            let error_response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(crate::routes::ApiError {
                    code: "SERVER_INTERNAL_ERROR".to_string(),
                    message: "Failed to initialize object store".to_string(),
                    field: None,
                }),
                metadata: crate::routes::ResponseMetadata {
                    timestamp: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    duration: 0,
                    request_id: None,
                },
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response();
        }
    };

    let heads = store.list_refs(&owner, &repo, "heads");
    let tags = store.list_refs(&owner, &repo, "tags");

    let response = RefsResponse { heads, tags };

    let api_response = ApiResponse {
        success: true,
        data: Some(response),
        error: None,
        metadata: crate::routes::ResponseMetadata {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            duration: 0,
            request_id: None,
        },
    };

    (StatusCode::OK, Json(api_response)).into_response()
}
