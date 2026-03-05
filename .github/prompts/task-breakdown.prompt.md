---
name: Task Breakdown
description: Decompose any feature into dependency-aware tasks
---

# Task Breakdown Prompt

Use this to break down a complex feature into tasks that can be executed in parallel or sequence.

## Process

### Input: A Feature Description
Example: "Implement pull request review functionality"

### Step 1: Identify Contract Boundaries
What contracts must be created or updated?
- Database schema changes (e.g., pr_reviews table)
- API endpoints (e.g., POST /pulls/:number/reviews)
- Error codes (e.g., PR_ALREADY_REVIEWED)
- CLI commands (if CLI needs updates)
- Types (e.g., PullRequestReview struct)

### Step 2: Break into Atomic Tasks
Each task should be completable in one session (~2-4 hours).

Example breakdown for PR reviews:

```
TASK-[NNN] — Add PR Review to Contracts
  DEPENDS_ON: TASK-001 (contracts exist)
  PRODUCES: contracts/api-contracts.md (review endpoints)
            contracts/db-schema.md (pr_reviews table)
            contracts/data-types.rs (PullRequestReview type)
  AGENT: contracts-agent

TASK-[NNN+1] — Database Migration for PR Reviews
  DEPENDS_ON: TASK-[NNN]
  PRODUCES: migrations/add_pr_reviews_table.sql
  AGENT: backend-agent

TASK-[NNN+2] — PR Review API Endpoints
  DEPENDS_ON: TASK-[NNN+1]
  PRODUCES: src/routes/pr_reviews.rs (POST/GET endpoints)
  AGENT: backend-agent

TASK-[NNN+3] — PR Review CLI Commands
  DEPENDS_ON: TASK-[NNN+2]
  PRODUCES: src/commands/review.rs (crust review approve/request)
  AGENT: cli-agent
```

### Step 3: Identify Dependencies
- Which tasks must run first?
- Which tasks can run in parallel?
- Where are the bottlenecks?

### Step 4: Add to reasoning/task-breakdown.md
Insert the new tasks into the master task list in dependency order.

---

## Task Template

```markdown
## TASK-[NNN] — [Task Name]
STATUS: [ ] PENDING

AGENT:        [which agent runs this]
DEPENDS_ON:   [TASK-MMM, TASK-OOO]  (or "nothing" if none)
READS:        [which contract files to read first]
PRODUCES:     [exactly what files/contracts this task creates]
HANDOFF_TO:   [TASK-NNN — which task unlocks from this]

DESCRIPTION:
[What to build, detailed enough that the agent needs no clarification]

ACCEPTANCE_CRITERIA:
- [ ] [specific, testable criterion 1]
- [ ] [specific, testable criterion 2]
- [ ] [specific, testable criterion 3]

SPAWN_COMMAND:
[Exact text to paste into Copilot Chat]
```

---

## Example: Full Task Breakdown for Medium Feature

Feature: **Organization Management**

```markdown
## TASK-010 — Add Org to Contracts
STATUS: [ ] PENDING
AGENT: contracts-agent
DEPENDS_ON: TASK-001
PRODUCES: contracts/ (Organization, OrgMember types; org endpoints; orgs table)
HANDOFF_TO: TASK-011

## TASK-011 — DB Migration for Organizations
STATUS: [ ] PENDING
AGENT: backend-agent
DEPENDS_ON: TASK-010
PRODUCES: migrations/add_organizations.sql
HANDOFF_TO: TASK-012, TASK-013

## TASK-012 — Org Management API
STATUS: [ ] PENDING
AGENT: backend-agent
DEPENDS_ON: TASK-011
PRODUCES: src/routes/orgs.rs
HANDOFF_TO: TASK-014

## TASK-013 — Org Member Permissions Logic
STATUS: [ ] PENDING
AGENT: backend-agent
DEPENDS_ON: TASK-011
PRODUCES: src/permissions/org.rs (permission checking)
HANDOFF_TO: TASK-012, TASK-014  (can parallelize: both need this)

## TASK-014 — Org Commands (CLI)
STATUS: [ ] PENDING
AGENT: cli-agent
DEPENDS_ON: TASK-012, TASK-013
PRODUCES: src/commands/org.rs
HANDOFF_TO: (testing)

## TASK-015 — Integration Tests for Orgs
STATUS: [ ] PENDING
AGENT: backend-agent
DEPENDS_ON: TASK-014
PRODUCES: tests/orgs.rs
HANDOFF_TO: (done)
```

Notice:
- TASK-012 and TASK-013 both depend on TASK-011 but are independent (parallel)
- TASK-014 depends on both TASK-012 and TASK-013 (joins the branches)
- Each task is small enough to complete in one session

---

## Principles

1. **Atomic**: Each task is completable end-to-end in one session
2. **Ordered**: Dependencies are explicit (no surprises)
3. **Parallel-Friendly**: Independent tasks can run in parallel
4. **Handoff-Clear**: Next task is unambiguous
5. **Contract-First**: Contracts written before implementation tasks

---

## When to Use

- Breaking down a feature from requirements
- Planning a sprint
- Understanding the critical path
- Identifying bottlenecks
- Estimating total work

---

## Output

Your output is:
1. New tasks added to reasoning/task-breakdown.md
2. SPAWN_COMMAND for each task (ready to paste into Copilot)
3. Dependency diagram (text-based, showing which tasks can parallelize)

Example output diagram:
```
TASK-001 → TASK-002 → TASK-003 (sequential)
         ↘ TASK-004 ↙        (parallel: 2 and 4 both depend on 1)
             ↓
           TASK-005 (waits for both 3 and 4)
```

---

## Sanity Check

Before finalizing a task breakdown:
- [ ] Every PRODUCES file maps to exactly one task
- [ ] Every DEPENDS_ON is resolvable (task exists and comes before)
- [ ] No circular dependencies (A → B → C → A is invalid)
- [ ] Each task is <4 hours of work (otherwise break it down more)
- [ ] Contracts are created before implementation tasks
- [ ] All tasks are added to reasoning/task-breakdown.md
