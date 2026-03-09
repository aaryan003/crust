# CRUST API Reference

**Version**: v1  
**Base URL**: `http://localhost:8080/api/v1`  
**Auth**: `Authorization: Bearer <token>` (JWT)  
**Content-Type**: `application/json`

> All responses (success and error) are wrapped in `ApiResponse<T>`:
>
> ```json
> {
>   "success": true,
>   "data": { ... },
>   "error": null,
>   "metadata": {
>     "timestamp": "2026-03-05T10:00:00.000Z",
>     "duration": 12,
>     "request_id": "req-abc123"
>   }
> }
> ```
>
> On failure, `success: false`, `data: null`, and `error` contains `{ "code": "...", "message": "...", "field": null }`.

---

## System

### `GET /health`

Health check. No auth required.

**Response 200:**
```json
{
  "status": "ok",
  "database": "ok",
  "disk": "ok",
  "uptime_seconds": 86400
}
```

---

## Authentication

### `POST /api/v1/auth/register`

Create a new user account.

**Auth required:** No

**Request:**
```json
{
  "username": "alice",
  "email": "alice@example.com",
  "password": "SecurePass123!"
}
```

**Response 200:**
```json
{
  "success": true,
  "data": {
    "user": {
      "id": "62cba3c1-82fe-4857-b05a-534c007fe318",
      "username": "alice",
      "email": "alice@example.com",
      "display_name": null,
      "is_active": true,
      "created_at": "2026-03-05T00:00:00Z"
    },
    "token": "eyJ...",
    "expires_at": "2026-03-06T00:00:00Z"
  },
  "error": null,
  "metadata": { "timestamp": "...", "duration": 45, "request_id": null }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `VALIDATE_USERNAME_TAKEN` | 409 | Username already in use |
| `VALIDATE_EMAIL_TAKEN` | 409 | Email already registered |
| `VALIDATE_WEAK_PASSWORD` | 400 | Password < 12 chars or lacks variety |
| `VALIDATE_INVALID_EMAIL` | 400 | Malformed email address |
| `AUTH_REGISTRATION_DISABLED` | 403 | Server has `ALLOW_REGISTRATION=false` |

---

### `POST /api/v1/auth/login`

Authenticate and receive a JWT.

**Auth required:** No

**Request:**
```json
{
  "username": "alice",
  "password": "SecurePass123!"
}
```

**Response 200:** Same shape as `/register` response.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `AUTH_INVALID_CREDENTIALS` | 401 | Wrong password |
| `AUTH_USER_NOT_FOUND` | 401 | Username doesn't exist |
| `AUTH_ACCOUNT_DISABLED` | 403 | Account has been disabled |

---

### `POST /api/v1/auth/logout`

Revoke current token (invalidates the JWT server-side).

**Auth required:** Yes

**Request:** Empty body

**Response 204:** No content

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `AUTH_MISSING_HEADER` | 401 | No `Authorization` header |
| `AUTH_TOKEN_INVALID` | 401 | Malformed token |

---

### `GET /api/v1/auth/me`

Get the currently authenticated user's profile.

**Auth required:** Yes

**Response 200:**
```json
{
  "success": true,
  "data": {
    "id": "62cba3c1-...",
    "username": "alice",
    "email": "alice@example.com",
    "display_name": "Alice Smith",
    "is_active": true,
    "created_at": "2026-03-01T10:00:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `AUTH_MISSING_HEADER` | 401 | No token |
| `AUTH_TOKEN_EXPIRED` | 401 | Token past expiry |
| `AUTH_TOKEN_REVOKED` | 401 | Token was logged out |

---

## Users

### `GET /api/v1/users/:username`

Get a user's public profile.

**Auth required:** No

**Response 200:**
```json
{
  "success": true,
  "data": {
    "id": "...",
    "username": "alice",
    "display_name": "Alice Smith",
    "created_at": "2026-03-01T10:00:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `USER_NOT_FOUND` | 404 | No user with that username |

---

### `PATCH /api/v1/users/me`

Update own display name or password.

**Auth required:** Yes

**Request:** (all fields optional)
```json
{
  "display_name": "Alice J. Smith",
  "password": "NewSecurePass456!"
}
```

**Response 200:** Updated user object.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `VALIDATE_WEAK_PASSWORD` | 400 | New password too weak |
| `AUTH_MISSING_HEADER` | 401 | No token |

---

## Repositories

### `POST /api/v1/repos`

Create a new repository owned by the authenticated user.

**Auth required:** Yes

**Request:**
```json
{
  "name": "my-project",
  "display_name": "My Project",
  "description": "A cool CRUST project",
  "is_public": false,
  "default_branch": "main"
}
```

**Response 201:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "owner_id": "uuid",
    "name": "my-project",
    "display_name": "My Project",
    "description": "A cool CRUST project",
    "is_public": false,
    "default_branch": "main",
    "created_at": "2026-03-05T10:00:00Z",
    "updated_at": "2026-03-05T10:00:00Z"
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `REPO_ALREADY_EXISTS` | 409 | Same owner + name already exists |
| `REPO_NAME_INVALID` | 400 | Name contains invalid chars |
| `VALIDATE_REQUIRED_FIELD` | 400 | `name` missing |

---

### `GET /api/v1/repos/:owner/:repo`

Get repository metadata.

**Auth required:** No (public repos), Yes (private repos)

**Example:** `GET /api/v1/repos/alice/my-project`

**Response 200:** Repository object (same shape as create response).

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `REPO_NOT_FOUND` | 404 | Repo doesn't exist |
| `REPO_PERMISSION_DENIED` | 403 | Private repo, no access |

---

### `PATCH /api/v1/repos/:owner/:repo`

Update repository settings (owner only).

**Auth required:** Yes

**Request:** (all fields optional)
```json
{
  "display_name": "New Display Name",
  "description": "Updated description",
  "is_public": true
}
```

**Response 200:** Updated repository object.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `REPO_NOT_FOUND` | 404 | Repo doesn't exist |
| `REPO_PERMISSION_DENIED` | 403 | Not the owner |

---

### `DELETE /api/v1/repos/:owner/:repo`

Delete repository (owner only).

**Auth required:** Yes

**Response 204:** No content.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `REPO_NOT_FOUND` | 404 | Repo doesn't exist |
| `REPO_PERMISSION_DENIED` | 403 | Not the owner |

---

### `GET /api/v1/repos/:owner/:repo/refs`

List all branches and tags.

**Auth required:** No (public), Yes (private)

**Response 200:**
```json
{
  "success": true,
  "data": {
    "heads": {
      "main": "d74320634a5019eaeb4bfd5444cbcd041f2d283...",
      "feat/auth": "31e84afda8b8b44b855..."
    },
    "tags": {}
  },
  "error": null,
  "metadata": { ... }
}
```

---

## Object Transport

> These endpoints are used internally by `crust push`, `crust fetch`, and `crust clone`. Direct use requires understanding the CRUSTPACK wire format.

---

### `POST /api/v1/repos/:owner/:repo/refs/preflight`

Tell the server which objects you want and which you already have.

**Auth required:** Yes

**Request:**
```json
{
  "wants": ["sha256hex..."],
  "haves": ["sha256hex...", "sha256hex..."]
}
```

**Response 200:**
```json
{
  "success": true,
  "data": {
    "wants": ["sha256hex..."],
    "haves": ["sha256hex..."]
  },
  "error": null,
  "metadata": { ... }
}
```

---

### `POST /api/v1/repos/:owner/:repo/objects/fetch`

Download objects in **CRUSTPACK binary format**.

**Auth required:** Yes

**Request (JSON):**
```json
{
  "wants": ["sha256hex..."],
  "haves": []
}
```

**Response 200:** Binary CRUSTPACK data (`Content-Type: application/octet-stream`)

The server performs full graph traversal: given a commit SHA, it returns the commit + all its parent commits + all trees + all blobs.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `PACK_EMPTY` | 400 | No objects found for requested IDs |
| `OBJECT_NOT_FOUND` | 404 | Requested SHA doesn't exist |

---

### `POST /api/v1/repos/:owner/:repo/objects/upload`

Upload objects in **CRUSTPACK binary format**.

**Auth required:** Yes (must have write permission on repo)

**Request:** Raw CRUSTPACK bytes (`Content-Type: application/octet-stream`)

**Response 200:**
```json
{
  "success": true,
  "data": {
    "objects_stored": 8,
    "conflicts": []
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `PACK_MALFORMED` | 400 | Invalid CRUSTPACK header |
| `PACK_CHECKSUM_MISMATCH` | 422 | SHA256 trailer doesn't match |
| `REPO_PERMISSION_DENIED` | 403 | Not authorized to push |
| `REPO_NOT_FOUND` | 404 | Repo doesn't exist |

---

### `POST /api/v1/repos/:owner/:repo/refs/update`

Update branch refs after uploading objects (atomic).

**Auth required:** Yes

**Request:**
```json
{
  "updates": [
    {
      "ref_name": "refs/heads/main",
      "old_sha": "31e84afda8b8...",
      "new_sha": "d74320634a50..."
    }
  ]
}
```

**Response 200:**
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

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `REF_CONFLICT` | 409 | `old_sha` doesn't match current value (diverged) |
| `REPO_PERMISSION_DENIED` | 403 | No write access |

---

## Pull Requests

### `POST /api/v1/repos/:owner/:repo/pulls`

Create a pull request.

**Auth required:** Yes

**Request:**
```json
{
  "title": "Add authentication system",
  "description": "Implements JWT-based login/register endpoints",
  "head_ref": "feat/auth",
  "base_ref": "main"
}
```

**Response 201:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "number": 1,
    "title": "Add authentication system",
    "description": "...",
    "state": "open",
    "head_ref": "feat/auth",
    "base_ref": "main",
    "author_id": "uuid",
    "created_at": "2026-03-05T10:00:00Z",
    "updated_at": "2026-03-05T10:00:00Z",
    "merged_at": null,
    "merge_commit_sha": null
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `PR_ALREADY_EXISTS` | 409 | Open PR for same head→base already exists |
| `PR_INVALID_BASE` | 400 | `base_ref` doesn't exist |
| `PR_INVALID_HEAD` | 400 | `head_ref` doesn't exist |

---

### `GET /api/v1/repos/:owner/:repo/pulls`

List pull requests.

**Auth required:** No (public), Yes (private)

**Query params:**
| Param | Values | Default |
|-------|--------|---------|
| `state` | `open` \| `merged` \| `closed` | `open` |
| `limit` | integer | `20` |

**Response 200:** Array of PR objects.

---

### `GET /api/v1/repos/:owner/:repo/pulls/:number`

Get a single pull request by number.

**Auth required:** No (public), Yes (private)

**Response 200:** Full PR object.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `PR_NOT_FOUND` | 404 | No PR with that number |

---

### `PATCH /api/v1/repos/:owner/:repo/pulls/:number`

Update a pull request (author or repo owner only).

**Auth required:** Yes

**Request:** (all optional)
```json
{
  "title": "Updated title",
  "description": "Updated description",
  "state": "closed"
}
```

**Response 200:** Updated PR object.

---

### `POST /api/v1/repos/:owner/:repo/pulls/:number/reviews`

Post a review on a pull request.

**Auth required:** Yes

**Request:**
```json
{
  "state": "approved",
  "body": "Looks good to me!"
}
```

`state` values: `"approved"` | `"changes_requested"` | `"commented"`

**Response 201:** Review object.

---

### `POST /api/v1/repos/:owner/:repo/pulls/:number/comments`

Post an inline comment on a specific line of the diff.

**Auth required:** Yes

**Request:**
```json
{
  "file_path": "src/auth.rs",
  "line_number": 42,
  "body": "This should return a Result, not panic."
}
```

**Response 201:** Comment object.

---

### `POST /api/v1/repos/:owner/:repo/pulls/:number/merge`

Merge a pull request.

**Auth required:** Yes (write permission on repo)

**Request:** Empty body `{}`

**Response 200:**
```json
{
  "success": true,
  "data": {
    "merged": true,
    "merge_commit_sha": "abc1234def567890...",
    "message": "Pull request merged"
  },
  "error": null,
  "metadata": { ... }
}
```

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `PR_ALREADY_MERGED` | 409 | PR already in merged state |
| `PR_ALREADY_CLOSED` | 410 | PR was closed without merging |
| `PR_MERGE_CONFLICT` | 409 | Branches have conflicting changes |

---

## Organizations

### `POST /api/v1/orgs`

Create an organization.

**Auth required:** Yes

**Request:**
```json
{
  "name": "acme-corp",
  "display_name": "Acme Corporation"
}
```

**Response 201:** Organization object.

---

### `GET /api/v1/orgs/:org`

Get organization details.

**Auth required:** No

**Response 200:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "name": "acme-corp",
    "display_name": "Acme Corporation",
    "owner_id": "uuid",
    "created_at": "..."
  },
  "error": null,
  "metadata": { ... }
}
```

---

### `POST /api/v1/orgs/:org/members/:username`

Add a user to an organization (org owner only).

**Auth required:** Yes

**Request:** Empty body `{}`

**Response 201:** Membership object.

**Error codes:**
| Code | HTTP | When |
|------|------|------|
| `ORG_NOT_FOUND` | 404 | Organization doesn't exist |
| `USER_NOT_FOUND` | 404 | User doesn't exist |
| `ORG_PERMISSION_DENIED` | 403 | Not the org owner |

---

### `DELETE /api/v1/orgs/:org/members/:username`

Remove a user from an organization.

**Auth required:** Yes (org owner)

**Response 204:** No content.

---

### `GET /api/v1/orgs/:org/members`

List all members of an organization.

**Auth required:** No

**Response 200:** Array of user objects.

---

## Teams

### `POST /api/v1/orgs/:org/teams`

Create a team within an organization.

**Auth required:** Yes (org owner)

**Request:**
```json
{
  "name": "backend",
  "display_name": "Backend Team"
}
```

**Response 201:** Team object.

---

### `GET /api/v1/orgs/:org/teams`

List all teams in an organization.

**Auth required:** No

**Response 200:** Array of team objects.

---

### `PUT /api/v1/orgs/:org/teams/:team/repos/:owner/:repo`

Grant a team access to a repository.

**Auth required:** Yes (repo owner)

**Request:**
```json
{
  "permission": "read"
}
```

`permission` values: `"read"` | `"write"` | `"admin"`

**Response 200:** Team-repo assignment object.

---

### `POST /api/v1/orgs/:org/teams/:team/members/:username`

Add a user to a team.

**Auth required:** Yes (org owner)

**Request:** Empty body `{}`

**Response 201:** Team member object.

---

## Error Reference

All errors follow this shape:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "AUTH_INVALID_CREDENTIALS",
    "message": "Invalid username or password",
    "field": null
  },
  "metadata": {
    "timestamp": "2026-03-05T10:00:00.000Z",
    "duration": 12,
    "request_id": null
  }
}
```

### Full Error Code Table

| Code | HTTP | Category |
|------|------|----------|
| `AUTH_INVALID_CREDENTIALS` | 401 | Auth |
| `AUTH_USER_NOT_FOUND` | 401 | Auth |
| `AUTH_ACCOUNT_DISABLED` | 403 | Auth |
| `AUTH_TOKEN_EXPIRED` | 401 | Auth |
| `AUTH_TOKEN_INVALID` | 401 | Auth |
| `AUTH_TOKEN_REVOKED` | 401 | Auth |
| `AUTH_MISSING_HEADER` | 401 | Auth |
| `AUTH_REGISTRATION_DISABLED` | 403 | Auth |
| `VALIDATE_REQUIRED_FIELD` | 400 | Validation |
| `VALIDATE_INVALID_EMAIL` | 400 | Validation |
| `VALIDATE_WEAK_PASSWORD` | 400 | Validation |
| `VALIDATE_USERNAME_TAKEN` | 409 | Validation |
| `VALIDATE_EMAIL_TAKEN` | 409 | Validation |
| `REPO_NOT_FOUND` | 404 | Repo |
| `REPO_ALREADY_EXISTS` | 409 | Repo |
| `REPO_PERMISSION_DENIED` | 403 | Repo |
| `REPO_NAME_INVALID` | 400 | Repo |
| `OBJECT_NOT_FOUND` | 404 | Objects |
| `OBJECT_CORRUPT` | 422 | Objects |
| `PACK_MALFORMED` | 400 | Pack |
| `PACK_CHECKSUM_MISMATCH` | 422 | Pack |
| `PACK_EMPTY` | 400 | Pack |
| `REF_NOT_FOUND` | 404 | Refs |
| `REF_CONFLICT` | 409 | Refs |
| `PR_NOT_FOUND` | 404 | PRs |
| `PR_ALREADY_MERGED` | 409 | PRs |
| `PR_MERGE_CONFLICT` | 409 | PRs |
| `ORG_NOT_FOUND` | 404 | Orgs |
| `ORG_PERMISSION_DENIED` | 403 | Orgs |
| `SERVER_INTERNAL_ERROR` | 500 | Server |
| `SERVER_DISK_FULL` | 507 | Server |
