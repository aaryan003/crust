# requirements.md — CRUST v2

---

## IDENTITY

PRODUCT_NAME: crust
VERSION: 2.0

PRODUCT_DESCRIPTION: >
  CRUST is a fully original self-hosted version control system built from
  scratch in Rust. It is NOT a git clone. It is NOT git-compatible. It has
  its own object format (SHA256 + zstd), its own CLI client (crust push /
  crust pull / crust commit), its own wire protocol (HTTPS + JWT), and a
  full hosting platform (accounts, repos, orgs, teams, pull requests).
  Users never type "git". They type "crust". Deployed via docker compose up.

---

## TECH STACK

LANGUAGE:         Rust (edition 2021)
WORKSPACE:        Cargo workspace — 3 crates
ASYNC_RUNTIME:    Tokio
HTTP_FRAMEWORK:   Axum
PACKAGE_MANAGER:  cargo
DEPLOYMENT:       Docker + Docker Compose (app container + PostgreSQL container)
DATABASE:         PostgreSQL 16 via sqlx (async, compile-time checked queries)
AUTH:             JWT only — stored in ~/.crust/credentials after `crust login`
TRANSPORT:        HTTPS + JWT (no SSH, no pkt-line, no git wire protocol)
FRONTEND:         none (backend only — v1)
REALTIME:         none (v1)

---

## WHAT CRUST IS NOT (hard constraints)

These are non-negotiable. Every agent must read this section.

- NOT git-compatible. No .git/ directory. No SHA1. No zlib. No pkt-line.
- NOT using any git library: git2, gitoxide, gix are FORBIDDEN.
- NOT using SSH transport. russh is FORBIDDEN.
- NOT spawning a git binary anywhere (not in server, not in client, not in tests).
- Users do NOT type "git". They type "crust".

---

## CRATE STRUCTURE

### crate 1: gitcore (library)
- Pure Rust CRUST object model
- Zero async. Zero network. Zero database. Zero HTTP.
- Can be cargo test'd with no external services
- Everything version-control related lives here
- crust-server and crust-cli consume this as a path dependency

### crate 2: crust-server (binary)
- Axum HTTP server + Tokio async runtime
- REST API: auth, repos, orgs, teams, PRs
- Object transport endpoints (push/fetch via CRUSTPACK format)
- PostgreSQL for all platform data
- Disk for all object data (calls gitcore)
- Deployed inside Docker container

### crate 3: crust-cli (binary)
- Full VCS client — replaces git entirely for the user
- Reads/writes .crust/ in repos (NOT .git/)
- Reads/writes ~/.crust/ for global config + credentials
- Calls gitcore for all local VCS operations
- Calls crust-server via HTTPS for remote operations
- Distributed as a standalone binary users install

---

## CRUST OBJECT FORMAT SPECIFICATION

This is the canonical spec. gitcore implements this exactly.

### Object ID
- Algorithm: SHA256
- Format: 64 lowercase hex characters
- Computed over: full object bytes (header + content together)

### Object Storage (loose)
- Path: `.crust/objects/{id[0..2]}/{id[2..64]}`
- Content on disk: zstd-compressed object bytes (header + raw content)

### Object Header Format (text, before content)
```
CRUST-OBJECT\n
type: {blob|tree|commit|tag}\n
size: {raw_content_byte_length_as_decimal}\n
\n
{raw content bytes follow immediately}
```

### Blob
- Content: raw file bytes, no transformation

### Tree Entry Format (binary, sorted by name)
```
{mode_decimal} {name_utf8}\0{32_raw_sha256_bytes}
```
- Modes: 100644 (regular file), 100755 (executable), 040000 (directory), 120000 (symlink)
- Entries sorted by name (directories sort as "name/")

### Commit Format (text)
```
tree {sha256_hex}\n
parent {sha256_hex}\n        (zero lines for root commit, one+ for merge)
author {name} <{email}> {unix_ts} {tz_offset}\n
committer {name} <{email}> {unix_ts} {tz_offset}\n
\n
{commit message — rest of content}
```

### Tag Format (text)
```
object {sha256_hex}\n
type {blob|tree|commit|tag}\n
tag {tag_name}\n
tagger {name} <{email}> {unix_ts} {tz_offset}\n
\n
{tag message}
```

---

## CRUSTPACK FORMAT SPECIFICATION

Used for push (client→server) and fetch (server→client) transport.
NOT the git packfile format. Clean, human-readable headers, compressed bodies.

### Pack Header
```
CRUSTPACK\n
version: 1\n
count: {object_count_decimal}\n
\n
```

### Per Object (repeating, count times)
```
id: {sha256_hex_64chars}\n
type: {blob|tree|commit|tag}\n
size: {compressed_byte_count_decimal}\n
{zstd compressed bytes of full object (header + content)}
```
Note: no delta compression in v1. Each object stored in full. Delta is v2.

### Pack Trailer
- 32 raw bytes: SHA256 of all preceding pack bytes
- Validates pack integrity on receive

---

## LOCAL REPOSITORY STRUCTURE

When a user runs `crust init` or `crust clone`:

```
{project_dir}/
├── .crust/                      ← VCS data directory (NOT .git/)
│   ├── config                   ← repo config (key=value format)
│   │     remote.origin.url = https://server.com/owner/repo
│   │     branch.default = main
│   ├── HEAD                     ← "ref: refs/heads/main" or raw sha256
│   ├── index                    ← staging area (binary, see index spec)
│   ├── objects/                 ← loose object store
│   │   └── {2char}/{62char}     ← zstd-compressed object files
│   └── refs/
│       ├── heads/               ← one file per branch, contains sha256\n
│       └── tags/                ← one file per tag, contains sha256\n
└── {user files...}
```

---

## GLOBAL USER CONFIGURATION

Created on first `crust login`. Lives in user home directory.

```
~/.crust/
├── config                       ← global config (key=value)
│     user.name = Jane Smith
│     user.email = jane@example.com
│     default.server = https://crust.example.com
└── credentials                  ← JSON, one entry per server
      [
        {
          "server": "https://crust.example.com",
          "username": "jane",
          "token": "{jwt}",
          "expires_at": "{iso8601}"
        }
      ]
```

Token refresh: if token is within 1 hour of expiry, auto-refresh on next command.
If token is expired and cannot refresh: prompt re-login, do not proceed.

---

## CRUST-CLI COMMANDS (full spec)

### Bootstrap
```
crust init
  Creates .crust/ in current directory.
  Writes HEAD = "ref: refs/heads/main"
  Creates objects/, refs/heads/, refs/tags/, empty index.

crust login <server-url>
  Prompts: Username: / Password:
  POST /api/v1/auth/login
  Stores JWT in ~/.crust/credentials
  Prints: "Logged in as {username} on {server}"

crust logout [server-url]
  Removes credentials for server from ~/.crust/credentials

crust whoami
  Reads ~/.crust/credentials, prints current user + server
```

### Working Tree
```
crust status
  Compares: HEAD commit tree vs index vs working directory
  Shows: staged changes, unstaged changes, untracked files
  Output format:
    On branch main
    Changes staged for commit:
      new file: src/main.rs
    Changes not staged:
      modified: README.md
    Untracked:
      scratch.txt

crust add <path> [path...]
  Hashes file content → creates blob object in .crust/objects/
  Updates .crust/index entry for path with new blob id + metadata

crust add .
  Recursively stages all modified/new files in working directory

crust restore <path>
  Removes path from index (unstages). Does not touch working file.

crust diff
  Shows unstaged changes (working dir vs index)

crust diff --staged
  Shows staged changes (index vs HEAD commit tree)
```

### History
```
crust commit -m "<message>"
  Reads user.name + user.email from ~/.crust/config
  Builds tree object from current index
  Creates commit object pointing to tree + HEAD parent(s)
  Updates current branch ref to new commit SHA256
  Prints: "[main a3f9c12] Your message"

crust log
  Walks commit graph from HEAD, prints commits newest-first
  Format: commit {sha256}\n
          Author: {name} <{email}>\n
          Date: {human readable}\n
          \n
              {message}\n

crust log --oneline
  Format: {sha256[0..8]} {first line of message}

crust show <sha256-or-branch>
  Shows commit metadata + full diff from parent
```

### Branching (CRUST's native branching feature)
```
crust branch
  Lists all local branches. Current branch marked with *.

crust branch <name>
  Creates branch at current HEAD.
  Writes .crust/refs/heads/<name> with current commit SHA256.

crust branch -d <name>
  Deletes branch ref file. Refuses if current branch.

crust checkout <name>
  Switches HEAD to ref: refs/heads/<name>
  Updates working directory to match target branch's tree
  Refuses if there are uncommitted changes (unless --force)

crust checkout -b <name>
  Creates branch + switches to it in one step

crust merge <branch>
  Finds merge base (common ancestor) of current + target branch
  If target is ahead of current (fast-forward): move ref, done
  If diverged: 3-way merge via gitcore merge engine
  If conflicts: writes conflict markers, prints conflicting files
  If clean: auto-creates merge commit with message "Merge branch '<name>'"
```

### Remote Sync
```
crust clone <url> [directory]
  If directory not given: uses repo name from URL
  Creates directory, initializes .crust/
  Sets remote.origin.url in .crust/config
  Fetches all objects from server
  Checks out default branch

crust remote add <name> <url>
  Adds remote to .crust/config

crust remote list
  Lists remotes from .crust/config

crust fetch [remote]
  Remote defaults to "origin"
  GET /api/v1/repos/{owner}/{repo}/refs → get server ref list
  Computes: wants (server has, client doesn't), haves (client has)
  POST /api/v1/repos/{owner}/{repo}/objects/fetch → get CRUSTPACK
  Unpacks objects to .crust/objects/
  Updates .crust/refs/remotes/{remote}/* (does NOT merge into local branch)

crust pull [remote] [branch]
  Runs crust fetch
  Then merges remote tracking branch into current local branch

crust push [remote] [branch]
  Remote defaults to "origin"
  Reads local branch SHA256 and remote tracking SHA256
  POST /api/v1/repos/{owner}/{repo}/refs/preflight → server tells us what it needs
  Builds CRUSTPACK of missing objects
  POST /api/v1/repos/{owner}/{repo}/objects/upload → upload pack
  POST /api/v1/repos/{owner}/{repo}/refs/update → atomic ref update
  Prints: result per ref (ok / rejected + reason)
```

### Debug Commands
```
crust cat-object <id>        → decompress + print object content
crust hash-object <file>     → compute and print object ID for file
crust ls-tree <id>           → print tree entries
crust verify-pack            → check .crust/objects/ integrity
```

---

## SERVER API SPECIFICATION (overview — detail in contracts/api-contracts.md)

Base path: /api/v1
All responses: ApiResponse<T> wrapper
Auth header: Authorization: Bearer {jwt} (except /auth/register and /auth/login)

### Auth
POST   /api/v1/auth/register
POST   /api/v1/auth/login
POST   /api/v1/auth/logout
GET    /api/v1/auth/me

### Users
GET    /api/v1/users/:username
PATCH  /api/v1/users/me

### Repositories
POST   /api/v1/repos
GET    /api/v1/repos/:owner/:repo
PATCH  /api/v1/repos/:owner/:repo
DELETE /api/v1/repos/:owner/:repo
GET    /api/v1/repos/:owner/:repo/refs
GET    /api/v1/repos/:owner/:repo/tree/:ref?/:path?
GET    /api/v1/repos/:owner/:repo/blob/:ref/:path
GET    /api/v1/repos/:owner/:repo/commits/:ref?
GET    /api/v1/repos/:owner/:repo/commits/:sha
GET    /api/v1/repos/:owner/:repo/compare/:base...:head

### Object Transport (used by crust-cli internally)
POST   /api/v1/repos/:owner/:repo/refs/preflight
POST   /api/v1/repos/:owner/:repo/objects/upload
POST   /api/v1/repos/:owner/:repo/objects/fetch
POST   /api/v1/repos/:owner/:repo/refs/update

### Organizations
POST   /api/v1/orgs
GET    /api/v1/orgs/:org
POST   /api/v1/orgs/:org/members/:username
DELETE /api/v1/orgs/:org/members/:username
GET    /api/v1/orgs/:org/members

### Teams
POST   /api/v1/orgs/:org/teams
GET    /api/v1/orgs/:org/teams
PUT    /api/v1/orgs/:org/teams/:team/repos/:owner/:repo
POST   /api/v1/orgs/:org/teams/:team/members/:username

### Pull Requests
POST   /api/v1/repos/:owner/:repo/pulls
GET    /api/v1/repos/:owner/:repo/pulls
GET    /api/v1/repos/:owner/:repo/pulls/:number
PATCH  /api/v1/repos/:owner/:repo/pulls/:number
POST   /api/v1/repos/:owner/:repo/pulls/:number/reviews
POST   /api/v1/repos/:owner/:repo/pulls/:number/comments
POST   /api/v1/repos/:owner/:repo/pulls/:number/merge

### System
GET    /health

---

## DATABASE (PostgreSQL)

Tables: users, repositories, repo_permissions, organizations, org_members,
        teams, team_members, team_repos, pull_requests, pr_reviews,
        pr_comments, revoked_tokens

NOTE: No ssh_keys table. SSH is removed. Auth is JWT only.
NOTE: Git object data lives on DISK, not in database.
      database stores: users, permissions, repo metadata, PR data.
      disk stores: .crust object files organized by SHA256.

---

## PERMISSION MODEL

Per-repo access levels: owner > write > read
Resolution order for user accessing repo:
  1. Is user the repo owner (or org owner)? → full access
  2. Does user have direct repo_permission row? → use that role
  3. Is user in a team with team_repos entry for this repo? → use team permission
  4. Is repo public? → read access
  5. → 403 Forbidden

---

## BRANCHING STRATEGY (as a CRUST feature)

CRUST supports branching natively. Core branching behavior:
  - Every repo starts with one branch: main
  - Branches are lightweight refs (one file in refs/heads/)
  - Switching branches updates working directory
  - Merging: fast-forward first, 3-way merge if diverged
  - Conflict markers format:
      <<<<<<< ours\n
      {our content}
      =======\n
      {their content}
      >>>>>>> theirs\n
  - Force push allowed (no branch protection in v1)
  - Remote tracking branches: refs/remotes/{remote}/{branch}

---

## ALLOWED RUST CRATES

gitcore crate only:
  - sha2            (SHA256 — RustCrypto, no external deps)
  - zstd            (compression/decompression)
  - hex             (sha256 hex encoding)
  - thiserror       (error types)

crust-server adds:
  - tokio           (async runtime, features = ["full"])
  - axum            (HTTP, features = ["json"])
  - sqlx            (postgres, uuid, chrono features)
  - serde + serde_json
  - argon2          (password hashing)
  - jsonwebtoken    (JWT)
  - tracing + tracing-subscriber
  - uuid            (platform entity IDs)
  - chrono          (timestamps)
  - tower + tower-http (middleware)
  - anyhow          (error propagation in binary)

crust-cli adds:
  - reqwest         (HTTPS client, features = ["json", "blocking" or async])
  - clap            (CLI arg parsing, features = ["derive"])
  - rpassword       (secure password prompt — no echo)
  - dirs            (cross-platform home directory)
  - anyhow          (error propagation)
  - indicatif       (progress bars for push/fetch)

---

## SERVER DISK LAYOUT

```
/data/
└── repos/
    └── {owner}/
        └── {repo}.crust/        ← server-side bare object store
            ├── objects/
            │   └── {2char}/{62char}
            └── refs/
                ├── heads/
                └── tags/
```

No HEAD file on server bare repos (server tracks default branch in PostgreSQL).
No index file on server (index is a client-side concept).

---

## DEPLOYMENT

Single command: docker compose up -d

Services:
  - db:  postgres:16-alpine
  - app: CRUST server binary

Required env vars:
  DATABASE_URL         postgres connection string
  JWT_SECRET           min 64 chars random string
  REPO_BASE_PATH       /data/repos
  PORT                 8080 (default)
  JWT_EXPIRY_SECONDS   86400 (default, 24h)
  LOG_LEVEL            info (default)
  ALLOW_REGISTRATION   true (default — set false to disable public signup)

---

## DEVELOPMENT BRANCHING STRATEGY (how we BUILD crust)

Main branches:
  main    → always deployable, protected, never commit directly
  dev     → integration branch, PRs merge here first

Feature branches:
  feat/TASK-NNN-short-name    (e.g. feat/TASK-004-object-hashing)

Flow:
  1. Pick next task from reasoning/task-breakdown.md
  2. git checkout -b feat/TASK-NNN from dev
  3. Implement task
  4. cargo test --workspace must pass
  5. cargo clippy --workspace -- -D warnings must pass
  6. PR to dev
  7. Mark TASK-NNN as [x] COMPLETE in task-breakdown.md
  8. Move to next task

Tags: v0.1.0, v0.2.0 etc — cut from main when a phase is complete.
