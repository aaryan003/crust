# CRUST CLI Guide

**Version**: 0.1.0  
**Binary**: `crust`  
**Config dir**: `~/.crust/`  
**Repo dir**: `.crust/` (inside every repo)

> CRUST is a fully self-hosted version control system. It is **not** git-compatible. Users type `crust`, not `git`.

---

## Quick Start

```bash
# 1. Log in to your CRUST server
crust login http://localhost:8080

# 2. Create a repo on the server (via API or UI), then clone it
crust clone http://localhost:8080/alice/my-project

# — OR — start a new local repo and push to existing remote
cd my-project
crust init
crust remote add origin http://localhost:8080/alice/my-project

# 3. Make changes, commit, push
crust add .
crust commit -m "Initial commit"
crust push
```

---

## Installation

The `crust` binary is built from the workspace:

```bash
cargo build --release -p crust-cli
# Binary at: target/release/crust

# Copy to PATH (optional)
cp target/release/crust /usr/local/bin/crust
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `--help`, `-h` | Show help for any command |
| `--version` | Print CRUST version |

---

## Command Reference

### Authentication

---

#### `crust login <server-url>`

Authenticate with a CRUST server and store credentials locally.

```bash
crust login http://localhost:8080
crust login https://crust.example.com
```

Prompts for username and password. Credentials are saved to `~/.crust/credentials` (JSON).

**Output:**
```
Username: alice
Password: [hidden]
Logged in as alice on http://localhost:8080
```

**Errors:**
- `AUTH_INVALID_CREDENTIALS` — wrong username or password
- Network error — server unreachable

---

#### `crust logout [server-url]`

Remove stored credentials for a server.

```bash
crust logout                          # auto-detects if only one server stored
crust logout http://localhost:8080
```

**Output:**
```
Logged out from http://localhost:8080
```

---

#### `crust whoami`

Show the currently authenticated user.

```bash
crust whoami
```

**Output:**
```
alice @ http://localhost:8080
Token expires at: 2030-01-01T00:00:00Z
```

**Exit code 1** if not logged in.

---

### Repository Initialization

---

#### `crust init`

Initialize a new CRUST repository in the current directory.

```bash
mkdir my-project && cd my-project
crust init
```

Creates the following structure:
```
.crust/
├── HEAD            # ref: refs/heads/main
├── config          # repo config (JSON)
├── index           # staging area
├── objects/        # object store (SHA256 + zstd)
└── refs/
    ├── heads/      # local branches
    └── tags/       # tags
```

**Output:**
```
Initialized empty CRUST repository in ./.crust/
```

---

### Working Tree Commands

---

#### `crust status`

Show the state of the working tree and staging area.

```bash
crust status
```

**Output:**
```
On branch main

Changes staged for commit:
  new file: src/main.rs
  modified: README.md

Changes not staged:
  modified: src/lib.rs

Untracked files:
  scratch.txt
```

---

#### `crust add <path> [path...]`

Stage one or more files (or `.` for all).

```bash
crust add README.md
crust add src/main.rs src/lib.rs
crust add .
```

Computes SHA256 for each file, creates blob objects, updates `.crust/index`.

**Output (per file):**
```
added README.md (blob: 3a7f8e9c...)
```

---

#### `crust restore <path>`

Unstage a file (remove from staging area, does **not** touch working tree).

```bash
crust restore README.md
```

**Output:**
```
unstaged README.md
```

---

#### `crust diff`

Show unstaged changes (working directory vs staging area).

```bash
crust diff
```

**Output (unified diff format):**
```diff
diff --crust src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@
 fn main() {
     println!("Hello");
+    println!("World");
 }
```

---

#### `crust commit -m "<message>"`

Create a commit from the staged index.

```bash
crust commit -m "Add README"
crust commit -m "Fix authentication bug"
```

Reads `user.name` and `user.email` from `~/.crust/config`.

**Output:**
```
[main 3a7f8e9] Add README
 1 files changed, 10 insertions(+)
```

---

### History Commands

---

#### `crust log`

Show commit history from HEAD, newest first.

```bash
crust log
```

**Output:**
```
commit 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
Author: Alice Smith <alice@example.com>
Date:   Wed Mar 4 10:30:00 2026 +0000

    Add authentication system

commit 1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0
Author: Alice Smith <alice@example.com>
Date:   Tue Mar 3 15:45:30 2026 +0000

    Initial commit
```

---

#### `crust log --oneline`

Compact one-line format.

```bash
crust log --oneline
```

**Output:**
```
3a7f8e9 Add authentication system
1b2c3d4 Initial commit
```

---

#### `crust show <sha-or-branch>`

Show a specific commit with full metadata.

```bash
crust show main
crust show 3a7f8e9
```

**Output:**
```
commit 3a7f8e9c...
Author: Alice Smith <alice@example.com>
Date:   Wed Mar 4 10:30:00 2026 +0000

    Add authentication system
```

---

### Branch Commands

---

#### `crust branch`

List all local branches. The current branch is marked with `*`.

```bash
crust branch
```

**Output:**
```
  feat/auth
* main
```

---

#### `crust branch <name>`

Create a new branch pointing to the current HEAD.

```bash
crust branch feat/new-login
crust branch fix/typo
```

**Output:**
```
Created branch feat/new-login
```

---

#### `crust branch -d <name>`

Delete a local branch (cannot delete the current branch).

```bash
crust branch -d feat/old-feature
```

**Output:**
```
Deleted branch feat/old-feature
```

---

#### `crust checkout <branch>`

Switch to an existing branch.

```bash
crust checkout feat/auth
```

**Output:**
```
Switched to branch feat/auth
```

---

#### `crust checkout -b <name>`

Create a new branch and switch to it immediately.

```bash
crust checkout -b feat/payments
```

**Output:**
```
Switched to new branch feat/payments
```

---

#### `crust merge <branch>`

Merge another branch into the current branch.

```bash
crust merge feat/auth
```

**Output (fast-forward):**
```
Fast-forward merge
Updated main to feat/auth
```

**Output (3-way merge):**
```
Auto-merging (simplified)...
Merge made by the 3-way strategy.
 1 file changed
```

---

### Remote Sync Commands

---

#### `crust remote add <name> <url>`

Register a remote URL.

```bash
crust remote add origin http://localhost:8080/alice/my-project
crust remote add upstream http://localhost:8080/upstream/project
```

**Output:**
```
Added remote 'origin'
```

---

#### `crust remote list`

List all configured remotes.

```bash
crust remote list
```

**Output:**
```
origin    http://localhost:8080/alice/my-project
upstream  http://localhost:8080/upstream/project
```

---

#### `crust clone <url> [directory]`

Clone a remote repository.

```bash
crust clone http://localhost:8080/alice/my-project
crust clone http://localhost:8080/alice/my-project ./local-name
```

Fetches all objects, initializes `.crust/`, checks out the default branch.

**Output:**
```
Cloning into 'my-project'...
remote: Enumerating objects: 1 refs
Downloaded objects
remote: Receiving objects: 100% (8/8), done.
Checked out branch 'main'.
```

---

#### `crust fetch [remote]`

Download new objects and update remote tracking refs (`refs/remotes/origin/*`). Does **not** merge into your local branch.

```bash
crust fetch
crust fetch upstream
```

**Output:**
```
Fetching from origin...
remote: Enumerating objects: 1 ref(s)
remote: Receiving objects: 100% (5/5), done.
Fetch complete.
```

After fetch, your local tracking ref (`.crust/refs/remotes/origin/main`) will point to the latest server commit.

---

#### `crust pull [remote] [branch]`

Fetch + merge the remote branch into your current local branch.

```bash
crust pull
crust pull origin main
```

**Output:**
```
Fetching from origin...
Already up to date.
Merging remotes/origin/main...
Merge made by the 3-way strategy.
```

---

#### `crust push [remote] [branch]`

Push local commits to the remote server.

```bash
crust push
crust push origin main
```

Collects all reachable objects (commits + trees + blobs), builds a CRUSTPACK, uploads, then updates the remote ref.

**Output:**
```
Pushing branch 'main' to origin...
Sending 8 object(s) (2004 bytes packed)...
Uploaded objects [████████████████████████████████] 2.00 KB/2.00 KB
remote: main -> 3a7f8e9c (8 objects)
```

**Errors:**
- `CLI_NOT_AUTHENTICATED` — not logged in
- `REPO_PERMISSION_DENIED` — no write access

---

### Debug Commands

---

#### `crust hash-object <file>`

Compute the CRUST object ID (SHA256) for a file without storing it.

```bash
crust hash-object README.md
```

**Output:**
```
3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

---

#### `crust cat-object <id>`

Decompress and print raw object content from the local object store.

```bash
crust cat-object 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

**Output:**
```
CRUST-OBJECT
type: blob
size: 42

[raw content]
```

---

#### `crust ls-tree <id>`

List entries in a tree object.

```bash
crust ls-tree abc1234def567890...
```

**Output:**
```
100644 blob 3a7f8e9c... README.md
100755 blob 1b2c3d4e... build.sh
040000 tree abc12345... src
```

Modes: `100644` = regular file, `100755` = executable, `040000` = directory (tree)

---

#### `crust verify-pack`

Verify the integrity of all objects in `.crust/objects/`.

```bash
crust verify-pack
```

**Output:**
```
Verifying 42 objects...
All objects OK
```

---

## Typical Workflows

### Start a New Project

```bash
# 1. Create repo on server via API
curl -X POST http://localhost:8080/api/v1/repos \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"my-project","is_public":false,"default_branch":"main"}'

# 2. Initialize locally
mkdir my-project && cd my-project
crust init
crust remote add origin http://localhost:8080/alice/my-project

# 3. First commit + push
echo "# My Project" > README.md
crust add README.md
crust commit -m "Initial commit"
crust push
```

---

### Day-to-Day Development

```bash
# Pull latest
crust pull

# Create feature branch
crust checkout -b feat/new-thing

# Make changes
vim src/lib.rs
crust add src/lib.rs
crust commit -m "Add new thing"

# Push branch
crust push

# Open a PR via API
curl -X POST http://localhost:8080/api/v1/repos/alice/my-project/pulls \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"Add new thing","head_ref":"feat/new-thing","base_ref":"main"}'
```

---

### Collaborate on Someone Else's Project

```bash
# Clone
crust clone http://localhost:8080/bob/their-project

cd their-project

# Check out a branch
crust checkout feat/auth

# View recent history
crust log --oneline

# Fetch latest without merging
crust fetch

# Merge remote changes
crust merge remotes/origin/main
```

---

## Configuration Files

### `~/.crust/credentials` (JSON)

```json
{
  "credentials": [
    {
      "server": "http://localhost:8080",
      "username": "alice",
      "token": "eyJ...",
      "expires_at": "2030-01-01T00:00:00Z"
    }
  ]
}
```

### `~/.crust/config` (JSON)

```json
{
  "user": {
    "name": "Alice Smith",
    "email": "alice@example.com"
  }
}
```

### `.crust/config` (JSON, per-repo)

```json
{
  "remotes": [
    {
      "name": "origin",
      "url": "http://localhost:8080/alice/my-project"
    }
  ]
}
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | User error (bad arguments, missing repo, wrong branch, etc.) |
| `2` | Runtime error (network failure, disk error, server error) |

---

## Object Format (Technical Reference)

Every CRUST object is stored as:

```
CRUST-OBJECT\n
type: {blob|tree|commit|tag}\n
size: {uncompressed_byte_count}\n
\n
{raw content bytes}
```

Object ID = `SHA256(header + content)` — 64-character hex string.

On disk: `.crust/objects/{id[0..2]}/{id[2..64]}` compressed with **zstd**.

> ⚠️ This is **not** git format. CRUST uses SHA256 (not SHA1) and zstd (not zlib).
