# CRUST — Development Setup Guide

**Last Updated**: 2026-03-05

This guide covers everything needed to build, run, and test CRUST locally.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start (Docker)](#quick-start-docker)
- [Local Development (No Docker)](#local-development-no-docker)
- [Building the CLI](#building-the-cli)
- [Environment Variables](#environment-variables)
- [Running Tests](#running-tests)
- [Project Structure](#project-structure)
- [Common Tasks](#common-tasks)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

| Tool | Minimum Version | Install |
|------|----------------|---------|
| Rust | 1.75 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Docker | 24+ | https://docs.docker.com/get-docker/ |
| Docker Compose | v2 | Bundled with Docker Desktop |
| PostgreSQL | 16 (optional, for local dev without Docker) | https://www.postgresql.org/download/ |

> The `rust-toolchain.toml` file at the repo root pins the exact Rust version. `rustup` will install it automatically on first `cargo build`.

---

## Quick Start (Docker)

The fastest path — no local PostgreSQL needed.

```bash
# 1. Clone the repository
git clone <repo-url> crust
cd crust

# 2. Configure environment
cp .env.example .env

# Edit .env and set a real JWT_SECRET (minimum 64 random characters)
# Generate one with:
openssl rand -base64 64

# 3. Start containers
docker compose up -d

# 4. Verify the server is healthy
curl http://localhost:8080/health
```

Expected response:
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "database": "ok",
    "disk": "ok",
    "uptime_seconds": 5
  }
}
```

### Viewing Logs

```bash
# Server logs
docker compose logs -f app

# Database logs
docker compose logs -f db

# All logs
docker compose logs -f
```

### Stopping

```bash
docker compose down          # Stop containers, keep volumes
docker compose down -v       # Stop containers AND delete database + object data
```

---

## Local Development (No Docker)

### 1. Start PostgreSQL

If you have PostgreSQL 16 running locally:

```bash
# Create database and user
psql postgres -c "CREATE USER crust_user WITH PASSWORD 'crust_password';"
psql postgres -c "CREATE DATABASE crust OWNER crust_user;"
```

### 2. Set Environment Variables

```bash
export DATABASE_URL="postgres://crust_user:crust_password@localhost:5432/crust"
export JWT_SECRET="your-super-secret-jwt-key-here-minimum-64-characters-long-!!!!"
export PORT=8080
export LOG_LEVEL=info
export REPO_BASE_PATH=/tmp/crust-repos
export JWT_EXPIRY_SECONDS=86400
export ALLOW_REGISTRATION=true

mkdir -p /tmp/crust-repos
```

Or use a `.env` file and load it:
```bash
cp .env.example .env
# Edit .env with your values
source .env   # or use `dotenv` / `direnv`
```

### 3. Run Migrations

Migrations run automatically on server startup. To run them manually:

```bash
# Install sqlx-cli if needed
cargo install sqlx-cli --features postgres

# Run migrations
sqlx migrate run --database-url "$DATABASE_URL" --source crust-server/migrations
```

### 4. Start the Server

```bash
# Development build (faster compilation, slower runtime)
cargo run -p crust-server

# Release build (slower compilation, fast runtime)
cargo build --release -p crust-server
./target/release/crust-server
```

The server will start and print:
```
INFO crust_server: Starting CRUST server on 0.0.0.0:8080
INFO crust_server: Database connected
INFO crust_server: Running migrations...
INFO crust_server: Migrations complete (2 applied)
INFO crust_server: Server listening on 0.0.0.0:8080
```

---

## Building the CLI

```bash
# Development build
cargo build -p crust-cli
./target/debug/crust --help

# Release build (recommended for actual use)
cargo build --release -p crust-cli
./target/release/crust --help

# Install globally
cargo install --path crust-cli
crust --help
```

### First Use

```bash
# Login to your running server
crust login http://localhost:8080

# Check who's logged in
crust whoami
```

Credentials are stored in `~/.crust/credentials` (JSON format):
```json
{
  "token": "eyJ...",
  "username": "alice",
  "server_url": "http://localhost:8080"
}
```

---

## Environment Variables

All server configuration is via environment variables (or `.env` file):

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `DATABASE_URL` | — | ✅ Yes | PostgreSQL connection string |
| `JWT_SECRET` | — | ✅ Yes | Secret for signing JWTs (min 64 chars) |
| `PORT` | `8080` | No | Server listen port |
| `LOG_LEVEL` | `info` | No | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `REPO_BASE_PATH` | `/data/repos` | No | Base directory for object storage |
| `JWT_EXPIRY_SECONDS` | `86400` | No | Token TTL in seconds (default: 24h) |
| `ALLOW_REGISTRATION` | `true` | No | Set `false` to disable new account creation |
| `RUST_LOG` | `crust_server=info` | No | Fine-grained log control (tracing format) |

### Object Storage Layout

Objects are stored at:
```
{REPO_BASE_PATH}/
└── {owner_uuid}/
    └── {repo_uuid}.crust/
        ├── objects/
        │   └── {2char}/
        │       └── {62char}      ← zstd-compressed CRUST object
        └── refs/
            ├── heads/
            │   └── main          ← SHA256 hex (64 chars)
            └── tags/
```

---

## Running Tests

### All Tests

```bash
cargo test --workspace
```

### Per-Crate Tests

```bash
# VCS library (pure unit tests — no DB, no network)
cargo test -p gitcore

# Server tests (requires DATABASE_URL set)
cargo test -p crust-server

# CLI tests
cargo test -p crust-cli
```

### Linting & Formatting

```bash
# Check for warnings (CI-blocking — must be zero)
cargo clippy --workspace -- -D warnings

# Format check (CI-blocking)
cargo fmt --check

# Auto-format
cargo fmt
```

### Expected Results (v0.1.0)

```
running 31 tests
...
test result: ok. 31 passed; 0 failed; 0 ignored
```

---

## Project Structure

```
crust/
├── Cargo.toml                  ← Workspace definition
├── rust-toolchain.toml         ← Pinned Rust version
├── Dockerfile                  ← Multi-stage build for crust-server
├── docker-compose.yml          ← App + PostgreSQL
├── .env.example                ← Environment variable template
│
├── gitcore/                    ← Pure VCS library (no async, no network)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              (public API)
│       ├── blob.rs             (file content objects)
│       ├── tree.rs             (directory objects)
│       ├── commit.rs           (commit objects)
│       ├── tag.rs              (annotated tag objects)
│       ├── object.rs           (generic object enum + storage)
│       ├── merge.rs            (3-way merge algorithm)
│       └── error.rs            (error types)
│
├── crust-server/               ← HTTP API server
│   ├── Cargo.toml
│   ├── migrations/
│   │   ├── 001_initial_schema.sql
│   │   └── 002_updated_at_triggers.sql
│   └── src/
│       ├── main.rs             (entry point, startup)
│       ├── lib.rs              (shared server types)
│       ├── database.rs         (pool setup, migration runner)
│       ├── routes.rs           (route registration)
│       ├── permissions.rs      (permission checks)
│       ├── auth/
│       │   ├── handlers.rs     (register, login, logout, me)
│       │   ├── middleware.rs   (JWT extraction + validation)
│       │   └── token.rs        (JWT encode/decode)
│       ├── routes/
│       │   ├── objects.rs      (preflight, upload, fetch, refs/update)
│       │   ├── prs.rs          (pull request endpoints)
│       │   ├── orgs.rs         (organization endpoints)
│       │   └── teams.rs        (team endpoints)
│       └── storage/
│           └── mod.rs          (ObjectStore: disk read/write)
│
├── crust-cli/                  ← CLI client binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             (Clap entry point, command dispatch)
│       ├── client.rs           (HTTP client wrapper)
│       ├── config.rs           (credentials + repo config)
│       ├── index.rs            (staging area management)
│       ├── pack.rs             (CRUSTPACK encode/decode)
│       ├── refs.rs             (local ref management)
│       ├── remote.rs           (push/fetch logic)
│       ├── working_tree.rs     (file system operations)
│       └── commands/
│           ├── add.rs, branch.rs, cat_object.rs, checkout.rs
│           ├── clone.rs, commit.rs, diff.rs, fetch.rs
│           ├── hash_object.rs, init.rs, log.rs, login.rs
│           ├── logout.rs, ls_tree.rs, merge.rs, pull.rs
│           ├── push.rs, remote.rs, restore.rs, show.rs
│           ├── status.rs, verify_pack.rs, whoami.rs
│           └── mod.rs
│
├── contracts/                  ← Single source of truth
│   ├── README.md               (ownership matrix)
│   ├── data-types.rs           (shared Rust types)
│   ├── object-format.md        (CRUST object spec)
│   ├── crustpack-format.md     (wire protocol)
│   ├── db-schema.md            (PostgreSQL tables)
│   ├── error-codes.md          (45+ error codes)
│   ├── api-contracts.md        (37 REST endpoints)
│   └── cli-commands.md         (24 CLI commands)
│
├── docs/                       ← User documentation
│   ├── ARCHITECTURE.md
│   ├── SETUP.md                ← This file
│   ├── CRUST-CLI-GUIDE.md
│   ├── CRUST-API-REFERENCE.md
│   └── CRUST-API.postman_collection.json
│
├── reasoning/                  ← Development tracking
│   ├── task-breakdown.md       (17 tasks, status + acceptance criteria)
│   └── learning.md             (architectural decisions + lessons)
│
└── tests/
    └── integration_tests.rs
```

---

## Common Tasks

### Add a New API Endpoint

1. **Check** `contracts/api-contracts.md` — does the endpoint spec exist?
2. **If not**: add the spec to `contracts/api-contracts.md` first
3. **Add route** in `crust-server/src/routes/` (or `routes.rs`)
4. **Register** the route in `crust-server/src/routes.rs`
5. **Add tests** verifying the response shape matches the contract
6. **Mark** the endpoint `[x] IMPLEMENTED` in `contracts/api-contracts.md`

### Add a New CLI Command

1. **Check** `contracts/cli-commands.md` — does the command spec exist?
2. **If not**: add the spec first
3. **Create** `crust-cli/src/commands/<command_name>.rs`
4. **Register** in `crust-cli/src/commands/mod.rs`
5. **Add subcommand** to Clap in `crust-cli/src/main.rs`
6. **Test** with a running server

### Run the SQLX Offline Query Cache

When you add new `sqlx::query!` macros, regenerate the offline cache:

```bash
# Server must be running with DATABASE_URL set
cargo sqlx prepare --workspace
```

This updates `.sqlx/` which is committed so Docker builds work without a live DB.

### Generate a Strong JWT Secret

```bash
openssl rand -base64 64
# or
head -c 64 /dev/urandom | base64
```

---

## Troubleshooting

### `error: failed to connect to database`

Ensure `DATABASE_URL` is set and PostgreSQL is running:
```bash
psql "$DATABASE_URL" -c "SELECT 1;"
```

### `SQLX_OFFLINE=true` build errors

The `.sqlx/` offline query cache may be out of date. Rebuild with a live DB:
```bash
cargo sqlx prepare --workspace
```

### `JWT_SECRET` too short

The server enforces a minimum 32-byte secret. Use at least 64 characters in production.

### Docker build fails on Apple Silicon (arm64)

The Dockerfile uses `rust:1.75-slim` (Debian) which builds correctly on both amd64 and arm64. If you see OpenSSL errors:
```bash
docker compose build --no-cache app
```

### Port 8080 already in use

Change the port in `.env`:
```
PORT=9090
```
Then `docker compose up -d` again.

### Objects not persisting after restart

The `docker-compose.yml` uses a named volume `crust_repos` for object storage. If you ran `docker compose down -v`, this volume was deleted. The volume persists across normal `docker compose down` restarts.

### `crust` CLI: "No credentials found"

Run `crust login http://localhost:8080` first. Credentials are stored at `~/.crust/credentials`.

### `crust` CLI: "Not a crust repository"

You must be inside a directory initialized with `crust init` (or a subdirectory of one).

---

## VS Code Setup

Recommended extensions (see `.vscode/extensions.json`):

- **rust-analyzer** — Rust language server
- **Even Better TOML** — Cargo.toml syntax
- **crates** — Dependency version hints
- **CodeLLDB** — Debugging Rust programs

Settings are pre-configured in `.vscode/settings.json`:
- Format on save with `rustfmt`
- Clippy as the check command
- rust-analyzer with workspace features

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Deep dive into the three-crate design |
| [docs/CRUST-CLI-GUIDE.md](CRUST-CLI-GUIDE.md) | All 24 CLI commands with examples |
| [docs/CRUST-API-REFERENCE.md](CRUST-API-REFERENCE.md) | All 37 REST API endpoints |
| [contracts/api-contracts.md](../contracts/api-contracts.md) | Authoritative API spec |
| [contracts/error-codes.md](../contracts/error-codes.md) | All error codes |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | How to contribute |
