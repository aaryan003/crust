# API Contracts Specification

VERSION: 1.0.0
WRITTEN_BY: server-agent
CONSUMED_BY: crust-cli, frontend developers (v2)
LAST_UPDATED: 2026-03-04

Base path: `/api/v1`
All responses wrapped in `ApiResponse<T>` (see contracts/data-types.rs)
All errors return error codes from contracts/error-codes.md

---

## Authentication

### POST /api/v1/auth/register

Register a new user account.

**Request**:
```json
{
  "username": "jane_doe",
  "email": "jane@example.com",
  "password": "SecurePassword123!"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user": { "id": "uuid", "username": "jane_doe", ... },
    "token": "eyJ...",
    "expires_at": "2026-03-05T10:30:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `VALIDATE_USERNAME_TAKEN` (409)
- `VALIDATE_EMAIL_TAKEN` (409)
- `VALIDATE_WEAK_PASSWORD` (400)
- `VALIDATE_INVALID_EMAIL` (400)
- `AUTH_REGISTRATION_DISABLED` (403)

**Auth required**: No

---

### POST /api/v1/auth/login

Authenticate and receive JWT.

**Request**:
```json
{
  "username": "jane_doe",
  "password": "SecurePassword123!"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user": { ... },
    "token": "eyJ...",
    "expires_at": "2026-03-05T10:30:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `AUTH_INVALID_CREDENTIALS` (401)
- `AUTH_USER_NOT_FOUND` (401)
- `AUTH_ACCOUNT_DISABLED` (403)

**Auth required**: No

---

### POST /api/v1/auth/logout

Revoke current token.

**Request**: Empty body

**Response** (204 No Content): Empty

**Errors**:
- `AUTH_MISSING_HEADER` (401)
- `AUTH_TOKEN_INVALID` (401)

**Auth required**: Yes (Bearer token)

---

### GET /api/v1/auth/me

Get authenticated user's profile.

**Response** (200 OK):
```json
{
  "success": true,
  "data": { "id": "uuid", "username": "jane_doe", ... },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `AUTH_MISSING_HEADER` (401)
- `AUTH_TOKEN_EXPIRED` (401)
- `AUTH_TOKEN_REVOKED` (401)

**Auth required**: Yes

---

## Users

### GET /api/v1/users/:username

Get public user profile.

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "username": "jane_doe",
    "display_name": "Jane Doe",
    "created_at": "2026-03-01T10:00:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `USER_NOT_FOUND` (404)

**Auth required**: No

---

### PATCH /api/v1/users/me

Update own profile.

**Request**:
```json
{
  "display_name": "Jane Smith",
  "password": "NewPassword123!"
}
```

(Optional fields, omit to leave unchanged)

**Response** (200 OK): Updated user object

**Errors**:
- `AUTH_MISSING_HEADER` (401)
- `VALIDATE_WEAK_PASSWORD` (400)
- `USER_PERMISSION_DENIED` (403)

**Auth required**: Yes

---

## Repositories

### POST /api/v1/repos

Create a new repository.

**Request**:
```json
{
  "name": "my-project",
  "display_name": "My Project",
  "description": "A cool project",
  "is_public": false,
  "default_branch": "main"
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "owner_id": "uuid",
    "name": "my-project",
    ...
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `AUTH_MISSING_HEADER` (401)
- `VALIDATE_REQUIRED_FIELD` (400)
- `REPO_ALREADY_EXISTS` (409)
- `REPO_NAME_INVALID` (400)

**Auth required**: Yes

---

### GET /api/v1/repos/:owner/:repo

Get repository metadata.

**Response** (200 OK): Full repository object

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### PATCH /api/v1/repos/:owner/:repo

Update repository (owner only).

**Request**:
```json
{
  "display_name": "Updated Name",
  "is_public": true
}
```

**Response** (200 OK): Updated repo object

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: Yes

---

### DELETE /api/v1/repos/:owner/:repo

Delete repository (owner only).

**Response** (204 No Content): Empty

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: Yes

---

### GET /api/v1/repos/:owner/:repo/refs

List all branches and tags.

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "heads": {
      "main": "3a7f8e9c...",
      "dev": "1b2c3d4e..."
    },
    "tags": {
      "v0.1.0": "abc1234...",
      "v0.2.0": "def4567..."
    }
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/tree/:ref?/:path?

Get tree listing at a reference and path.

**Query params**:
- `ref`: Branch/tag name or commit SHA (default: default_branch)
- `path`: File path within tree (default: root)

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    { "mode": "100644", "name": "README.md", "sha": "...", "type": "blob" },
    { "mode": "040000", "name": "src", "sha": "...", "type": "tree" }
  ],
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REF_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/blob/:ref/:path

Get file blob content.

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "path": "README.md",
    "sha": "...",
    "size": 1024,
    "content": "base64-encoded content"
  },
  "error": null,
  "metadata": { ... }
}
```

Or raw binary (if `Accept: application/octet-stream`):
```
[raw file bytes]
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REF_NOT_FOUND` (404)
- `OBJECT_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/commits/:ref?

List commits on a branch/tag.

**Query params**:
- `ref`: Branch/tag/SHA (default: default_branch)
- `limit`: Max commits (default: 20, max: 100)

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "sha": "3a7f...",
      "tree_sha": "abc1...",
      "parent_shas": ["1b2c..."],
      "author": { "name": "Jane", "email": "jane@example.com", "timestamp": 1704067200, "timezone_offset": "+0000" },
      "committer": { ... },
      "message": "Initial commit"
    }
  ],
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REF_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/commits/:sha

Get single commit details.

**Response** (200 OK): Commit object (same as above)

**Errors**:
- `REPO_NOT_FOUND` (404)
- `OBJECT_NOT_FOUND` (404)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/compare/:base...:head

Compare two commits/branches.

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "base_sha": "1b2c...",
    "head_sha": "3a7f...",
    "commits": [...],
    "diff_summary": {
      "files_changed": 5,
      "insertions": 120,
      "deletions": 30
    }
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `REF_NOT_FOUND` (404)
- `OBJECT_NOT_FOUND` (404)

**Auth required**: No (if public), Yes (if private)

---

## Object Transport

### POST /api/v1/repos/:owner/:repo/refs/preflight

(Client → Server) Tell server what objects we need.

**Request**:
```json
{
  "wants": ["3a7f...", "1b2c..."],
  "haves": ["abc1234..."]
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "wants": ["3a7f...", "1b2c..."],
    "haves": ["abc1234..."]
  },
  "error": null,
  "metadata": { ... }
}
```

**Auth required**: Yes

---

### POST /api/v1/repos/:owner/:repo/objects/fetch

(Server → Client) Download objects in CRUSTPACK format.

**Request**: JSON with "wants" list (from preflight)

**Response** (200 OK): Binary CRUSTPACK data

**Errors**:
- `OBJECT_NOT_FOUND` (404)
- `PACK_EMPTY` (400)

**Auth required**: Yes

---

### POST /api/v1/repos/:owner/:repo/objects/upload

(Client → Server) Upload objects in CRUSTPACK format.

**Request**: Binary CRUSTPACK data

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "objects_stored": 42,
    "conflicts": []
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `PACK_MALFORMED` (400)
- `PACK_CHECKSUM_MISMATCH` (422)
- `OBJECT_CORRUPT` (422)
- `REPO_PERMISSION_DENIED` (403)
- `SERVER_DISK_FULL` (507)

**Auth required**: Yes (write permission on repo)

---

### POST /api/v1/repos/:owner/:repo/refs/update

(Client → Server) Atomic ref update (after objects uploaded).

**Request**:
```json
{
  "updates": [
    { "ref_name": "refs/heads/main", "old_sha": "1b2c...", "new_sha": "3a7f..." }
  ]
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    { "ref_name": "refs/heads/main", "ok": true }
  ],
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REF_CONFLICT` (409)
- `REF_LOCKED` (423)
- `REPO_PERMISSION_DENIED` (403)

**Auth required**: Yes (write permission)

---

## Pull Requests

### POST /api/v1/repos/:owner/:repo/pulls

Create a pull request.

**Request**:
```json
{
  "title": "Add auth system",
  "description": "Implements JWT-based authentication",
  "head_ref": "feat/auth",
  "base_ref": "main"
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "number": 1,
    "state": "open",
    ...
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `REPO_NOT_FOUND` (404)
- `PR_ALREADY_EXISTS` (409)
- `PR_INVALID_BASE` (400)
- `PR_INVALID_HEAD` (400)

**Auth required**: Yes (write permission on repo)

---

### GET /api/v1/repos/:owner/:repo/pulls

List pull requests.

**Query params**:
- `state`: "open" | "merged" | "closed" (default: "open")
- `limit`: Max PRs (default: 20)

**Response** (200 OK): Array of PR objects

**Auth required**: No (if public), Yes (if private)

---

### GET /api/v1/repos/:owner/:repo/pulls/:number

Get single PR.

**Response** (200 OK): PR object with full details

**Auth required**: No (if public), Yes (if private)

---

### PATCH /api/v1/repos/:owner/:repo/pulls/:number

Update PR (title, description, state).

**Request**:
```json
{
  "title": "Updated title",
  "state": "closed"
}
```

**Response** (200 OK): Updated PR object

**Auth required**: Yes (author or repo owner)

---

### POST /api/v1/repos/:owner/:repo/pulls/:number/reviews

Post a review (approve/request changes/comment).

**Request**:
```json
{
  "state": "approved",
  "body": "Looks good!"
}
```

**Response** (201 Created): Review object

**Auth required**: Yes

---

### POST /api/v1/repos/:owner/:repo/pulls/:number/comments

Post inline comment on diff.

**Request**:
```json
{
  "file_path": "src/auth.rs",
  "line_number": 42,
  "body": "This function looks suspicious"
}
```

**Response** (201 Created): Comment object

**Auth required**: Yes

---

### POST /api/v1/repos/:owner/:repo/pulls/:number/merge

Merge the pull request.

**Request**: Empty body (or merge strategy options in v2)

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "merged": true,
    "merge_commit_sha": "xyz...",
    "message": "Pull request merged"
  },
  "error": null,
  "metadata": { ... }
}
```

**Errors**:
- `PR_NOT_FOUND` (404)
- `PR_ALREADY_MERGED` (409)
- `PR_ALREADY_CLOSED` (410)
- `PR_MERGE_CONFLICT` (409)

**Auth required**: Yes (write permission)

---

## Organizations

### POST /api/v1/orgs

Create organization.

**Request**:
```json
{
  "name": "my-org",
  "display_name": "My Organization"
}
```

**Response** (201 Created): Organization object

**Auth required**: Yes

---

### GET /api/v1/orgs/:org

Get organization details.

**Response** (200 OK): Organization object

**Auth required**: No

---

### POST /api/v1/orgs/:org/members/:username

Add user to organization.

**Request**: Empty body

**Response** (201 Created): Membership object

**Errors**:
- `ORG_NOT_FOUND` (404)
- `USER_NOT_FOUND` (404)
- `ORG_PERMISSION_DENIED` (403)

**Auth required**: Yes (org owner)

---

### DELETE /api/v1/orgs/:org/members/:username

Remove user from organization.

**Response** (204 No Content)

**Auth required**: Yes (org owner)

---

### GET /api/v1/orgs/:org/members

List organization members.

**Response** (200 OK): Array of user objects

**Auth required**: No

---

## Teams

### POST /api/v1/orgs/:org/teams

Create team within organization.

**Request**:
```json
{
  "name": "backend",
  "display_name": "Backend Team"
}
```

**Response** (201 Created): Team object

**Auth required**: Yes (org owner)

---

### GET /api/v1/orgs/:org/teams

List teams in organization.

**Response** (200 OK): Array of team objects

**Auth required**: No

---

### PUT /api/v1/orgs/:org/teams/:team/repos/:owner/:repo

Grant team access to repository.

**Request**:
```json
{
  "permission": "read"
}
```

**Response** (200 OK): Team-repo assignment

**Auth required**: Yes (repo owner)

---

### POST /api/v1/orgs/:org/teams/:team/members/:username

Add user to team.

**Response** (201 Created): Team member object

**Auth required**: Yes (org owner)

---

## System

### GET /health

Health check endpoint.

**Response** (200 OK):
```json
{
  "status": "ok",
  "database": "ok",
  "disk": "ok",
  "uptime_seconds": 86400
}
```

**Auth required**: No
