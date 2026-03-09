# Error Codes Specification

VERSION: 1.0.0
WRITTEN_BY: contracts-agent
CONSUMED_BY: all agents, error-codes.md, contracts/data-types.rs
LAST_UPDATED: 2026-03-04

All error codes are UPPER_SNAKE_CASE strings. These codes are used in:
- API responses (ApiError.code field)
- CLI error output
- Server logs (for searchability)

---

## Authentication Errors (auth_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| AUTH_INVALID_CREDENTIALS | 401 | Invalid username or password | POST /api/v1/auth/login, bad credentials |
| AUTH_USER_NOT_FOUND | 401 | User not found | POST /api/v1/auth/login, username doesn't exist |
| AUTH_ACCOUNT_DISABLED | 403 | Account has been disabled | User.is_active = false |
| AUTH_TOKEN_EXPIRED | 401 | Token has expired | JWT exp < now |
| AUTH_TOKEN_INVALID | 401 | Token is invalid or malformed | JWT decode error |
| AUTH_TOKEN_REVOKED | 401 | Token has been revoked | Token in revoked_tokens table |
| AUTH_MISSING_HEADER | 401 | Authorization header missing | No Bearer token |
| AUTH_INVALID_HEADER | 401 | Authorization header malformed | Not "Bearer {token}" format |
| AUTH_REGISTRATION_DISABLED | 403 | Registration is disabled | ALLOW_REGISTRATION env var = false |

---

## Validation Errors (validate_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| VALIDATE_REQUIRED_FIELD | 400 | Required field missing | e.g., username in POST /register |
| VALIDATE_INVALID_EMAIL | 400 | Email format invalid | Not a valid email |
| VALIDATE_WEAK_PASSWORD | 400 | Password does not meet requirements | < 12 chars, no variety |
| VALIDATE_USERNAME_TAKEN | 409 | Username is already taken | Duplicate username |
| VALIDATE_EMAIL_TAKEN | 409 | Email is already taken | Duplicate email |
| VALIDATE_INVALID_FORMAT | 400 | Input format invalid | e.g., invalid SHA256 hex |
| VALIDATE_STRING_TOO_LONG | 400 | String exceeds max length | e.g., title > 500 chars |
| VALIDATE_INVALID_ENUM | 400 | Invalid enum value | e.g., state not in [open, merged, closed] |

---

## Repository Errors (repo_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| REPO_NOT_FOUND | 404 | Repository not found | Repo doesn't exist |
| REPO_ALREADY_EXISTS | 409 | Repository already exists | owner + name conflict |
| REPO_PERMISSION_DENIED | 403 | You don't have permission to access this repo | User role too low |
| REPO_PRIVATE | 403 | Repository is private | User not authenticated, repo not public |
| REPO_NAME_INVALID | 400 | Repository name invalid | Name doesn't match pattern |
| REPO_FORK_FAILED | 500 | Failed to fork repository | Internal error during fork |

---

## Object Storage Errors (object_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| OBJECT_NOT_FOUND | 404 | Object not found | SHA256 object doesn't exist on server |
| OBJECT_CORRUPT | 422 | Object is corrupt or invalid | Decompression/validation failed |
| OBJECT_INVALID_HEADER | 400 | Object header is malformed | Doesn't match CRUST-OBJECT format |
| OBJECT_INVALID_FORMAT | 400 | Object format is invalid | Type, size, or content invalid |
| OBJECT_CHECKSUM_MISMATCH | 422 | Object checksum doesn't match ID | SHA256(object) != claimed ID |
| OBJECT_ALREADY_EXISTS | 409 | Object with this ID already exists | Idempotent re-upload (OK) |
| OBJECT_SIZE_EXCEEDED | 413 | Object exceeds maximum size | > server limit |

---

## Pack Errors (pack_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| PACK_MALFORMED | 400 | Pack format is malformed | Missing CRUSTPACK header, invalid structure |
| PACK_UNSUPPORTED_VERSION | 400 | Pack version not supported | version field != 1 |
| PACK_TRUNCATED | 400 | Pack is incomplete | Fewer bytes than declared |
| PACK_CHECKSUM_MISMATCH | 422 | Pack checksum verification failed | SHA256(pack) != trailer |
| PACK_EMPTY | 400 | Pack contains no objects | count: 0 |

---

## Reference (Branch/Tag) Errors (ref_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| REF_NOT_FOUND | 404 | Reference not found | Branch or tag doesn't exist |
| REF_ALREADY_EXISTS | 409 | Reference already exists | Branch name taken |
| REF_INVALID_NAME | 400 | Reference name invalid | e.g., contains spaces |
| REF_LOCKED | 423 | Reference is locked | Push in progress |
| REF_CONFLICT | 409 | Reference update conflict | Old SHA doesn't match (non-force push) |
| REF_PROTECTED | 403 | Cannot force-push to protected branch | (v2 feature) |
| REF_INVALID_SHA | 400 | Invalid or missing commit SHA | Not valid SHA256 hex |

---

## Pull Request Errors (pr_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| PR_NOT_FOUND | 404 | Pull request not found | PR number doesn't exist |
| PR_ALREADY_EXISTS | 409 | Pull request already exists | Same source/target branch |
| PR_INVALID_BASE | 400 | Invalid base branch | Base branch doesn't exist |
| PR_INVALID_HEAD | 400 | Invalid head branch | Head branch doesn't exist |
| PR_MERGE_CONFLICT | 409 | Cannot merge: conflicts detected | Requires manual resolution |
| PR_ALREADY_MERGED | 409 | PR already merged | Cannot merge again |
| PR_ALREADY_CLOSED | 410 | PR is closed | Cannot reopen (v2 feature) |

---

## Organization Errors (org_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| ORG_NOT_FOUND | 404 | Organization not found | Org doesn't exist |
| ORG_ALREADY_EXISTS | 409 | Organization already exists | Name taken |
| ORG_PERMISSION_DENIED | 403 | You don't have permission for this org | Not owner/member |
| ORG_NAME_INVALID | 400 | Organization name invalid | Doesn't match pattern |

---

## Team Errors (team_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| TEAM_NOT_FOUND | 404 | Team not found | Team doesn't exist |
| TEAM_ALREADY_EXISTS | 409 | Team already exists | Name taken within org |
| TEAM_PERMISSION_DENIED | 403 | You don't have permission for this team | Not member |

---

## User Errors (user_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| USER_NOT_FOUND | 404 | User not found | Username doesn't exist |
| USER_ALREADY_EXISTS | 409 | User already exists | Username or email conflict |
| USER_PERMISSION_DENIED | 403 | You don't have permission to modify this user | Not self or admin |

---

## Server Errors (server_*)

| Code | HTTP Status | Message | Context |
|------|-------------|---------|---------|
| SERVER_INTERNAL_ERROR | 500 | Internal server error | Unhandled panic/exception |
| SERVER_DATABASE_ERROR | 500 | Database error | Query failed |
| SERVER_DISK_ERROR | 500 | Disk error | I/O failure reading/writing objects |
| SERVER_DISK_FULL | 507 | Disk is full | No space for object storage |
| SERVER_TIMEOUT | 504 | Request timeout | Long-running operation exceeded limit |
| SERVER_MAINTENANCE | 503 | Server is in maintenance | Temporary downtime |

---

## CLI-Specific Errors (cli_*)

(Only in client binary output, never in API responses)

| Code | Message | Context |
|------|---------|---------|
| CLI_NO_REPOSITORY | Not a CRUST repository | .crust/ not found |
| CLI_NOT_AUTHENTICATED | Not authenticated | ~/.crust/credentials missing or expired |
| CLI_NETWORK_ERROR | Network error | Cannot reach server |
| CLI_INVALID_ARGUMENT | Invalid argument | Bad command-line usage |
| CLI_WORKING_TREE_DIRTY | Working tree has uncommitted changes | Cannot switch branches |
| CLI_MERGE_IN_PROGRESS | Merge in progress | Cannot commit until merge resolved |
| CLI_CONFLICT_MARKERS | Conflict markers found | Cannot commit file with conflict markers |

---

## Usage in Responses

All API error responses are wrapped:

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
    "timestamp": "2026-03-04T10:30:45.123456Z",
    "duration": 42,
    "request_id": "req-abc123"
  }
}
```

Validation errors can include a `field`:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATE_WEAK_PASSWORD",
    "message": "Password does not meet requirements",
    "field": "password"
  },
  "metadata": { ... }
}
```

---

## Mapping to HTTP Status

Standard mapping (not all codes map to all statuses):

- **400 Bad Request**: Input validation, format errors
- **401 Unauthorized**: Authentication required, invalid/expired token
- **403 Forbidden**: Authenticated but permission denied
- **404 Not Found**: Resource doesn't exist
- **409 Conflict**: Collision (duplicate resource, ref conflict, merge conflict)
- **413 Payload Too Large**: Upload size limit
- **422 Unprocessable Entity**: Semantic validation (corrupt object, checksum mismatch)
- **423 Locked**: Resource temporarily locked
- **500 Internal Server Error**: Unrecoverable server error
- **503 Service Unavailable**: Maintenance or overload
- **504 Gateway Timeout**: Long operation exceeded limit

---

## Future Extensions

As CRUST grows, add new error codes here with proper documentation rather than inventing them in code.
