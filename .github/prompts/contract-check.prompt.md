---
name: Contract Check
description: Verify implementation matches contracts before committing
---

# Contract Check Prompt

Run this before committing any code that crosses a system boundary (API, DB, CLI, object format).

## Checklist

### For Every API Call Your Code Makes

- [ ] Find the endpoint in `contracts/api-contracts.md`
- [ ] Verify HTTP method matches (GET, POST, PATCH, DELETE, etc.)
- [ ] Verify path matches exactly (including parameter names)
- [ ] Verify request body shape matches (if POST/PATCH/PUT)
- [ ] Verify response shape matches (JSON structure, field names)
- [ ] Verify you handle every error code listed in the contract for this endpoint

### For Every Type You Use

- [ ] Confirm it's imported from `contracts/data-types.rs`
- [ ] Confirm you haven't redefined or extended it in your module
- [ ] Confirm all fields match the contract (names, types)

### For Every Database Query

- [ ] Confirm the table name exists in `contracts/db-schema.md`
- [ ] Confirm all column names exist in the schema
- [ ] Confirm foreign key relations are correct
- [ ] Confirm indexes are used where schema specifies them

### For Every Error Returned

- [ ] Confirm the error code is in `contracts/error-codes.md`
- [ ] Confirm you're not inventing new error codes mid-implementation

### For Every CLI Command Implementation

- [ ] Confirm the command name matches `contracts/cli-commands.md`
- [ ] Confirm all arguments match the spec
- [ ] Confirm help text matches the spec
- [ ] Confirm exit codes (0=success, 1=user error, 2=runtime error)

### For Every Object Format Operation

- [ ] Confirm you're following `contracts/object-format.md` exactly
- [ ] Confirm object header format: `CRUST-OBJECT\ntype: X\nsize: N\n\n`
- [ ] Confirm tree entries are sorted by name
- [ ] Confirm commits include author/committer signatures
- [ ] Confirm SHA256 hashing (not SHA1)
- [ ] Confirm zstd compression (not zlib)

---

## Mismatch Resolution

If your implementation differs from the contract:

**Case 1: Contract is Right, Your Code is Wrong**
→ Fix your code to match the contract. That's the truth.

**Case 2: Contract is Wrong, Your Code is Right**
→ Update the contract file first, then commit both.
→ Also update the VERSION field in the contract (e.g., 1.0.0 → 1.0.1)
→ Write a note explaining why the contract changed

**Case 3: Contract is Missing**
→ STOP. Do not commit. Create the contract file first.
→ Identify which agent owns that contract (see contracts/README.md)
→ Submit the missing contract for review before committing code

**Case 4: Ambiguous**
→ Choose the interpretation that's closer to contracts/, not further away.
→ If still ambiguous, escalate to main-agent for clarification.

---

## Output

When you run this check, output one of:

### SUCCESS
```
✅ CONTRACT CHECK PASSED
All code matches contracts/
No violations found.
Ready to commit.
```

### FAILURE
```
❌ CONTRACT CHECK FAILED

Violations found:
1. API endpoint: POST /repos uses "repo_id" but contract specifies "id"
2. Error code: REPO_INVALID_NAME not in contracts/error-codes.md
3. Type: User.display_name field not in contracts/data-types.rs

Fix these before committing.
```

---

## When to Run

- Before every `git commit`
- Before every PR
- When reviewing another's code
- When feeling unsure about implementation details

This is your safety net. Use it liberally.
