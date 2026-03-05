# CRUST Quick Reference

**Last Updated**: 2026-03-04  
**Status**: ✅ Pre-Repo Complete

---

## 🚀 Start Here (5 minutes)

```
1. Open VS Code
2. Open Copilot Chat (Cmd+Shift+A or Ctrl+Shift+A)
3. Paste:

@main-agent

I'm ready to start building CRUST. What's the next task?

4. Follow agent's guidance
```

---

## 📚 Key Files (Read in This Order)

| # | File | Purpose | Read When |
|---|------|---------|-----------|
| 1 | `.github/copilot-instructions.md` | System context | Every session |
| 2 | `reasoning/task-breakdown.md` | Current tasks | Start of day |
| 3 | `docs/ARCHITECTURE.md` | System design | Getting started |
| 4 | `contracts/` | Boundaries | Before coding |
| 5 | `WORKFLOW.md` | Development flow | When stuck |

---

## 🎯 The Three-Crate Architecture

```
gitcore              (library)
  ├── Pure Rust
  ├── No async, network, or DB
  ├── Object types: Blob, Tree, Commit, Tag
  └── SHA256 hashing + zstd compression

crust-server        (binary)
  ├── Axum HTTP server
  ├── PostgreSQL database
  ├── REST API (60+ endpoints)
  └── JWT authentication

crust-cli           (binary)
  ├── Command-line client
  ├── Users type "crust" (not "git")
  ├── 25 commands
  └── Calls server via HTTPS + JWT
```

---

## ⚠️ Hard Constraints (Don't Violate These!)

❌ **FORBIDDEN**
- `git` libraries (git2, gitoxide, gix, russh)
- SSH transport
- Git binary invocation
- SHA1 hashing
- zlib compression
- Git packfile format
- Git format compatibility

✅ **REQUIRED**
- SHA256 hashing
- zstd compression
- CRUSTPACK format
- HTTPS + JWT
- Users type "crust"
- PostgreSQL
- Rust with Tokio + Axum

---

## 📋 Contracts (Single Source of Truth)

### Read Before Implementing

| Contract | Purpose | Size |
|----------|---------|------|
| `data-types.rs` | Shared types | 280 lines |
| `object-format.md` | Object spec | 450 lines |
| `crustpack-format.md` | Wire protocol | 350 lines |
| `db-schema.md` | Database | 400 lines |
| `error-codes.md` | Error codes | 350 lines |
| `api-contracts.md` | API endpoints | 800 lines |
| `cli-commands.md` | CLI commands | 600 lines |

**Rule**: If contract doesn't exist, create it before coding.

---

## 🧠 AI Agent Orchestration

### Main Agent (Orchestrator)
- Reads requirements
- Checks task status
- Identifies next task
- Spawns specialized agent
- Receives handoff

### Specialized Agents
- `contracts-agent` → Generates contracts
- `backend-agent` → Implements API + DB
- `gitcore-agent` → Implements VCS library
- `cli-agent` → Implements CLI client

### How to Spawn
```
@[agent-name]
SPAWNED_BY: main-agent
TASK: TASK-[NNN] — [Task Name]
PRODUCES: [list of files]
DEPENDS_ON: [dependencies]
```

---

## 🔄 Task Phases

| Phase | Tasks | Focus | Duration |
|-------|-------|-------|----------|
| 0 | TASK-001 | Contracts | 1 day |
| 1 | TASK-002 to 007 | Backend (scaffold, DB, auth, objects, repos, transport) | 5 days |
| 2 | TASK-008 to 012 | CLI (scaffold, working tree, history, remote, debug) | 4 days |
| 3 | TASK-013 to 014 | Platform (PRs, orgs/teams) | 3 days |
| 4 | TASK-015 to 017 | Integration, deployment, docs | 2 days |

---

## ✅ Pre-Commit Checklist

Before committing code:

```bash
# 1. Test everything
cargo test --workspace

# 2. No warnings
cargo clippy --workspace -- -D warnings

# 3. Format correct
cargo fmt --check

# 4. Verify contract match
# (use .github/prompts/contract-check.prompt.md)
```

---

## 🛠️ Common Tasks

### Add a New API Endpoint
1. Update `contracts/api-contracts.md`
2. Create route in `src/routes/`
3. Return `ApiResponse<T>`
4. Use error codes from `contracts/error-codes.md`
5. Test + commit

### Add a New CLI Command
1. Update `contracts/cli-commands.md`
2. Create `src/commands/<name>.rs`
3. Handle all error codes
4. Test + commit

### Fix a Contract
1. Update contract file
2. Update VERSION + LAST_UPDATED
3. Update affected code
4. Commit both together
5. Log in `reasoning/learning.md`

---

## 🧪 Testing Strategy

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p gitcore
cargo test -p crust-server
cargo test -p crust-cli

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name -- --exact
```

---

## 📊 Progress Tracking

### Check Status
```bash
# Count completed tasks
grep "STATUS: \[x\] COMPLETE" reasoning/task-breakdown.md | wc -l

# Total tasks
grep "## TASK-" reasoning/task-breakdown.md | wc -l

# Calculate percentage
(completed / total) * 100
```

### Update Status
1. Open `reasoning/task-breakdown.md`
2. Find task
3. Change `STATUS: [ ] PENDING` to `STATUS: [x] COMPLETE`
4. Commit

---

## 🚢 Deployment

### One Command
```bash
docker compose up -d
```

### What It Starts
- PostgreSQL (port 5432)
- CRUST Server (port 8080)

### Verify
```bash
curl http://localhost:8080/health
```

### Environment
- Copy `.env.example` → `.env`
- Fill in: DATABASE_URL, JWT_SECRET, PORT, etc.

---

## 🐛 Debugging

### Stuck? Try This
1. **Read the contract** — Is it clear?
2. **Check `.github/copilot-instructions.md`** — Does it answer?
3. **Check `reasoning/learning.md`** — Has this been solved?
4. **Ask `@main-agent`** — Need guidance?

### Tests Failing?
```bash
# Run specific test with output
cargo test failing_test -- --nocapture

# Check library-only tests
cargo test -p gitcore

# Check integration tests
cargo test --test '*'
```

### Compiler Errors?
- Check imports (should be from contracts/)
- Check types (should match contracts/data-types.rs)
- Check error codes (should be from contracts/error-codes.md)

---

## 🌳 Directory Structure

```
crust/
├── .github/
│   ├── copilot-instructions.md    ← MOST IMPORTANT
│   ├── agents/                     ← Agent personas
│   └── prompts/                    ← Reusable prompts
├── contracts/                      ← SINGLE SOURCE OF TRUTH
│   ├── data-types.rs
│   ├── object-format.md
│   ├── crustpack-format.md
│   ├── db-schema.md
│   ├── error-codes.md
│   ├── api-contracts.md
│   └── cli-commands.md
├── reasoning/
│   ├── task-breakdown.md           ← Task tracking
│   └── learning.md                 ← Decisions + lessons
├── docs/ARCHITECTURE.md            ← System design
├── gitcore/                        ← VCS library
├── crust-server/                   ← HTTP server
├── crust-cli/                      ← CLI client
├── WORKFLOW.md                     ← Dev workflow
├── README.md                       ← Project overview
└── .env.example                    ← Config template
```

---

## 💡 Key Concepts

### Contract-First
Read contract → Implement code → Test → Commit

### Deterministic Serialization
Same input → Same bytes always → Same SHA256 hash

### Three-Way Merge
Load three trees (ours, theirs, base) → Apply merge logic → Resolve conflicts

### ApiResponse Wrapper
Every API response: success bool + data + error + metadata

### CRUSTPACK
Objects packed for transport: header + count + objects + checksum

### Permission Hierarchy
Owner > Org Owner > Team > Direct > Public > None

---

## 🎓 Learning Resources

- **Quick Ref**: This document
- **Task List**: `reasoning/task-breakdown.md`
- **System Context**: `.github/copilot-instructions.md`
- **Architecture**: `docs/ARCHITECTURE.md`
- **Workflow**: `WORKFLOW.md`
- **Contracts**: `contracts/`

---

## 📞 Getting Help

### Immediate
- Read `.github/copilot-instructions.md`
- Search this file

### Short Term
- Check `reasoning/learning.md` for past decisions
- Check `contracts/` for specs
- Check `WORKFLOW.md` for examples

### Long Term
- Ask `@main-agent` in Copilot Chat
- Escalate blockers to main agent
- Update `reasoning/learning.md` with lessons

---

## ✨ Final Tips

1. **Always read contracts first** — They are the truth
2. **Test frequently** — `cargo test --workspace` before each commit
3. **Document decisions** — Update `reasoning/learning.md`
4. **Use task breakdown** — Know where you are
5. **Follow spawn protocol** — Consistent handoffs
6. **Verify no warnings** — `cargo clippy --all -- -D warnings`
7. **Keep formatting consistent** — `cargo fmt`

---

## 🚀 Ready?

```
@main-agent

I'm ready to start building CRUST. What's the next task?
```

---

**Last Updated**: 2026-03-04  
**Status**: ✅ Pre-Repository Complete
