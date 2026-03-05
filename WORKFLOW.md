# CRUST Development Workflow

This document describes the development workflow for building CRUST.

## Overview

CRUST uses a **contract-first, task-driven** workflow powered by Claude and GitHub Copilot.

## Key Principles

1. **Contracts Before Code**: All system boundaries (API, DB, CLI, objects) are defined in `contracts/` before implementation
2. **Task Breakdown**: Complex work is broken into atomic tasks in `reasoning/task-breakdown.md`
3. **Agent-Driven**: Each task is owned by a specific agent (@contracts-agent, @backend-agent, @cli-agent, etc.)
4. **Dependency Graph**: Tasks declare their dependencies; parallelizable tasks run together
5. **Handoff Protocol**: When one task completes, a handoff note briefs the next agent

## Quick Start

### 1. Read the Context Files (in this order)
```bash
# Always-on system context
cat .github/copilot-instructions.md

# Current execution state
cat reasoning/task-breakdown.md

# Architectural decisions and lessons
cat reasoning/learning.md

# Contracts (single source of truth)
ls contracts/
```

### 2. Start a New Session
```
@main-agent
I'm ready to start building CRUST. What's the next task?
```

Main agent will:
- Check which tasks are complete
- Identify next incomplete task
- Check that all dependencies are met
- Tell you which agent to spawn

### 3. Spawn an Agent for the Task
```
@contracts-agent
SPAWNED_BY: main-agent
TASK: TASK-001 — Generate All Contracts
CONTEXT_FILES: requirements-v2.md
PRODUCES: contracts/*.rs, contracts/*.md
ACCEPTANCE_CRITERIA:
  - [ ] All entities have types
  - [ ] No {{PLACEHOLDER}} strings
HANDOFF_TO: TASK-002 (next task)
```

Agent will:
- Read all required context
- Do the work autonomously
- Generate a handoff note when done
- Provide handoff note to main agent

### 4. Update Task Status
After a task completes:
```bash
# Open reasoning/task-breakdown.md
# Find the task
# Change STATUS from [ ] PENDING to [x] COMPLETE
# Commit
```

### 5. Continue to Next Task
```
@main-agent
Task TASK-001 is complete. What's next?
```

## Workflow Diagram

```
┌─────────────────────────┐
│ @main-agent: Check      │
│ reasoning/task-*.md     │
│ Identify next task      │
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│ Spawn specific agent for task           │
│ @backend-agent / @cli-agent / etc.      │
│ Provide: TASK, DEPENDS_ON, PRODUCES     │
└────────────┬────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│ Agent does the work                     │
│ - Reads required contracts/context      │
│ - Implements code or generates contracts│
│ - Tests thoroughly                      │
│ - Writes handoff note                   │
└────────────┬────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│ @main-agent: Receives handoff note      │
│ - Updates task status [x] COMPLETE      │
│ - Logs learnings to learning.md         │
│ - Identifies next task                  │
└────────────┬────────────────────────────┘
             │
             ▼
       Repeat (Step 5+)
```

## Contract-First Process

Before ANY code is written:

1. **Check if contract exists**: `ls contracts/<name>`
2. **If YES**: Read it thoroughly before implementing
3. **If NO**: Stop. Request contracts-agent to create it first.

Example:
```
I need to add a new API endpoint.

→ Check contracts/api-contracts.md for the endpoint spec
→ Is it there?
  YES: Follow the spec exactly when implementing
  NO: Stop. Create the endpoint spec first (in contracts/api-contracts.md)
     Then implement code to match the spec
```

## Testing Before Handoff

Every task must pass tests before marking complete:

```bash
# For all crates
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

# For specific crate
cargo test -p gitcore
cargo test -p crust-server
cargo test -p crust-cli
```

## Common Tasks

### Add a New API Endpoint

1. Update `contracts/api-contracts.md` with endpoint spec (or create if missing)
2. Create the route in `src/routes/`
3. Implement handler calling gitcore/database as needed
4. Return `ApiResponse<T>` wrapper
5. Handle errors from `contracts/error-codes.md`
6. Write tests
7. Run `cargo test --workspace`
8. Mark endpoint [x] IMPLEMENTED in api-contracts.md

### Add a New CLI Command

1. Update `contracts/cli-commands.md` with command spec (or create if missing)
2. Create `src/commands/<command>.rs`
3. Implement using clap, gitcore, and HTTP client
4. Handle all error codes from `contracts/error-codes.md`
5. Write tests
6. Run `cargo test --workspace`
7. Verify help text: `crust <command> --help`

### Fix a Contract Bug

If you discover a contract is wrong:

1. Update the contract file (e.g., `contracts/api-contracts.md`)
2. Update VERSION field (e.g., 1.0.0 → 1.0.1)
3. Update LAST_UPDATED date
4. Update any affected implementation code
5. Commit both contract and code together
6. Document the change in `reasoning/learning.md`

## Debugging Workflow

### I'm stuck, what do I do?

1. **Check the contract**: Is the contract clear and complete?
2. **Check .github/copilot-instructions.md**: Does it answer your question?
3. **Check reasoning/learning.md**: Has this been solved before?
4. **Ask @main-agent**: Escalate to main agent for guidance

### Tests are failing

1. **Check error message**: What exactly is failing?
2. **Read the contract**: Does your code match the spec?
3. **Use cargo test --lib**: Debug with unit tests first
4. **Use cargo test --test**: Then integration tests

### Compiler errors

1. **Check imports**: Are you importing from contracts/?
2. **Check types**: Do types match contracts/data-types.rs?
3. **Check error codes**: Do errors come from contracts/error-codes.md?

## Progress Tracking

### Weekly Check-In
```bash
# Count completed tasks
grep "STATUS: \[x\] COMPLETE" reasoning/task-breakdown.md | wc -l

# Total tasks
grep "## TASK-" reasoning/task-breakdown.md | wc -l

# Percentage complete
echo "X of Y tasks complete (XX%)"
```

### Full Audit
```bash
# Check contracts vs. implementation
# See .github/prompts/contract-check.prompt.md
@contracts-agent
TASK: Full Contract Audit

Check that every contract in contracts/ is correctly implemented in src/.
Report any mismatches.
```

## Handoff Protocol

When completing a task, generate a handoff note:

```
TASK_COMPLETED: TASK-[NNN] — [Name]
AGENT: [your-agent]
STATUS: COMPLETE

PRODUCED:
- [file 1]: [what it contains]
- [file 2]: [what it contains]

TESTS_PASSING:
- [x] cargo test --workspace
- [x] cargo clippy
- [x] cargo fmt

NEXT_AGENT: [next-agent]
NEXT_TASK: TASK-[NNN] — [Next Task]

THEY_NEED_TO_KNOW:
- [critical detail 1]
- [critical detail 2]

BLOCKERS_NEW: (if any)
```

See `.github/prompts/handoff.prompt.md` for full template.

## Git Workflow (Optional)

If using git:

```bash
# Create feature branch
git checkout -b feat/TASK-[NNN]-short-name

# Work on task
# ... make changes, commit ...

# Before pushing, verify
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt

# Push
git push -u origin feat/TASK-[NNN]-short-name

# Make PR, get review
# Merge to dev (dev is integration branch)
# Delete feature branch

# When phase completes, tag
git tag -a v0.1.0 -m "Phase 1 complete"
git push origin v0.1.0
```

## Directory Structure Reminder

```
crust/
├── .github/
│   ├── copilot-instructions.md   ← Always-on context
│   ├── agents/                    ← Agent personas
│   ├── prompts/                   ← Reusable prompts
│   ├── skills/                    ← Skills library
│   ├── instructions/              ← Framework-specific guidance
│   └── workflows/                 ← CI/CD workflows
├── contracts/                     ← Single source of truth
│   ├── data-types.rs
│   ├── object-format.md
│   ├── crustpack-format.md
│   ├── db-schema.md
│   ├── error-codes.md
│   ├── api-contracts.md
│   ├── cli-commands.md
│   └── README.md
├── reasoning/
│   ├── task-breakdown.md          ← Current task list
│   └── learning.md                ← Architecture decisions
├── gitcore/                       ← VCS library (no async/network)
│   └── src/
├── crust-server/                  ← HTTP server (Axum + Tokio)
│   └── src/
├── crust-cli/                     ← CLI client
│   └── src/
├── docs/                          ← User documentation
├── Cargo.workspace.toml           ← Workspace definition
├── .env.example                   ← Environment variables
└── README.md                      ← Project overview
```

## Key Files to Know

- `.github/copilot-instructions.md` — Read this first, always
- `contracts/` — The truth; read before implementing
- `reasoning/task-breakdown.md` — Know where you are in the build
- `.github/agents/main-agent.agent.md` — How orchestration works
- `.github/prompts/handoff.prompt.md` — Handoff format
- `.github/prompts/contract-check.prompt.md` — Verify code vs. contracts

## Final Note

**Never skip reading the contracts.** They are the single source of truth. When in doubt, refer to contracts before code. Contracts-first prevents rework and keeps all agents in sync.

Happy building! 🚀
