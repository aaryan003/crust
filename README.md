<div align="center">

<img src="docs/logo.png" alt="CRUST Logo" width="180" />

**A fully original, self-hosted version control system — built from scratch in Rust.**

[![Build](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square&logo=github-actions)](.)
[![Version](https://img.shields.io/badge/version-0.1.0-orange?style=flat-square)](.)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-original-blue?style=flat-square)](LICENSE)
[![Status](https://img.shields.io/badge/status-production--ready-brightgreen?style=flat-square)](.)

> CRUST is intentionally **NOT** git. SHA256, zstd, JWT, CRUSTPACK — built different, by design.

[Quick Start](#-quick-start) · [Architecture](#-architecture) · [CLI Reference](docs/CRUST-CLI-GUIDE.md) · [API Reference](docs/CRUST-API-REFERENCE.md) · [Contributing](CONTRIBUTING.md)

</div>

---

## Why CRUST?

| | Git | **CRUST** |
|---|---|---|
| Hashing | SHA1 | **SHA256** |
| Compression | zlib | **zstd** |
| Auth | SSH keys | **HTTPS + JWT** |
| Wire protocol | packfile | **CRUSTPACK** |
| Command | `git` | **`crust`** |
| Self-hosted | complex | **`docker compose up`** |

CRUST is a ground-up reimplementation — no `git2`, no `gitoxide`, no SSH, no `.git/` directory, no compatibility shims. It is its own thing.

---

## Feature Status

| Feature | Status |
|---|---|
| User registration & JWT authentication | ✅ Complete |
| Repository CRUD | ✅ Complete |
| Object storage (Blob / Tree / Commit / Tag) | ✅ Complete |
| CRUSTPACK wire protocol (push / fetch) | ✅ Complete |
| Branch management | ✅ Complete |
| 3-way merge with conflict detection | ✅ Complete |
| Pull requests (create / review / merge) | ✅ Complete |
| Organizations & teams (RBAC) | ✅ Complete |
| Full CLI — 24 commands | ✅ Complete |
| Docker deployment | ✅ Complete |
| Repository content browser (tree / blob) | ⏳ v0.2 |
| Web UI | ⏳ v0.2 |
| Webhooks / CI integration | ⏳ v0.2 |

---

## 🚀 Quick Start

### Server (Docker)

```bash
# 1. Clone
git clone <this-repo> crust && cd crust

# 2. Configure — set a strong JWT_SECRET (min 64 chars)
cp .env.example .env

# 3. Launch
docker compose up -d

# 4. Health check
curl http://localhost:8080/health
# → {"status":"ok","database":"ok","disk":"ok"}
```

Server is live at **`http://localhost:8080`**.

---

### CLI

**Install**

```bash
# Build from source
cargo build --release -p crust-cli
cp target/release/crust /usr/local/bin/crust
```

**Typical workflow**

```bash
# Auth
crust login http://localhost:8080

# Init & commit
mkdir my-project && cd my-project
crust init
echo "fn main() {}" > src/main.rs
crust add src/main.rs
crust commit -m "Initial commit"

# Create remote repo (API)
curl -X POST http://localhost:8080/api/v1/repos \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"my-project","display_name":"My Project","is_public":true}'

# Push
crust remote add origin http://localhost:8080/alice/my-project
crust push

# Branch & collaborate
crust branch feat/login
crust checkout feat/login
crust add .
crust commit -m "Add login feature"
crust push

# Merge
crust checkout main
crust merge feat/login
crust push
```

Full reference: [docs/CRUST-CLI-GUIDE.md](docs/CRUST-CLI-GUIDE.md)

---

## 🏗 Architecture

CRUST is a Cargo workspace — three focused crates with a clean dependency graph.

```
crust/
├── gitcore/          ← Pure Rust VCS library (no async, no I/O, no DB)
├── crust-server/     ← Axum HTTP server + PostgreSQL
└── crust-cli/        ← Standalone CLI binary
```

### `gitcore` — VCS Core

Zero external I/O. Fully deterministic. Used as a library by both server and CLI.

```
Objects:     Blob · Tree · Commit · Tag
Format:      CRUST-OBJECT\ntype: ...\nsize: ...\n\n{bytes}
Object ID:   SHA256(header + content)  →  64-char lowercase hex
Compression: zstd
Merge:       3-way with conflict detection
```

### `crust-server` — HTTP API

```
Framework:   Axum + Tokio
Database:    PostgreSQL 16 via sqlx (compile-time checked queries)
Auth:        JWT Bearer tokens · argon2 password hashing
Storage:     /data/repos/{owner_id}/{repo_id}.crust/objects/{2char}/{62char}
Protocol:    CRUSTPACK binary (see contracts/crustpack-format.md)
Endpoints:   29 REST endpoints — auth · repos · objects · PRs · orgs · teams
```

### `crust-cli` — Command-Line Client

```
Args:        Clap (derive-based)
HTTP:        reqwest (blocking)
Local state: .crust/ (repo)  ·  ~/.crust/ (user config)
Commands:    24 — init · login · commit · push · pull · clone · branch · merge ...
```

Detailed design: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

---

## 🔒 Hard Constraints

These are absolute. Any violation is a build failure.

```
❌ NOT git-compatible        — no .git/, no git object format
❌ NOT using git libraries   — git2 / gitoxide / gix / russh are FORBIDDEN
❌ NOT using SSH transport   — all auth is JWT over HTTPS
❌ NOT spawning git binary   — anywhere, ever

✅ SHA256    for all object hashing
✅ zstd      for compression
✅ CRUSTPACK for wire protocol
✅ JWT       for all authentication
✅ crust     — users type `crust`, not `git`
```

---

## 🛠 Technology Stack

| Layer | Technology |
|---|---|
| Language | Rust 2021 edition |
| Async runtime | Tokio |
| HTTP framework | Axum |
| Database | PostgreSQL 16 |
| SQL | sqlx (compile-time checked) |
| Auth | JWT + argon2 |
| Compression | zstd |
| Hashing | SHA256 (`sha2` crate) |
| CLI | Clap (derive) |
| HTTP client | reqwest (blocking) |
| Deployment | Docker Compose |

---

## 🧪 Development

```bash
# Requirements: Rust 1.75+, Docker, PostgreSQL 16

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Lint — zero warnings required
cargo clippy --workspace -- -D warnings
```

Full setup guide: [docs/SETUP.md](docs/SETUP.md)

---

## 📁 Project Layout

```
crust/
├── gitcore/
│   └── src/                  blob · tree · commit · tag · merge · object
├── crust-server/
│   ├── src/                  auth · routes · storage · permissions
│   └── migrations/           PostgreSQL schema
├── crust-cli/
│   └── src/
│       ├── commands/         24 command implementations
│       ├── client.rs         HTTP client
│       ├── config.rs         credentials & config
│       ├── index.rs          staging area
│       ├── pack.rs           CRUSTPACK encode/decode
│       └── working_tree.rs   filesystem operations
├── contracts/                API · DB · CLI · object format specs
├── docs/                     user documentation
├── reasoning/                architecture decisions & task tracking
├── Dockerfile
├── docker-compose.yml
└── Cargo.toml
```

---

## 📖 Documentation

| Document | Description |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Three-crate design, object format, wire protocol, DB schema |
| [docs/SETUP.md](docs/SETUP.md) | Dev setup, environment variables, running locally |
| [docs/CRUST-CLI-GUIDE.md](docs/CRUST-CLI-GUIDE.md) | All 24 CLI commands with examples |
| [docs/CRUST-API-REFERENCE.md](docs/CRUST-API-REFERENCE.md) | All 37 REST API endpoints |
| [docs/CRUST-API.postman_collection.json](docs/CRUST-API.postman_collection.json) | Postman collection — import and test immediately |
| [contracts/](contracts/) | Single source of truth for all system boundaries |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Code standards, contribution guide |

---

## Version History

| Version | Date | Notes |
|---|---|---|
| 0.1.0 | 2026-03-05 | Initial release — 24 CLI commands, 29 API endpoints, Docker |

---

<div align="center">

CRUST is an original system designed from first principles.  
Not affiliated with or derived from Git, GitHub, or any other VCS.

**CRUST is intentionally NOT git. That's the whole point.**

</div>