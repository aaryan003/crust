// contracts/data-types.rs
// VERSION: 1.0.0
// WRITTEN_BY: contracts-agent
// CONSUMED_BY: gitcore, crust-server, crust-cli
// LAST_UPDATED: 2026-03-04

/// Standard wrapper for all API responses and internal operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub timestamp: String, // ISO8601 UTC
    pub duration: u64,     // milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,                    // matches error-codes.md
    pub message: String,                 // human-readable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,           // for validation errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// User account in the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,           // UUID
    pub username: String,     // unique, lowercase, alphanumeric + dash
    pub email: String,        // unique
    #[serde(skip_serializing)]
    pub password_hash: String, // argon2
    pub display_name: String,
    #[serde(skip_serializing)]
    pub created_at: String,   // ISO8601 UTC
    #[serde(skip_serializing)]
    pub updated_at: String,   // ISO8601 UTC
}

/// Repository on the platform
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repository {
    pub id: String,            // UUID
    pub owner_id: String,      // user or org id
    pub name: String,          // unique within owner, lowercase, alphanumeric + dash/underscore
    pub display_name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub default_branch: String, // e.g., "main"
    pub created_at: String,    // ISO8601 UTC
    pub updated_at: String,    // ISO8601 UTC
}

/// Permission level for repository access
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum RepositoryPermission {
    #[serde(rename = "owner")]
    Owner,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "read")]
    Read,
}

/// User's access to a specific repository
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RepoPermission {
    pub id: String,
    pub user_id: String,
    pub repo_id: String,
    pub permission: String, // "owner", "write", "read"
    pub created_at: String,
}

/// Organization for grouping repos and users
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: String,
    pub name: String,          // unique, lowercase, alphanumeric + dash
    pub display_name: String,
    pub description: Option<String>,
    pub owner_id: String,      // user who created org
    pub created_at: String,
    pub updated_at: String,
}

/// Team within an organization
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Team {
    pub id: String,
    pub org_id: String,
    pub name: String,          // unique within org
    pub display_name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Pull request
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PullRequest {
    pub id: String,
    pub repo_id: String,
    pub number: i32,           // sequential per repo, not global
    pub title: String,
    pub description: Option<String>,
    pub author_id: String,
    pub state: String,         // "open", "merged", "closed"
    pub head_ref: String,      // source branch name
    pub head_sha: String,      // commit SHA256
    pub base_ref: String,      // target branch name
    pub base_sha: String,      // commit SHA256
    pub created_at: String,
    pub updated_at: String,
}

/// Pull request review
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PullRequestReview {
    pub id: String,
    pub pr_id: String,
    pub user_id: String,
    pub state: String,         // "pending", "approved", "requested_changes", "commented"
    pub body: Option<String>,
    pub created_at: String,
}

/// A single commit from the server's perspective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,           // SHA256 hex
    pub tree_sha: String,
    pub parent_shas: Vec<String>,
    pub author: PersonInfo,
    pub committer: PersonInfo,
    pub message: String,
}

/// Author/Committer signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonInfo {
    pub name: String,
    pub email: String,
    pub timestamp: i64,        // unix seconds
    pub timezone_offset: String, // e.g., "+0000", "-0500"
}

/// A single tree entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub mode: String,          // "100644", "100755", "040000", "120000"
    pub name: String,
    pub sha: String,           // SHA256 hex
}

/// Auth token request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Auth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,         // JWT
    pub expires_at: String,    // ISO8601 UTC
}

/// Preflight response for push/fetch (tells client what server needs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefPreflight {
    pub wants: Vec<String>,    // object SHAs server needs
    pub haves: Vec<String>,    // object SHAs server already has
}

/// Handshake for object upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectUploadStart {
    pub pack_size: u64,        // total bytes to upload
}

/// Result of upload attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectUploadResult {
    pub success: bool,
    pub objects_stored: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Request to update remote refs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefUpdateRequest {
    pub updates: Vec<RefUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefUpdate {
    pub ref_name: String,      // e.g., "refs/heads/main"
    pub old_sha: Option<String>, // for conflict detection; None for force
    pub new_sha: String,
}

/// Response to ref update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefUpdateResult {
    pub ref_name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Object metadata (used for storage/transmission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub sha256: String,        // 64 char hex
    pub obj_type: String,      // "blob", "tree", "commit", "tag"
    pub size: u64,             // uncompressed size
    pub compressed_size: u64,  // after zstd
}

/// Git index entry (from .crust/index file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: String,
    pub sha256: String,        // object SHA256
    pub mode: u32,             // file mode
    pub mtime_sec: u64,
    pub mtime_nsec: u32,
    pub ctime_sec: u64,
    pub ctime_nsec: u32,
    pub size: u32,
    pub flags: u32,
}
