# CRUST Architecture

**Last Updated**: 2026-03-04  
**Version**: 2.0

## Overview

CRUST is a fully original, self-hosted version control system built from scratch in Rust. It is NOT git and is NOT git-compatible.

### Why NOT Git?

- Git's data model (SHA1, zlib, pkt-line) is from 2005
- Git's SSH-based auth is complex for hosting
- CRUST intentionally uses modern alternatives (SHA256, zstd, HTTPS+JWT)
- CRUST is simpler, faster, and more aligned with modern infrastructure

### Key Distinctions

| Aspect | Git | CRUST |
|--------|-----|-------|
| Hash Algorithm | SHA1 (160-bit) | SHA256 (256-bit) |
| Compression | zlib | zstd (faster, better ratio) |
| Transport | SSH + pkt-line | HTTPS + CRUSTPACK |
| Auth | SSH keys | JWT tokens |
| CLI Command | `git` | `crust` |
| Format | Git format (incompatible) | CRUST format (original) |
| License | GPL-2.0 | TBD |

---

## Three-Crate Architecture

CRUST is organized as a Cargo workspace with 3 crates:

### 1. `gitcore` — Pure VCS Library

**Purpose**: Immutable, reusable VCS object model

**Characteristics**:
- ✅ Zero external dependencies (only sha2, zstd, hex, thiserror)
- ✅ Zero async I/O
- ✅ Zero database access
- ✅ Zero network calls
- ✅ Fully deterministic (same input → same output always)

**Contents**:
- Object types: `Blob`, `Tree`, `Commit`, `Tag`
- Object serialization/deserialization (per contracts/object-format.md)
- SHA256 hashing (always)
- Merge algorithm (3-way merge, conflict detection)
- Error types

**Testing**: `cargo test -p gitcore` requires no external services (pure unit tests)

**Used by**: Both `crust-server` (via library dependency) and `crust-cli` (via library dependency)

### 2. `crust-server` — HTTP API + Object Storage

**Purpose**: Hosting platform (repos, users, permissions, PRs, orgs, teams)

**Characteristics**:
- ✅ Tokio async runtime
- ✅ Axum HTTP framework
- ✅ PostgreSQL via sqlx (compile-time checked queries)
- ✅ Disk-based object storage
- ✅ JWT authentication
- ✅ Production-grade logging/tracing

**Contents**:
- HTTP routes (all endpoints from contracts/api-contracts.md)
- Database layer (PostgreSQL schema from contracts/db-schema.md)
- Permission system (owner/write/read hierarchy)
- Object transport (CRUSTPACK pack upload/fetch)
- Organization and team management
- Pull request functionality

**REST API**: Base path `/api/v1`

**Authentication**: JWT Bearer tokens (from Authorization header)

**Storage Layout**:
```
/data/repos/
└── {owner_id}/                  (UUID)
    └── {repo_id}.crust/         (UUID + .crust)
        ├── objects/
        │   └── {2char}/{62char}
        └── refs/
            ├── heads/
            └── tags/
```

**Testing**: `cargo test -p crust-server` (requires test PostgreSQL)

**Deployment**: Single Docker container + PostgreSQL container

### 3. `crust-cli` — Command-Line Client

**Purpose**: User-facing VCS client (replaces `git`)

**Characteristics**:
- ✅ Standalone binary
- ✅ Clap for argument parsing
- ✅ HTTPS client (reqwest)
- ✅ Local VCS operations (using gitcore)
- ✅ Remote sync (fetch/push to server)

**Contents**:
- Command implementations: `crust init`, `crust commit`, `crust push`, etc.
- Local config (~/.crust/config)
- Credentials management (~/.crust/credentials, JSON format)
- HTTP client with JWT auth
- Working tree operations (add, diff, status, commit)
- Branching (branch, checkout, merge)
- Remote sync (clone, fetch, pull, push)

**Commands**: Full spec in contracts/cli-commands.md

**Exit Codes**:
- 0: Success
- 1: User error (bad args, missing repo, etc.)
- 2: Runtime error (network, disk, server error)

**Testing**: `cargo test -p crust-cli` (can use mock server or integration tests)

---

## System Boundaries (Contracts)

The `contracts/` directory defines all system boundaries BEFORE code is written.

### Key Contract Files

| File | Purpose | Written By | Read By |
|------|---------|-----------|---------|
| `data-types.rs` | Shared types (User, Repo, Org, types, PR, etc.) | contracts-agent | All crates |
| `object-format.md` | CRUST object spec (SHA256, zstd, headers) | contracts-agent | gitcore |
| `crustpack-format.md` | Wire protocol for object transport | contracts-agent | server + cli |
| `db-schema.md` | PostgreSQL schema (tables, indexes, FKs) | contracts-agent | server |
| `error-codes.md` | All error codes (UPPER_SNAKE_CASE) | contracts-agent | All agents |
| `api-contracts.md` | Every HTTP endpoint (method, path, request, response, errors) | backend-agent | cli + frontend (v2) |
| `cli-commands.md` | Every CLI command (arguments, behavior, output, exit codes) | cli-agent | cli implementation |

**Rule**: Before writing any code, check the contract. If it's missing, write it first.

---

## Object Format (CRUST-Specific)

Every object in CRUST follows this exact format (see `contracts/object-format.md`):

### Disk Storage
```
.crust/objects/{id[0..2]}/{id[2..64]}
```

Content on disk: `zstd(header + raw_content)`

### Object Header
```
CRUST-OBJECT\n
type: {blob|tree|commit|tag}\n
size: {uncompressed_byte_length}\n
\n
{raw content bytes}
```

### Object ID
SHA256 hash of `(header + content)` = 64 lowercase hex characters

### Types

**Blob**: Raw file content (no interpretation)

**Tree**: Directory listing, binary format
```
{mode} {name}\0{32_raw_sha256_bytes}  [repeating, sorted by name]
```

**Commit**: Text format with metadata
```
tree {sha256_hex}
parent {sha256_hex}  [zero or more for merge]
author {name} <{email}> {unix_ts} {tz_offset}
committer {name} <{email}> {unix_ts} {tz_offset}

{commit message — rest of content}
```

**Tag**: Annotated tag (text format)
```
object {sha256_hex}
type {blob|tree|commit|tag}
tag {tag_name}
tagger {name} <{email}> {unix_ts} {tz_offset}

{tag message}
```

---

## Wire Protocol (CRUSTPACK)

Objects are transmitted in CRUSTPACK format (see `contracts/crustpack-format.md`):

### Format
```
CRUSTPACK\n
version: 1\n
count: {object_count}\n
\n
id: {sha256_hex}\n
type: {blob|tree|commit|tag}\n
size: {compressed_byte_count}\n
{size bytes of zstd-compressed object}
[repeat per object]
{32 bytes: SHA256 of all preceding pack bytes}
```

### Usage
- Client uploads objects to server: `POST /api/v1/repos/{owner}/{repo}/objects/upload`
- Server sends objects to client: `POST /api/v1/repos/{owner}/{repo}/objects/fetch`

### Validation
- Server/client verifies pack header (CRUSTPACK magic, version, count)
- Each object decompressed and validated
- Trailer SHA256 checksum verified

---

## Database Schema (PostgreSQL 16)

Key tables (see `contracts/db-schema.md`):

- `users`: Platform users (username, email, password_hash)
- `repositories`: Repos with ownership (owner_id, name, is_public)
- `repo_permissions`: Explicit access grants (user → repo, read/write/owner)
- `organizations`: Groups for managing repos/teams
- `org_members`: Membership in organizations
- `teams`: Groups within organizations
- `team_members`: Membership in teams
- `team_repos`: Team access to repos
- `pull_requests`: PRs with metadata (state, merge commit)
- `pr_reviews`: Code reviews
- `pr_comments`: Inline comments on diffs
- `revoked_tokens`: Blacklist for JWT revocation

**Important**: Objects are stored on disk, not in database. Database stores references (SHA256 hashes) and metadata only.

---

## Permission Hierarchy

Permission resolution (highest to lowest):

1. **Repo Owner** (from repositories.owner_id)
   - Full access (admin)
   
2. **Org Owner** (if repo owned by org, user is org owner)
   - Full access to org repos
   
3. **Team Permission** (via team_repos)
   - Explicit read/write to repos assigned to team
   
4. **Direct Permission** (via repo_permissions)
   - Explicit read/write/owner grant to user
   
5. **Public Repo**
   - Read-only access for all authenticated users
   
6. **No Access**
   - 403 Forbidden

---

## Error Handling Pattern

All API responses use this wrapper (see `contracts/data-types.rs`):

```rust
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

pub struct ApiError {
    pub code: String,       // "UPPER_SNAKE_CASE" from error-codes.md
    pub message: String,    // human-readable
    pub field: Option<String>,  // for validation errors
}
```

### HTTP Status Mapping
- 400: Input validation error
- 401: Auth required or invalid
- 403: Authenticated but permission denied
- 404: Resource not found
- 409: Conflict (duplicate, ref conflict, merge conflict)
- 422: Semantic error (corrupt object, checksum mismatch)
- 500: Internal server error
- 503: Service unavailable

All error codes are defined in `contracts/error-codes.md` — never invent new ones mid-implementation.

---

## CLI Paradigm

Users interact via `crust` commands (NOT `git`).

### Bootstrap
```bash
crust init                    # Initialize local repo
crust login https://...       # Authenticate with server
```

### Local Operations
```bash
crust status                  # Show working tree state
crust add src/main.rs         # Stage file
crust diff                    # Show unstaged changes
crust commit -m "msg"         # Create commit
```

### Branching
```bash
crust branch feat/foo         # Create branch
crust checkout feat/foo       # Switch branch
crust merge main              # Merge branch (3-way)
```

### Remote Sync
```bash
crust clone https://.../repo  # Clone repository
crust fetch                   # Fetch objects
crust push                    # Push commits
crust pull                    # Fetch + merge
```

### History
```bash
crust log                     # Show commit history
crust show main               # Show commit + diff
```

---

## Development Workflow (Contract-First)

1. **Read Contract** — Every boundary is defined before code
2. **Implement** — Write code that conforms to contract
3. **Test** — Verify all acceptance criteria
4. **Handoff** — Generate handoff note for next task
5. **Repeat** — Move to next task

All work is tracked in `reasoning/task-breakdown.md` (17 tasks, 4 phases).

Key files:
- `.github/copilot-instructions.md` — Always-on system context
- `.github/agents/` — Agent personas
- `.github/prompts/` — Reusable task templates
- `contracts/` — Single source of truth
- `reasoning/task-breakdown.md` — Task tracking
- `reasoning/learning.md` — Architectural decisions

---

## Deployment

### Docker Compose
```bash
docker-compose up -d
```

### Environment Variables
```
DATABASE_URL=postgres://user:pass@db:5432/crust
JWT_SECRET=... (min 64 random chars)
REPO_BASE_PATH=/data/repos
PORT=8080
JWT_EXPIRY_SECONDS=86400
LOG_LEVEL=info
ALLOW_REGISTRATION=true
```

### Health Check
```bash
curl http://localhost:8080/health
```

---

## Key Principles

1. **Contract-First**: Contracts before code
2. **No Git Compatibility**: Intentionally incompatible
3. **Pure Gitcore**: Server and CLI both use gitcore library
4. **Single Truth**: contracts/ is authoritative
5. **Deterministic**: Objects serialize identically every time
6. **Type-Safe**: Rust + sqlx compile-time checks
7. **Error Codes**: All errors defined upfront
8. **Async Where It Matters**: gitcore is sync, server is async
9. **Tested**: Full test suite before marking tasks complete
10. **Documented**: Every public API has docs

---

## What Makes CRUST Different

| Aspect | CRUST | Git |
|--------|-------|-----|
| **Data Model** | Original design | 20-year-old design |
| **Hashing** | SHA256 (256-bit, modern) | SHA1 (160-bit, deprecated) |
| **Compression** | zstd (fast, modern) | zlib (from 2000s) |
| **Transport** | HTTPS + JWT (cloud-native) | SSH + pkt-line (complex) |
| **Hosting** | Native SaaS support | Requires server setup |
| **UI** | Roadmap (v2+) | No built-in UI |
| **Merging** | 3-way merge (clean conflicts) | Similar to git |
| **Immutability** | Objects are immutable | Objects are immutable |
| **Branching** | Native lightweight branches | Native lightweight branches |
| **Performance** | Optimized for modern hardware | From 2005 era |

---

## Future Roadmap

### v0.1 (Current)
- ✅ Core VCS operations (commit, branch, merge, push, pull)
- ✅ User authentication (JWT)
- ✅ Repositories (create, list, public/private)
- ✅ Organizations and teams
- ✅ Pull requests (create, review, merge)

### v0.2 (Planned)
- Web UI for repository browsing
- Webhooks and CI/CD integration
- Code search across repositories
- Advanced PR features (branch protection, required reviews)

### v1.0 (Future)
- Rebase support
- Submodules
- Sparse checkout
- Pack file delta compression
- Commit signing
- Advanced analytics

---

## End of Architecture Document

For implementation details, see specific contracts in `contracts/`.  
For development instructions, see `WORKFLOW.md`.  
For task tracking, see `reasoning/task-breakdown.md`.
