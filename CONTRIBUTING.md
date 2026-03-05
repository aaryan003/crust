# Contributing to CRUST

Thank you for your interest in contributing to CRUST!

CRUST is a fully original version control system built in Rust. It is intentionally **NOT git-compatible** — please read the hard constraints below before starting.

---

## Table of Contents

- [Hard Constraints (Read First)](#hard-constraints-read-first)
- [Development Setup](#development-setup)
- [Contract-First Workflow](#contract-first-workflow)
- [Code Standards](#code-standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Architecture Overview](#architecture-overview)
- [Where to Start](#where-to-start)

---

## Hard Constraints (Read First)

These are non-negotiable. Any contribution that violates these will be rejected.

| Constraint | Rule |
|-----------|------|
| ❌ No git libraries | `git2`, `gitoxide`, `gix`, `russh` are FORBIDDEN |
| ❌ No git binary | Never spawn `git` anywhere |
| ❌ No SSH auth | All authentication is JWT only |
| ❌ No `.git/` directories | Local repos use `.crust/` |
| ❌ No SHA1 | Object IDs are always SHA256 (64 hex chars) |
| ❌ No zlib | Compression is always zstd |
| ❌ No pkt-line | Wire protocol is CRUSTPACK |
| ✅ Users type `crust` | Never `git` |

If you're thinking "this would be easier with git" — stop. That's intentional. CRUST is designed to be incompatible.

---

## Development Setup

See [docs/SETUP.md](docs/SETUP.md) for the full guide. Quick version:

```bash
# Prerequisites: Rust 1.75+, Docker, PostgreSQL 16

# Clone and build
git clone <repo> crust && cd crust
cargo build --workspace

# Start the server (Docker)
cp .env.example .env  # set JWT_SECRET
docker compose up -d

# Verify
curl http://localhost:8080/health
```

---

## Contract-First Workflow

**Every change that crosses a system boundary must start with a contract update.**

System boundaries are defined in `contracts/`:

| Contract | What it defines |
|----------|----------------|
| `contracts/api-contracts.md` | Every HTTP endpoint |
| `contracts/cli-commands.md` | Every CLI command |
| `contracts/db-schema.md` | Every database table |
| `contracts/object-format.md` | On-disk object format |
| `contracts/crustpack-format.md` | Wire protocol |
| `contracts/error-codes.md` | All error codes |
| `contracts/data-types.rs` | Shared Rust types |

### Adding a New API Endpoint

1. Add the endpoint spec to `contracts/api-contracts.md` (method, path, request shape, response shape, error codes)
2. If new error codes are needed, add them to `contracts/error-codes.md` first
3. Implement the endpoint in `crust-server/src/routes/`
4. Add a test verifying the response shape
5. Mark the endpoint `[x] IMPLEMENTED` in `contracts/api-contracts.md`

### Adding a New CLI Command

1. Add the command spec to `contracts/cli-commands.md` (arguments, behavior, output, exit codes)
2. Create `crust-cli/src/commands/<command>.rs`
3. Register in `crust-cli/src/commands/mod.rs` and `crust-cli/src/main.rs`
4. Test against a running server

### Adding a New Error Code

1. Add to `contracts/error-codes.md` with HTTP status, description, and when it's used
2. Add the Rust constant in the codebase where it's used
3. Never invent error codes inline — they must be in the contract first

---

## Code Standards

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt                   # auto-format
cargo fmt --check           # CI check (no changes allowed)
```

### Linting

Zero clippy warnings are required:

```bash
cargo clippy --workspace -- -D warnings
```

Common clippy rules enforced:
- No `unwrap()` in production paths — use `?` or `Result`
- No unused imports
- No dead code
- Prefer `thiserror` for error types

### Naming Conventions

| Category | Convention | Examples |
|----------|-----------|---------|
| Structs / Enums | `PascalCase` | `User`, `ApiResponse<T>`, `CrustError` |
| Functions / Methods | `snake_case` | `get_user()`, `parse_pack()` |
| Constants | `UPPER_SNAKE_CASE` | `MAX_PACK_SIZE`, `DEFAULT_PORT` |
| Error codes | `UPPER_SNAKE_CASE` | `AUTH_INVALID_CREDENTIALS`, `REPO_NOT_FOUND` |
| File names | `snake_case.rs` | `object_store.rs`, `auth_middleware.rs` |
| API paths | `kebab-case` | `/api/v1/repos/:owner/:repo` |
| DB tables | `snake_case` | `users`, `repo_permissions` |
| JSON keys | `snake_case` | `user_id`, `created_at` |
| CLI commands | `lowercase` | `crust init`, `crust commit` |

### Error Handling

All public functions that can fail return `Result<T, E>`. No panics in production code.

All API responses use the `ApiResponse<T>` wrapper:

```rust
// Success
ApiResponse {
    success: true,
    data: Some(payload),
    error: None,
    metadata: ResponseMetadata { timestamp, duration, request_id },
}

// Error
ApiResponse {
    success: false,
    data: None,
    error: Some(ApiError {
        code: "REPO_NOT_FOUND".to_string(),   // from contracts/error-codes.md
        message: "Repository not found".to_string(),
        field: None,
    }),
    metadata: ResponseMetadata { ... },
}
```

**Never** expose stack traces in API responses.

### Async Boundaries

- `gitcore` — synchronous only. No `async`/`await`, no Tokio, no I/O.
- `crust-server` — fully async (Tokio + Axum).
- `crust-cli` — synchronous (blocking reqwest).

---

## Testing Requirements

Every change must have tests. A PR with no tests for new behaviour will be requested to add them.

### Test Levels

| Level | Crate | Command | Notes |
|-------|-------|---------|-------|
| Unit | `gitcore` | `cargo test -p gitcore` | No external deps, fully pure |
| Integration | `crust-server` | `cargo test -p crust-server` | Requires `DATABASE_URL` |
| End-to-end | `tests/` | `cargo test --test '*'` | Full stack |

### Before Submitting a PR

```bash
# All must pass
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

If `crust-server` tests require a database, set `DATABASE_URL` first:

```bash
export DATABASE_URL="postgres://crust_user:crust_password@localhost:5432/crust_test"
cargo test -p crust-server
```

### Object Format Tests

When modifying `gitcore`, verify that:
1. Object IDs are deterministic (`SHA256(header + content)` = same bytes → same ID)
2. Serialization round-trips cleanly (serialize → deserialize → re-serialize = identical bytes)
3. Tree entries are always sorted by name

---

## Pull Request Process

### Branch Naming

```
feat/<short-description>       # new feature
fix/<short-description>        # bug fix
docs/<short-description>       # documentation only
refactor/<short-description>   # no behaviour change
test/<short-description>       # tests only
```

### PR Checklist

Before requesting review, verify:

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Zero clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] Contracts updated if any system boundary changed
- [ ] Error codes in `contracts/error-codes.md` if new errors added
- [ ] No git libraries, no SSH, no SHA1, no zlib
- [ ] No `unwrap()` in production code paths
- [ ] No stack traces exposed in API responses

### PR Size

Keep PRs focused. One PR = one concern. Large PRs are hard to review and slow to merge.

If your change requires updating a contract AND implementing it, that's one PR (contract + implementation together, since they're coupled).

---

## Architecture Overview

Three crates in a Cargo workspace:

```
gitcore/        ← Pure Rust VCS core (no async, no network, no DB)
crust-server/   ← Axum HTTP server + PostgreSQL backend
crust-cli/      ← Standalone CLI binary (24 commands)
```

Both `crust-server` and `crust-cli` depend on `gitcore` as a library. `gitcore` has no knowledge of either.

Key files to understand the system:

- [.github/copilot-instructions.md](.github/copilot-instructions.md) — Full system description
- [contracts/api-contracts.md](contracts/api-contracts.md) — Every API endpoint
- [contracts/object-format.md](contracts/object-format.md) — On-disk format
- [contracts/crustpack-format.md](contracts/crustpack-format.md) — Wire protocol
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Design decisions

---

## Where to Start

### Good First Issues

- Implement one of the 8 scaffolded content-read endpoints (see `contracts/api-contracts.md`)
  - `GET /repos/:owner/:repo/refs` — list branches and tags
  - `GET /repos/:owner/:repo/blob/:ref/:path` — download a file
  - `GET /repos/:owner/:repo/commits/:ref` — commit history
- Add more unit tests to `gitcore` (merge edge cases, tree sorting, etc.)
- Improve error messages in the CLI

### Larger Projects

- Web UI (repo browser, PR interface)
- Webhooks / CI/CD integration
- CRUSTPACK delta compression (reduce transfer sizes)
- Commit signing
- Branch protection rules

---

## Questions

Check these in order:

1. [.github/copilot-instructions.md](.github/copilot-instructions.md) — system constraints and philosophy
2. [contracts/](contracts/) — authoritative specs for all boundaries
3. [reasoning/learning.md](reasoning/learning.md) — architectural decisions and past resolutions
4. Open an issue

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
