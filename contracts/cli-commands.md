# CLI Commands Specification

VERSION: 1.0.0
WRITTEN_BY: cli-agent
CONSUMED_BY: crust-cli implementation
LAST_UPDATED: 2026-03-04

All commands follow the pattern: `crust <command> [options] [arguments]`

Exit codes:
- 0: Success
- 1: User error (bad arguments, missing repo, etc.)
- 2: Runtime error (network, disk, server error)

---

## Bootstrap Commands

### crust init

Initialize a new CRUST repository in current directory.

```bash
crust init
```

Creates:
- `.crust/` directory
- `.crust/HEAD` (ref: refs/heads/main)
- `.crust/config` (empty, user can edit)
- `.crust/objects/` (directory)
- `.crust/refs/heads/` (directory)
- `.crust/refs/tags/` (directory)
- `.crust/index` (empty)

Output:
```
Initialized empty CRUST repository in ./.crust/
```

Errors:
- `CLI_INVALID_ARGUMENT`: Current directory doesn't exist
- Already exists: "Repository already exists at ./.crust"

---

### crust login <server-url>

Authenticate with a CRUST server and store credentials.

```bash
crust login https://crust.example.com
```

Prompts:
```
Username: jane_doe
Password: [hidden input]
```

Stores JWT in `~/.crust/credentials` (JSON format).

Output:
```
Logged in as jane_doe on https://crust.example.com
```

Errors:
- `AUTH_INVALID_CREDENTIALS`: Wrong username/password
- Network error: "Could not reach server"

---

### crust logout [server-url]

Remove credentials for a server.

```bash
crust logout https://crust.example.com
```

If no server specified, prompts to choose from available credentials.

Output:
```
Logged out from https://crust.example.com
```

---

### crust whoami

Show current authenticated user and server.

```bash
crust whoami
```

Output:
```
jane_doe @ https://crust.example.com
(token expires at: 2026-03-05 10:30:00 UTC)
```

Errors:
- `CLI_NOT_AUTHENTICATED`: No valid credentials

---

## Working Tree Commands

### crust status

Show state of working directory, index, and HEAD.

```bash
crust status
```

Output:
```
On branch main

Changes staged for commit:
  new file: src/main.rs
  modified: README.md

Changes not staged:
  modified: src/lib.rs
  deleted: old_file.txt

Untracked files:
  scratch.txt
  .DS_Store
```

Errors:
- `CLI_NO_REPOSITORY`: Not in a CRUST repo

---

### crust add <path> [path...]

Stage file(s) in the index.

```bash
crust add src/main.rs
crust add .  # stages all modified/new files
```

Computes SHA256 for each file, creates blob objects, updates `.crust/index`.

Output (per file):
```
added src/main.rs (blob: 3a7f8e9c...)
```

Errors:
- `CLI_NO_REPOSITORY`: Not in CRUST repo
- File not found: "src/main.rs: No such file or directory"

---

### crust restore <path>

Unstage file(s) from the index (doesn't modify working tree).

```bash
crust restore src/main.rs
```

Output:
```
unstaged src/main.rs
```

Errors:
- `CLI_NO_REPOSITORY`: Not in CRUST repo

---

### crust diff

Show unstaged changes (working directory vs index).

```bash
crust diff
```

Output (unified diff format):
```
diff --crust src/main.rs
index 3a7f8e9..abc1234
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
     println!("Hello");
+    println!("World");
 }
```

---

### crust diff --staged

Show staged changes (index vs HEAD commit tree).

```bash
crust diff --staged
```

Output: Unified diff format (as above)

---

## History Commands

### crust commit -m "<message>"

Create a commit from the staged index.

```bash
crust commit -m "Add authentication system"
```

Reads `user.name` and `user.email` from `~/.crust/config` (or prompts if not set).

Output:
```
[main 3a7f8e9] Add authentication system
 2 files changed, 42 insertions(+), 10 deletions(-)
```

Errors:
- `CLI_NO_REPOSITORY`: Not in CRUST repo
- `CLI_NOT_AUTHENTICATED`: No user.name/email configured
- `CLI_WORKING_TREE_DIRTY`: Refusing to commit if unresolved conflicts exist
- Empty message: "Commit message cannot be empty"

---

### crust log

Show commit history from HEAD, newest first.

```bash
crust log
```

Output:
```
commit 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
Author: Jane Smith <jane@example.com>
Date: Wed Mar 4 10:30:00 2026 +0000

    Add authentication system

    - Implement JWT login/register
    - Add token refresh logic

commit 1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1
Author: John Doe <john@example.com>
Date: Tue Mar 3 15:45:30 2026 +0000

    Initial commit
```

---

### crust log --oneline

Show commits in compact format.

```bash
crust log --oneline
```

Output:
```
3a7f8e9 Add authentication system
1b2c3d4 Initial commit
```

---

### crust show <sha-or-branch>

Show commit details + diff from parent.

```bash
crust show main
crust show 3a7f8e9
```

Output:
```
commit 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
Author: Jane Smith <jane@example.com>
Date: Wed Mar 4 10:30:00 2026 +0000

    Add authentication system

diff --crust a/src/auth.rs b/src/auth.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/src/auth.rs
@@ -0,0 +1,50 @@
+fn login() {
+    // auth code
+}
```

---

## Branching Commands

### crust branch

List all local branches.

```bash
crust branch
```

Output:
```
* main
  dev
  feat/auth
```

(* marks current branch)

---

### crust branch <name>

Create a new branch at current HEAD.

```bash
crust branch feat/new-feature
```

Output:
```
Created branch feat/new-feature
```

---

### crust branch -d <name>

Delete a branch (refuses if current branch).

```bash
crust branch -d feat/auth
```

Output:
```
Deleted branch feat/auth
```

---

### crust checkout <name>

Switch to a branch.

```bash
crust checkout dev
```

Output:
```
Switched to branch dev
Updated 5 files
```

Errors:
- `CLI_WORKING_TREE_DIRTY`: Cannot switch with uncommitted changes (unless --force)

---

### crust checkout -b <name>

Create and switch to a new branch.

```bash
crust checkout -b feat/new-feature
```

Output:
```
Switched to new branch feat/new-feature
```

---

### crust merge <branch>

Merge another branch into current branch.

```bash
crust merge dev
```

If fast-forward possible:
```
Fast-forward merge
Updated main to dev (1 new commit)
```

If 3-way merge:
```
Auto-merging src/main.rs
Merge made by the 3-way strategy.
 src/main.rs | 5 ++-
 1 file changed, 4 insertions(+), 1 deletion(-)
```

If conflicts:
```
CONFLICT (content): Merge conflict in src/main.rs
Auto-merge failed; fix conflicts and commit.

Conflicting files:
  src/main.rs
```

Conflict markers format:
```
<<<<<<< ours
our content
=======
their content
>>>>>>> theirs
```

Errors:
- `CLI_MERGE_IN_PROGRESS`: Already merging (resolve first)
- `CLI_CONFLICT_MARKERS`: File contains conflict markers (must resolve first)

---

## Remote Sync Commands

### crust clone <url> [directory]

Clone a repository from server.

```bash
crust clone https://crust.example.com/alice/my-project
crust clone https://crust.example.com/alice/my-project ./my-local-name
```

Creates directory, initializes `.crust/`, fetches all objects, checks out default branch.

Output:
```
Cloning into 'my-project'...
remote: Enumerating objects: 42, done
remote: Receiving objects: 100% (42/42), 1.2 MB | 1.2 MB/s, done
Checked out main branch
```

---

### crust remote add <name> <url>

Add a remote to `.crust/config`.

```bash
crust remote add upstream https://crust.example.com/upstream/project
```

Output:
```
Added remote 'upstream'
```

---

### crust remote list

Show all configured remotes.

```bash
crust remote list
```

Output:
```
origin    https://crust.example.com/alice/my-project
upstream  https://crust.example.com/upstream/project
```

---

### crust fetch [remote]

Fetch objects and ref updates from remote (doesn't merge).

```bash
crust fetch
crust fetch upstream
```

Output:
```
Fetching from origin...
remote: Enumerating objects: 5, done
remote: Receiving objects: 100% (5/5), 120 KB | 120 KB/s
Updated refs/remotes/origin/main to 3a7f8e9
Updated refs/remotes/origin/dev to 1b2c3d4
```

---

### crust pull [remote] [branch]

Fetch + merge remote branch into current local branch.

```bash
crust pull
crust pull origin main
```

Output:
```
Fetching from origin...
[as above]
Merging origin/main into main...
Fast-forward merge
Updated main to origin/main
```

---

### crust push [remote] [branch]

Push local branch to remote.

```bash
crust push
crust push origin main
```

Output:
```
Pushing to origin...
Preparing pack: 5 objects, 120 KB
Uploading...
Updating refs...
remote: 3a7f8e9..5c6d7e8 main -> main
Everything up to date
```

Errors:
- `CLI_NOT_AUTHENTICATED`: Not logged in to remote server
- `REF_CONFLICT`: Remote has diverged (need merge/rebase first)

---

## Debug Commands

### crust cat-object <id>

Decompress and print object content.

```bash
crust cat-object 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

Output:
```
CRUST-OBJECT
type: blob
size: 42

[raw content bytes]
```

---

### crust hash-object <file>

Compute object ID for a file without storing.

```bash
crust hash-object src/main.rs
```

Output:
```
3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

---

### crust ls-tree <id>

List tree entries in a tree object.

```bash
crust ls-tree 3a7f8e9c...
```

Output:
```
100644 blob 3a7f8e9c... README.md
100755 blob 1b2c3d4e... script.sh
040000 tree abc12345... src
```

---

### crust verify-pack

Check integrity of all objects in `.crust/objects/`.

```bash
crust verify-pack
```

Output:
```
Verifying 42 objects...
All objects OK
```

Errors:
- `OBJECT_CORRUPT`: Object failed validation

---

## Global Options

All commands support:
- `--help, -h`: Show help
- `--verbose, -v`: Verbose output
- `--config <file>`: Use alternate config file

Example:
```bash
crust --verbose push
crust --config ~/custom/.crust/config commit -m "msg"
```
