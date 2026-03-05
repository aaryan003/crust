# CRUST Copilot Instructions

**Last Updated**: 2026-03-04  
**Version**: 1.0.0  
**Model**: Claude via GitHub Copilot

---

## PRODUCT OVERVIEW

**CRUST** is a fully original self-hosted version control system built entirely from scratch in Rust.

It is **NOT git**. It is NOT git-compatible. It has:
- Its own object format (SHA256 + zstd, not SHA1 + zlib)
- Its own CLI client (users type `crust`, not `git`)
- Its own wire protocol (HTTPS + JWT, no SSH, no pkt-line)
- Its own hosting platform (accounts, repos, orgs, teams, pull requests)

Deployed via `docker compose up`.

---

## HARD CONSTRAINTS (Non-Negotiable)

These are absolute rules. Violating any one is a failure.

- ❌ NOT git-compatible — no .git/ directory, no git format
- ❌ NOT using git libraries — git2, gitoxide, gix, russh are FORBIDDEN
- ❌ NOT using SSH transport — all auth is JWT
- ❌ NOT spawning git binary anywhere (not in server, not in tests, nowhere)
- ❌ Users do NOT type "git" — they type "crust"
- ❌ Object hashing is SHA256 (not SHA1)
- ❌ Compression is zstd (not zlib)
- ❌ Wire protocol is CRUSTPACK (not git packfile)

If you find yourself writing code that would be "easier with git," stop and re-read the requirements. CRUST is intentionally incompatible.

---

## ARCHITECTURE

### Three Crates (Cargo workspace)

```
crust/
├── Cargo.workspace.toml
├── gitcore/              (library — pure Rust VCS model)
│   └── src/
├── crust-server/         (binary — Axum server + Tokio)
│   └── src/
├── crust-cli/            (binary — command-line client)
│   └── src/
└── contracts/            (handoff layer — schemas, specs, types)
    └── *.md, *.rs
```

**gitcore** (library):
- No async. No network. No database. No HTTP.
- Pure Rust implementation of object types, hashing, serialization, merge logic.
- Both server and CLI use this as a dependency.
- Tested with `cargo test -p gitcore` — no external services needed.

**crust-server** (binary):
- Axum web server (Tokio async runtime)
- PostgreSQL connection pool (sqlx, async)
- REST API endpoints from contracts/api-contracts.md
- Object storage: `/data/repos/{owner_id}/{repo_id}.crust/objects/`
- Disk + database (database for users/permissions, disk for objects)

**crust-cli** (binary):
- Full VCS client replacing git entirely
- Reads/writes `.crust/` in repos (NOT `.git/`)
- Reads/writes `~/.crust/` for config + credentials
- Calls gitcore for local VCS operations
- Calls crust-server via HTTPS for remote operations
- Distributed as standalone binary

### Contracts Directory (Handoff Layer)

**MOST IMPORTANT**: All boundaries (API, database, object format, CLI) are specified in `contracts/` before any code is written.

```
contracts/
├── README.md                  (ownership matrix)
├── data-types.rs             (shared types)
├── object-format.md          (SHA256, zstd, CRUST-OBJECT header)
├── crustpack-format.md       (wire protocol for push/fetch)
├── db-schema.md              (PostgreSQL tables + indexes)
├── error-codes.md            (all error codes UPPER_SNAKE_CASE)
├── api-contracts.md          (every endpoint: method, path, request, response, errors)
└── cli-commands.md           (every command, arguments, behavior, exit codes)
```

**Rule**: Before writing any code that crosses a boundary (API call, DB query, CLI command, object format), check the contract first. If it doesn't exist, write it before writing code.

---

## TECH STACK & CONSTRAINTS

| Component | Tech | Version | Notes |
|-----------|------|---------|-------|
| Language | Rust | 2021 edition | |
| Async Runtime | Tokio | latest | with feature: "full" |
| HTTP Framework | Axum | latest | |
| Database | PostgreSQL | 16+ | sqlx for compile-time checked queries |
| Auth | JWT | | jsonwebtoken crate, no SSH |
| Compression | zstd | | zstd crate for object storage |
| Hashing | SHA256 | | sha2 crate from RustCrypto |
| Package Manager | Cargo | | workspace with 3 crates |
| Deployment | Docker Compose | | app container + PostgreSQL container |
| CLI | Clap | | derive-based argument parsing |

### Forbidden Crates

- ❌ git2, gitoxide, gix (git libraries)
- ❌ russh (SSH)
- ❌ Any git-compatibility layer

### Required Crates

**gitcore**:
- sha2, zstd, hex, thiserror

**crust-server**:
- tokio (full), axum, sqlx (postgres, uuid, chrono), serde/serde_json
- argon2, jsonwebtoken, uuid, chrono
- tower, tower-http, tracing, tracing-subscriber
- anyhow

**crust-cli**:
- reqwest (json, blocking), clap (derive), rpassword, dirs
- anyhow, indicatif

---

## KEY FILES

### `.github/copilot-instructions.md` (this file)
Always-on context. Claude reads this on every interaction.

### `reasoning/task-breakdown.md`
**PRIMARY EXECUTION PLAN**. Every task:
- Has a unique number (TASK-001, TASK-002, ...)
- Lists exact dependencies (DEPENDS_ON)
- Lists what contracts it reads/produces
- Has a SPAWN_COMMAND for Copilot Chat

Reference this constantly. Update it as tasks complete.

### `contracts/` directory
Single source of truth for all system boundaries. Every agent reads this before writing code.

### `.github/agents/*.agent.md`
Personas for different phases of work. Each agent has:
- Role and expertise
- Pre-flight checks (contracts to read first)
- Implementation rules (CRUST-specific constraints)
- Spawn protocol (how to delegate to other agents)

---

## NAMING CONVENTIONS

| Type | Convention | Examples |
|------|-----------|---------|
| Rust Structs/Enums | PascalCase | `User`, `ApiResponse<T>`, `RepositoryPermission` |
| Rust Functions/Methods | snake_case | `get_user_data()`, `is_valid_email()` |
| Rust Constants | UPPER_SNAKE_CASE | `MAX_RETRIES`, `DEFAULT_PORT` |
| Error Codes | UPPER_SNAKE_CASE | `AUTH_INVALID_CREDENTIALS`, `REPO_NOT_FOUND` |
| File Names | kebab-case | `user-service.rs`, `auth-middleware.rs` |
| API Endpoints | kebab-case paths | `/api/v1/repos/:owner/:repo`, `/auth/login` |
| Database Tables | snake_case | `users`, `organizations`, `repo_permissions` |
| JSON Keys | snake_case | `user_id`, `created_at`, `is_public` |
| CLI Commands | lowercase | `crust init`, `crust commit`, `crust push` |
| Branches/Tags | kebab-case or UPPER | `feat/auth-system`, `v0.1.0` |

---

## ERROR HANDLING PATTERN

All responses follow this shape:

```rust
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

pub struct ApiError {
    pub code: String,           // from error-codes.md
    pub message: String,        // human-readable
    pub field: Option<String>,  // for validation errors
}

pub struct ResponseMetadata {
    pub timestamp: String,      // ISO8601 UTC
    pub duration: u64,          // milliseconds
    pub request_id: Option<String>,
}
```

Rules:
- **Every** API response, success or error, uses ApiResponse<T>
- **All** error codes come from contracts/error-codes.md (never invent new ones mid-code)
- **Never** expose stack traces in production responses
- **Always** use try-catch for async operations
- **Always** log errors with context: code, request, user, timestamp

Example:
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
    "request_id": "req-abc123def456"
  }
}
```

---

## CONTRACT-FIRST WORKFLOW

Before writing ANY code that crosses a boundary:

1. **Read** the relevant contract file from contracts/
2. **Check** that the contract exists and is complete
3. **Follow** the spec exactly (no interpretation, no shortcuts)
4. **If the contract doesn't exist**: STOP and create it first (spawn contracts-agent if needed)

Example: "I need to add a new API endpoint"
- Check contracts/api-contracts.md for the endpoint spec
- If it's not there, add it to contracts/api-contracts.md first
- Then write the code to match the spec exactly
- Update contracts/api-contracts.md to mark endpoint [x] IMPLEMENTED

Example: "I need a new database table"
- Check contracts/db-schema.md for the table spec
- If it's not there, define it in contracts/db-schema.md first
- Then create a migration file
- Then write the code to use it

---

## OBJECT FORMAT (CRUST-SPECIFIC)

Every object in CRUST has this exact format (see contracts/object-format.md):

```
CRUST-OBJECT\n
type: {blob|tree|commit|tag}\n
size: {uncompressed_byte_length}\n
\n
{raw content bytes}
```

All together: `SHA256(header + content)` = object ID (64 char hex)

On disk: `.crust/objects/{id[0..2]}/{id[2..64]}` contains `zstd(header + content)`

**Key points**:
- Object ID is always SHA256 (64 hex chars)
- Content is zstd-compressed before storage
- Header is deterministic (exact format)
- Tree entries are sorted by name
- Commits can have 0+ parents

---

## WIRE PROTOCOL (CRUSTPACK)

Objects are transmitted between client and server in CRUSTPACK format (see contracts/crustpack-format.md):

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

Used for:
- `POST /api/v1/repos/{owner}/{repo}/objects/upload` (client → server)
- `POST /api/v1/repos/{owner}/{repo}/objects/fetch` (server → client)

---

## CLI PARADIGM

Users interact via `crust` commands (NOT `git`).

Every command maps to a spec in contracts/cli-commands.md.

Examples:
```bash
crust init                                # init repo
crust status                              # show working tree state
crust add src/main.rs                     # stage file
crust commit -m "Add auth"                # create commit
crust push                                # push to remote
crust clone https://server.com/alice/repo # clone
crust log --oneline                       # show history
crust merge feat/auth                     # merge branch
```

Exit codes:
- 0: Success
- 1: User error (bad args, missing repo, etc.)
- 2: Runtime error (network, disk, server error)

---

## DATABASE (PostgreSQL 16)

Schema is in contracts/db-schema.md. Key tables:
- `users` (accounts)
- `repositories` (repos with ownership)
- `organizations` (org grouping)
- `teams` (team grouping within orgs)
- `repo_permissions` (explicit access grants)
- `pull_requests` (PRs with merge state)
- `pr_reviews` (code reviews)
- `pr_comments` (inline comments)

**No SSH keys table** (auth is JWT only).

Objects are stored on DISK, not in database:
- Database: users, permissions, PR metadata
- Disk: .crust object files organized by SHA256

---

## TESTING STRATEGY

| Level | What | Tool | Coverage |
|-------|------|------|----------|
| Unit | Pure functions, gitcore | cargo test -p gitcore | 90%+ |
| Integration | API endpoints | cargo test --test '*' | All endpoints in api-contracts.md |
| Contract | Code vs contracts | reasoning/learning.md audit | 100% (final PR gate) |

Run before commit:
```bash
cargo clippy --workspace -- -D warnings  # no warnings
cargo test --workspace                  # all tests pass
cargo fmt --check                        # formatting correct
```

---

## DEPLOYMENT

Single command to deploy:
```bash
docker compose up -d
```

Services:
- `db`: postgres:16-alpine
- `app`: crust-server binary

Environment variables:
```
DATABASE_URL=postgres://user:pass@db:5432/crust
JWT_SECRET=... (min 64 random chars)
REPO_BASE_PATH=/data/repos
PORT=8080
JWT_EXPIRY_SECONDS=86400
LOG_LEVEL=info
ALLOW_REGISTRATION=true
```

---

## REASONING DIRECTORY (Progress Tracking)

### reasoning/task-breakdown.md
**PRIMARY FILE FOR TRACKING EXECUTION**

Every task has:
- Unique number (TASK-001, ...)
- STATUS: [ ] PENDING | [>] IN PROGRESS | [x] COMPLETE | [!] BLOCKED
- AGENT: which agent runs it
- DEPENDS_ON: dependencies (task must wait for these)
- READS: contracts this task reads
- PRODUCES: files/contracts this task creates
- DESCRIPTION: what to build
- ACCEPTANCE_CRITERIA: testable outcomes
- SPAWN_COMMAND: exact text to paste in Copilot Chat

Example:
```markdown
## TASK-001 — Generate All Contracts
STATUS: [x] COMPLETE
AGENT: contracts-agent
DEPENDS_ON: (none)
PRODUCES: contracts/data-types.rs, contracts/api-contracts.md, ...
DESCRIPTION: Generate all shared contracts before any feature code...
ACCEPTANCE_CRITERIA:
  - [x] All entities have types
  - [x] No {{PLACEHOLDER}}
```

### reasoning/learning.md
Log architectural decisions and blockers encountered.

---

## AGENT WORKFLOW

### Main Agent Orchestrator
Reads requirements, checks task-breakdown.md, spawns next agent.

### Contracts Agent
Generates all contracts/ files before any code.

### Backend Agent
Implements APIs, database layer, object storage.

### Gitcore Agent
Implements pure VCS library (no network, no DB).

### CLI Agent
Implements command-line client.

### Each Agent Has Pre-Flight Check
Before writing code:
1. Read required contracts from contracts/
2. Verify all dependencies are complete
3. If any contract missing: STOP and request contracts-agent

---

## HOW TO USE THIS IN COPILOT

### Start a New Task

When ready to work on a task:

```
@main-agent

I'm ready to start building CRUST. What's the next task?
```

Main agent will read task-breakdown.md and tell you which agent to spawn.

### Spawn a Specific Agent

```
@contracts-agent
TASK: TASK-001 — Generate All Contracts
CONTEXT_FILES: requirements-v2.md
PRODUCES: contracts/data-types.rs, contracts/api-contracts.md, ...
```

The agent will work autonomously on that task, reading contracts before implementing.

### Check Progress

```
@main-agent

What's our progress? How many tasks are complete?
```

Main agent reads task-breakdown.md and reports.

### Audit Contracts vs. Implementation

```
@contracts-agent
TASK: Final Contract Audit

Check that every contract in contracts/ is correctly implemented in src/.
Report any mismatches.
```

---

## KEY PRINCIPLES

1. **Contract-First**: Contracts before code. Every boundary defined before implementation.
2. **No Git Compatibility**: CRUST is intentionally incompatible with git.
3. **Pure Gitcore Library**: Server and CLI both use gitcore as a library, not reimplementing.
4. **Single Source of Truth**: contracts/ is the truth; all other code reads from it.
5. **Deterministic Serialization**: Objects serialize to the same bytes every time.
6. **No Panics**: Public API returns Result<T>, never panics.
7. **Typed All The Way**: Strong typing (Rust + sqlx compile-time checks).
8. **Error Codes Over Messages**: Client code looks up error codes to handle, not string matching.
9. **Async Where It Matters**: gitcore is pure sync; server is fully async.
10. **Test Before Handoff**: Every task has passing tests before marking [x] COMPLETE.

---

## RED FLAGS (Stop and re-read requirements if you see these)

- ❌ Any git library import
- ❌ Any git binary invocation
- ❌ SHA1 instead of SHA256
- ❌ zlib instead of zstd
- ❌ SSH key handling
- ❌ git wire protocol (pkt-line)
- ❌ Users typing "git" instead of "crust"
- ❌ Code that looks easier "if we just used git"

**Remember**: CRUST is intentionally NOT git. That's the whole point.

---

## QUICK REFERENCE

- **Requirements**: [requirements-v2.md](requirements-v2.md)
- **Contracts**: [contracts/](contracts/)
- **Tasks**: [reasoning/task-breakdown.md](reasoning/task-breakdown.md)
- **Agents**: [.github/agents/](. github/agents/)
- **Prompts**: [.github/prompts/](.github/prompts/)
- **Errors**: [contracts/error-codes.md](contracts/error-codes.md)
- **API**: [contracts/api-contracts.md](contracts/api-contracts.md)
- **Objects**: [contracts/object-format.md](contracts/object-format.md)
- **Database**: [contracts/db-schema.md](contracts/db-schema.md)

---

## This is Your North Star

When in doubt, read this file again. It contains the full model of how CRUST works.

Then read the relevant contract file.

Then read the agent file for the task at hand.

Then code.

Never the other way around.
