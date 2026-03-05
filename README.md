# CRUST

> A fully original, self-hosted version control system built from scratch in Rust.

**Version**: 0.1.0 | **Status**: ✅ Production-Ready | **Last Updated**: 2026-03-05

CRUST is intentionally **NOT** git. It uses SHA256 (not SHA1), zstd compression (not zlib), HTTPS + JWT authentication (not SSH keys), and its own CRUSTPACK wire protocol. Users type `crust`, not `git`.

---

## Table of Contents

- [Quick Start (Docker)](#quick-start-docker)
- [Quick Start (CLI)](#quick-start-cli)
- [Architecture](#architecture)
- [Feature Status](#feature-status)
- [Documentation](#documentation)
- [Hard Constraints](#hard-constraints)
- [Technology Stack](#technology-stack)
- [Development Setup](#development-setup)
- [Running Tests](#running-tests)

---

## Quick Start (Docker)

The fastest way to run CRUST — one command:

```bash
# 1. Clone this repo
git clone <this-repo> crust && cd crust

# 2. Copy and edit environment config
cp .env.example .env
# ⚠️  IMPORTANT: Set a strong JWT_SECRET (min 64 chars)

# 3. Start server + database
docker compose up -d

# 4. Verify health
curl http://localhost:8080/health
# → {"status":"ok","database":"ok","disk":"ok",...}
```

The server is now running at **http://localhost:8080**.

---

## Quick Start (CLI)

### Install

```bash
# Build from source
cargo build --release -p crust-cli
cp target/release/crust /usr/local/bin/crust

# Or run directly
./target/release/crust --help
```

### Typical Workflow

```bash
# Authenticate with your CRUST server
crust login http://localhost:8080

# Create a new local repo
mkdir my-project && cd my-project
crust init

# Add files and commit
echo "fn main() {}" > src/main.rs
crust add src/main.rs
crust commit -m "Initial commit"

# Create the repo on the server first (API-only for now)
curl -X POST http://localhost:8080/api/v1/repos \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-project", "display_name": "My Project", "is_public": true}'

# Connect to remote and push
crust remote add origin http://localhost:8080/alice/my-project
crust push

# Collaborate
crust clone http://localhost:8080/alice/my-project
crust branch feat/login
crust checkout feat/login
# ... make changes ...
crust add .
crust commit -m "Add login feature"
crust push

# Merge via pull request (server-side)
# Or merge locally:
crust checkout main
crust merge feat/login
crust push
```

Full CLI reference: [docs/CRUST-CLI-GUIDE.md](docs/CRUST-CLI-GUIDE.md)

---

## Architecture

CRUST is a Cargo workspace with three crates:

```
crust/
├── gitcore/        ← Pure Rust VCS library (no async, no network, no DB)
├── crust-server/   ← Axum HTTP server + PostgreSQL
└── crust-cli/      ← Standalone CLI binary
```

### `gitcore` — VCS Core

Pure library. Zero external I/O. Fully deterministic.

- **Objects**: `Blob`, `Tree`, `Commit`, `Tag` — SHA256 hashed, zstd compressed
- **Object format**: `CRUST-OBJECT\ntype: ...\nsize: ...\n\n{bytes}`
- **Object ID**: `SHA256(header + content)` — 64 lowercase hex chars
- **Merge**: 3-way merge with conflict detection
- **Used by**: Both `crust-server` and `crust-cli` as a library dependency

### `crust-server` — HTTP API

- **Framework**: Axum + Tokio
- **Database**: PostgreSQL 16 via sqlx (compile-time checked queries)
- **Auth**: JWT Bearer tokens, argon2 password hashing
- **Object storage**: `/data/repos/{owner_id}/{repo_id}.crust/objects/{2char}/{62char}`
- **Wire protocol**: CRUSTPACK (binary, see `contracts/crustpack-format.md`)
- **29 REST endpoints** across auth, repos, objects, PRs, orgs, teams

### `crust-cli` — Command-Line Client

- **Argument parsing**: Clap (derive-based)
- **HTTP client**: reqwest (blocking)
- **Local state**: `.crust/` directory in repo, `~/.crust/` for user config
- **24 commands**: init, login, commit, push, pull, clone, branch, merge, and more

Detailed architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

### ✅ Copilot Instructions
- `.github/copilot-instructions.md` — 2,200+ lines of always-on context

This file is read by Claude on every interaction. It contains:
- CRUST overview, hard constraints, architecture
- Tech stack, naming conventions
- Error handling pattern (ApiResponse<T> wrapper)
- Object format, wire protocol, database model
- Testing strategy, deployment model
- Agent workflow, red flags, quick reference

---

## Feature Status

| Feature | Status |
|---------|--------|
| User registration & authentication (JWT) | ✅ Complete |
| Repository CRUD (create, read, update, delete) | ✅ Complete |
| Object storage (Blob, Tree, Commit, Tag) | ✅ Complete |
| CRUSTPACK wire protocol (push/fetch) | ✅ Complete |
| Branch management (create, delete, checkout) | ✅ Complete |
| 3-way merge with conflict detection | ✅ Complete |
| Pull requests (create, review, merge) | ✅ Complete |
| Organizations & teams with role-based access | ✅ Complete |
| Full CLI (24 commands) | ✅ Complete |
| Docker deployment | ✅ Complete |
| Repository content browsing (tree/blob/commits) | ⏳ v0.2 |
| Web UI | ⏳ v0.2 |
| Webhooks / CI integration | ⏳ v0.2 |

---

## Hard Constraints

These are absolute. Violating any one is a build failure.

- ❌ **NOT git-compatible** — no `.git/` directory, no git format
- ❌ **NOT using git libraries** — git2, gitoxide, gix, russh are FORBIDDEN
- ❌ **NOT using SSH transport** — all auth is JWT
- ❌ **NOT spawning git binary** — anywhere, anytime
- ✅ **SHA256** for hashing (not SHA1)
- ✅ **zstd** for compression (not zlib)
- ✅ **CRUSTPACK** for wire protocol (not git packfile)
- ✅ **HTTPS + JWT** for all authentication
- ✅ Users type **`crust`** (not `git`)

---

## Technology Stack

| Component | Tech |
|-----------|------|
| Language | Rust 2021 edition |
| Async runtime | Tokio |
| HTTP framework | Axum |
| Database | PostgreSQL 16 |
| SQL | sqlx (compile-time checked) |
| Auth | JWT + argon2 |
| Compression | zstd |
| Hashing | SHA256 (sha2 crate) |
| CLI | Clap (derive) |
| HTTP client | reqwest (blocking) |
| Deployment | Docker Compose |

---

## Development Setup

See [docs/SETUP.md](docs/SETUP.md) for the full guide. TL;DR:

```bash
# Requirements: Rust 1.75+, Docker, PostgreSQL 16

# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Zero warnings (required)
cargo clippy --workspace -- -D warnings
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Three-crate design, object format, wire protocol, DB schema |
| [docs/SETUP.md](docs/SETUP.md) | Development setup, environment variables, running locally |
| [docs/CRUST-CLI-GUIDE.md](docs/CRUST-CLI-GUIDE.md) | All 24 CLI commands with examples |
| [docs/CRUST-API-REFERENCE.md](docs/CRUST-API-REFERENCE.md) | All 37 REST API endpoints |
| [docs/CRUST-API.postman_collection.json](docs/CRUST-API.postman_collection.json) | Postman collection (import and test immediately) |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute, code standards |
| [contracts/](contracts/) | Single source of truth for all system boundaries |

---

## Project Layout

```
crust/
├── gitcore/               ← Pure Rust VCS library
│   └── src/               (blob, tree, commit, tag, merge, object)
├── crust-server/          ← Axum HTTP server
│   ├── src/               (auth, routes, storage, permissions)
│   └── migrations/        (PostgreSQL schema migrations)
├── crust-cli/             ← CLI client binary
│   └── src/
│       ├── commands/      (24 command implementations)
│       ├── client.rs      (HTTP client)
│       ├── config.rs      (credentials & config management)
│       ├── index.rs       (staging area)
│       ├── pack.rs        (CRUSTPACK encode/decode)
│       └── working_tree.rs (file system ops)
├── contracts/             ← API, DB, CLI, object format specs
├── docs/                  ← User documentation
├── reasoning/             ← Architecture decisions & task tracking
├── Dockerfile
├── docker-compose.yml
└── Cargo.toml
```

---

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 0.1.0 | 2026-03-05 | Full implementation: 24 CLI commands, 29 API endpoints, Docker |

---

## License

CRUST is an original system designed from first principles.  
It is not affiliated with or derived from Git, GitHub, or any other VCS.

---

> CRUST is intentionally NOT git. That's the whole point.
