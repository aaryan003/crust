---
name: Agent Handoff
description: Generate structured handoff notes when one agent completes and another begins
---

# Handoff Prompt

Use this format when completing a task to hand off to the next agent.

## Template

```
TASK_COMPLETED: TASK-[NNN] — [Task Name]
AGENT: [your-agent-name]
STATUS: COMPLETE

PRODUCED:
- [file 1]: [what it contains, metrics]
- [file 2]: [what it contains, metrics]
- [contract updates]: [which contracts were modified/created]

TESTS_PASSING:
- [ ] cargo clippy --workspace -- -D warnings
- [ ] cargo test --workspace
- [ ] [custom test specific to this task]

UPDATED_CONTRACTS:
- [contract file]: [changes made, if any]

NEXT_AGENT: [agent-name]
NEXT_TASK: TASK-[NNN] — [Next Task Name]

THEY_NEED_TO_KNOW:
- [critical implementation detail 1]
- [critical implementation detail 2]
- [any deviations from original contract]
- [configuration they must use]

BLOCKERS_RESOLVED: [what was blocking before, now fixed]
BLOCKERS_NEW: [any new issues the next agent must watch for]

QUALITY_NOTES:
- [anything about code quality, tech debt, or architectural choices]
- [recommendations for the next phase]
```

## Example (Real)

```
TASK_COMPLETED: TASK-001 — Generate All Contracts
AGENT: contracts-agent
STATUS: COMPLETE

PRODUCED:
- contracts/data-types.rs: 40+ type definitions (User, Repo, Org, PR, etc.)
- contracts/object-format.md: Full CRUST object spec with examples
- contracts/crustpack-format.md: Wire format with pack structure, trailer checksum
- contracts/db-schema.md: 12 tables with foreign keys, indexes, cascade rules
- contracts/error-codes.md: 45 error codes with HTTP status mapping
- contracts/api-contracts.md: 60+ endpoint stubs (auth, repos, PRs, orgs, teams)
- contracts/cli-commands.md: 25 CLI commands with args, output, errors
- contracts/README.md: Ownership matrix (who writes/reads each contract)

TESTS_PASSING:
- [x] All contract files are complete (no {{PLACEHOLDER}})
- [x] All examples follow CRUST format (not git format)
- [x] All error codes are UPPER_SNAKE_CASE
- [x] All endpoints documented with Request/Response/Errors
- [x] No git library references

UPDATED_CONTRACTS:
- None (this was task 1)

NEXT_AGENT: main-agent (to spawn backend-agent)
NEXT_TASK: TASK-002 — Project Scaffold & Config

THEY_NEED_TO_KNOW:
- All objects use SHA256 (not SHA1)
- All API responses wrap data in ApiResponse<T> structure
- All error codes are defined in contracts/error-codes.md (never invent new ones)
- Database is PostgreSQL 16, use sqlx (compile-time checked queries)
- Three crates: gitcore (lib), crust-server (bin), crust-cli (bin)
- Cargo workspace in root
- No git libraries allowed
- No SSH transport (JWT only)

BLOCKERS_RESOLVED: (none — this was first task)
BLOCKERS_NEW: (none)

QUALITY_NOTES:
- Contracts are production-complete, ready for code generation
- Object format specification is exact to the byte (deterministic serialization)
- Error codes map cleanly to HTTP status codes (400, 401, 403, 404, 409, etc.)
- Database schema supports soft deletes and permission hierarchy
- API is versioned at /api/v1 (allows future /api/v2)
```

## When to Use This

- At the end of each TASK (when you mark [x] COMPLETE)
- When passing work to the next agent
- When multiple agents work in sequence
- For documentation of what was completed

## Key Fields

**TASK_COMPLETED**: Exact task name from task-breakdown.md

**STATUS**: Always "COMPLETE" for successful handoff

**PRODUCED**: Quantify — how many types, how many endpoints, etc.

**THEY_NEED_TO_KNOW**: Critical stuff the next agent MUST know to avoid rework
- Implementation deviations from spec
- Configuration they must use
- Performance considerations
- Security implications

**BLOCKERS_NEW**: Warn of any emerging issues (tech debt, architectural concerns)

**QUALITY_NOTES**: Observations about the work for the project learning log

---

## What Happens with This Note

1. Main agent reads it
2. Main agent updates task-breakthrough.md to mark task [x] COMPLETE
3. Main agent uses THEY_NEED_TO_KNOW to brief the next agent
4. Main agent files QUALITY_NOTES and BLOCKERS_NEW in reasoning/learning.md
5. Next agent reads the handoff note before starting
