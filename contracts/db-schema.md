# Database Schema Specification

VERSION: 1.0.0
WRITTEN_BY: contracts-agent
CONSUMED_BY: db-agent, backend-agent
LAST_UPDATED: 2026-03-04

## PostgreSQL 16

All tables use:
- `id`: UUID primary key (auto-generated)
- `created_at`: TIMESTAMP WITH TIME ZONE (server default: now())
- `updated_at`: TIMESTAMP WITH TIME ZONE (server default: now(), update trigger)

---

## users

Stores platform users and credentials.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| username | VARCHAR(255) | UNIQUE NOT NULL | Lowercase, alphanumeric + dash, 3-64 chars |
| email | VARCHAR(255) | UNIQUE NOT NULL | Valid email format |
| password_hash | VARCHAR(255) | NOT NULL | Argon2 hash, min 128 chars |
| display_name | VARCHAR(255) | NOT NULL | Can include spaces, emoji |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| is_active | BOOLEAN | NOT NULL DEFAULT true | Soft delete support |

**Indexes**:
- `CREATE INDEX idx_users_username ON users(username)`
- `CREATE INDEX idx_users_email ON users(email)`

**Lifecycle**: Deletion: set `is_active = false`, keep row

---

## organizations

Groups for managing teams and repositories.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| name | VARCHAR(255) | UNIQUE NOT NULL | Lowercase, alphanumeric + dash, 3-64 chars |
| display_name | VARCHAR(255) | NOT NULL | Can include spaces |
| description | TEXT | | Optional |
| owner_id | UUID | NOT NULL FK users(id) | User who created org |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Indexes**:
- `CREATE INDEX idx_orgs_owner_id ON organizations(owner_id)`
- `CREATE INDEX idx_orgs_name ON organizations(name)`

---

## org_members

Membership in organizations.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| org_id | UUID | NOT NULL FK organizations(id) | Org to join |
| user_id | UUID | NOT NULL FK users(id) | User joining |
| role | VARCHAR(50) | NOT NULL DEFAULT 'member' | 'owner', 'member' |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(org_id, user_id)`

**Indexes**:
- `CREATE INDEX idx_org_members_org ON org_members(org_id)`
- `CREATE INDEX idx_org_members_user ON org_members(user_id)`

---

## repositories

VCS repositories.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| owner_id | UUID | NOT NULL | User or Org ID (no FK enforced—could be either) |
| name | VARCHAR(255) | NOT NULL | Lowercase, alphanumeric + dash/underscore |
| display_name | VARCHAR(255) | NOT NULL | Human-readable name |
| description | TEXT | | Optional |
| is_public | BOOLEAN | NOT NULL DEFAULT false | Public read access |
| default_branch | VARCHAR(255) | NOT NULL DEFAULT 'main' | e.g., 'main' |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(owner_id, name)`

**Indexes**:
- `CREATE INDEX idx_repos_owner ON repositories(owner_id)`
- `CREATE INDEX idx_repos_public ON repositories(is_public)`

**Disk Mapping**: Object store at `/data/repos/{owner_id}/{repo_id}.crust/`

---

## repo_permissions

Explicit access grants (overrides ownership/org logic).

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| user_id | UUID | NOT NULL FK users(id) | User being granted access |
| repo_id | UUID | NOT NULL FK repositories(id) | Repo to access |
| permission | VARCHAR(50) | NOT NULL | 'owner', 'write', 'read' |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(user_id, repo_id)`

**Indexes**:
- `CREATE INDEX idx_repo_perms_user ON repo_permissions(user_id)`
- `CREATE INDEX idx_repo_perms_repo ON repo_permissions(repo_id)`

---

## teams

Groups within organizations for managing access.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| org_id | UUID | NOT NULL FK organizations(id) | Parent org |
| name | VARCHAR(255) | NOT NULL | Unique within org |
| display_name | VARCHAR(255) | NOT NULL | Human-readable |
| description | TEXT | | Optional |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(org_id, name)`

**Indexes**:
- `CREATE INDEX idx_teams_org ON teams(org_id)`

---

## team_members

Users in teams.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| team_id | UUID | NOT NULL FK teams(id) | Team to join |
| user_id | UUID | NOT NULL FK users(id) | User joining |
| role | VARCHAR(50) | NOT NULL DEFAULT 'member' | 'maintainer', 'member' |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(team_id, user_id)`

**Indexes**:
- `CREATE INDEX idx_team_members_team ON team_members(team_id)`
- `CREATE INDEX idx_team_members_user ON team_members(user_id)`

---

## team_repos

Repos assigned to teams with permission levels.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| team_id | UUID | NOT NULL FK teams(id) | Team accessing repo |
| repo_id | UUID | NOT NULL FK repositories(id) | Repo being accessed |
| permission | VARCHAR(50) | NOT NULL | 'read', 'write' |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(team_id, repo_id)`

**Indexes**:
- `CREATE INDEX idx_team_repos_team ON team_repos(team_id)`
- `CREATE INDEX idx_team_repos_repo ON team_repos(repo_id)`

---

## pull_requests

Pull requests in repositories.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| repo_id | UUID | NOT NULL FK repositories(id) | Repo this PR is for |
| number | INTEGER | NOT NULL | Seq per repo (not global) |
| title | VARCHAR(255) | NOT NULL | PR title |
| description | TEXT | | PR description |
| author_id | UUID | NOT NULL FK users(id) | User who opened PR |
| state | VARCHAR(50) | NOT NULL DEFAULT 'open' | 'open', 'merged', 'closed' |
| head_ref | VARCHAR(255) | NOT NULL | Source branch name |
| head_sha | VARCHAR(64) | NOT NULL | Commit SHA256 (hex) |
| base_ref | VARCHAR(255) | NOT NULL | Target branch name |
| base_sha | VARCHAR(64) | NOT NULL | Commit SHA256 (hex) |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Unique Constraint**: `UNIQUE(repo_id, number)`

**Indexes**:
- `CREATE INDEX idx_prs_repo ON pull_requests(repo_id)`
- `CREATE INDEX idx_prs_author ON pull_requests(author_id)`
- `CREATE INDEX idx_prs_state ON pull_requests(state)`

---

## pr_reviews

Code reviews on pull requests.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| pr_id | UUID | NOT NULL FK pull_requests(id) | PR being reviewed |
| user_id | UUID | NOT NULL FK users(id) | Reviewer |
| state | VARCHAR(50) | NOT NULL | 'pending', 'approved', 'requested_changes', 'commented' |
| body | TEXT | | Review comment |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Indexes**:
- `CREATE INDEX idx_pr_reviews_pr ON pr_reviews(pr_id)`
- `CREATE INDEX idx_pr_reviews_user ON pr_reviews(user_id)`

---

## pr_comments

Inline comments on diffs in PRs.

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| pr_id | UUID | NOT NULL FK pull_requests(id) | PR this comment is on |
| author_id | UUID | NOT NULL FK users(id) | Comment author |
| file_path | VARCHAR(500) | NOT NULL | File path in diff |
| line_number | INTEGER | NOT NULL | Line in file |
| body | TEXT | NOT NULL | Comment content |
| created_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |
| updated_at | TIMESTAMP | NOT NULL DEFAULT now() | UTC |

**Indexes**:
- `CREATE INDEX idx_pr_comments_pr ON pr_comments(pr_id)`
- `CREATE INDEX idx_pr_comments_author ON pr_comments(author_id)`

---

## revoked_tokens

Blacklist for invalidating JWTs (e.g., on logout).

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| id | UUID | PRIMARY KEY | Auto-generated |
| token_jti | VARCHAR(255) | UNIQUE NOT NULL | JWT `jti` claim |
| user_id | UUID | NOT NULL FK users(id) | User who held token |
| revoked_at | TIMESTAMP | NOT NULL DEFAULT now() | When revoked |
| expires_at | TIMESTAMP | NOT NULL | When token would expire anyway |

**Indexes**:
- `CREATE INDEX idx_revoked_tokens_user ON revoked_tokens(user_id)`
- `CREATE INDEX idx_revoked_tokens_expires ON revoked_tokens(expires_at)` — for cleanup

**Maintenance**: Periodically delete rows where `expires_at < now()` (optional, not required)

---

## Migrations

Use sqlx migrate or similar for version-controlled migrations.

File naming: `001_initial_schema.sql`, `002_add_teams.sql`, etc.

**First migration** (`001_initial_schema.sql`):
- Create all tables above
- Create all indexes
- Create trigger for `updated_at` on all tables:

```sql
CREATE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
-- ... repeat for all tables with updated_at
```

---

## Constraints Summary

- All foreign keys cascade on delete (consider implications)
- All unique constraints are indexed for performance
- `is_active` fields support soft deletes (rows remain for audit)
- Timestamps in UTC (PG default for TIMESTAMP WITH TIME ZONE)
- No custom sequences; UUIDs are primary identifiers

---

## Object Storage (NOT in Database)

Object data lives on disk, managed by gitcore:
- Location: `/data/repos/{owner_id}/{repo_id}.crust/objects/`
- NOT in PostgreSQL
- Database tracks object existence implicitly (current branch SHAs in PRs, commit logs)
- Garbage collection: Delete objects not referenced by any branch/tag/PR (future feature)
