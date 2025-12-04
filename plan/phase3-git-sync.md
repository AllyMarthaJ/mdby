# Phase 3: Git Sync & Collaboration Plan

## Overview

Phase 3 enables multi-user collaboration by implementing git sync with intelligent conflict resolution.

## Goals

1. Seamless push/pull with remote repositories
2. Automatic conflict detection
3. Document-aware merge strategies
4. Clear conflict resolution workflow

## Current State

- Git repository initialization works
- Automatic commits on each operation
- No remote synchronization
- Conflict resolution code exists but unused

---

## Task 1: Remote Configuration

**Priority:** High
**Effort:** Low

### Description
Manage remote repository configuration.

### Commands

```sql
-- Add/update remote
SET REMOTE origin 'git@github.com:user/db.git'

-- Remove remote
UNSET REMOTE origin

-- Show remotes
SHOW REMOTES
```

### Storage

```yaml
# .mdby/config.yaml
remotes:
  origin:
    url: git@github.com:user/db.git
    branch: main
  backup:
    url: /path/to/backup
    branch: main
sync:
  auto_push: false
  auto_pull: false
  conflict_strategy: merge_fields
```

### Implementation Steps

1. Add `Statement::SetRemote`, `Statement::UnsetRemote`, `Statement::ShowRemotes`
2. Add `Config` struct in `src/config.rs`
3. Save/load config from `.mdby/config.yaml`
4. Integrate with git remote commands

---

## Task 2: Pull Implementation

**Priority:** High
**Effort:** High

### Description
Fetch and merge remote changes.

### Algorithm

```
1. Fetch from remote
2. Get list of changed files between local HEAD and remote HEAD
3. For each changed file:
   a. If only remote changed: Fast-forward (accept remote)
   b. If only local changed: Keep local
   c. If both changed: Conflict resolution
4. Update local HEAD
5. Report results
```

### Conflict Detection

```rust
pub struct PullResult {
    pub pulled_commits: usize,
    pub updated_documents: Vec<String>,
    pub conflicts: Vec<Conflict>,
}

pub struct Conflict {
    pub document_path: String,
    pub collection: String,
    pub doc_id: String,
    pub local_version: Document,
    pub remote_version: Document,
    pub base_version: Option<Document>,
}
```

### Implementation Steps

1. Implement `git.fetch(remote)` in git module
2. Implement `git.diff_commits(local, remote)`
3. Implement `git.merge_file(path, strategy)`
4. Build `PullResult` with conflict list
5. Update `sync()` to return detailed results

---

## Task 3: Push Implementation

**Priority:** High
**Effort:** Medium

### Description
Push local commits to remote.

### Algorithm

```
1. Check if remote has new commits (pull first if needed)
2. Get local commits not on remote
3. Push commits to remote
4. Report results
```

### Push Result

```rust
pub struct PushResult {
    pub pushed_commits: usize,
    pub rejected: bool,
    pub rejection_reason: Option<String>,
}
```

### Implementation Steps

1. Implement `git.push(remote, branch)`
2. Handle push rejection (remote has new commits)
3. Implement force-push option (with warning)
4. Track push/pull state

---

## Task 4: Conflict Resolution Strategies

**Priority:** High
**Effort:** High

### Description
Implement multiple strategies for resolving document conflicts.

### Strategies

#### 1. Ours Wins
Keep local version, discard remote.

```rust
fn resolve_ours(ours: &Document, _theirs: &Document) -> Document {
    ours.clone()
}
```

#### 2. Theirs Wins
Keep remote version, discard local.

```rust
fn resolve_theirs(_ours: &Document, theirs: &Document) -> Document {
    theirs.clone()
}
```

#### 3. Merge Fields
Merge non-conflicting fields, mark conflicts.

```rust
fn resolve_merge_fields(
    base: Option<&Document>,
    ours: &Document,
    theirs: &Document,
) -> MergeResult {
    let mut merged = Document::new(&ours.id);
    let mut conflicts = Vec::new();

    // Get all field names
    let all_fields: HashSet<_> = ours.fields.keys()
        .chain(theirs.fields.keys())
        .collect();

    for field in all_fields {
        let ours_val = ours.fields.get(field);
        let theirs_val = theirs.fields.get(field);
        let base_val = base.and_then(|b| b.fields.get(field));

        match (ours_val, theirs_val, base_val) {
            // Both same - no conflict
            (Some(o), Some(t), _) if o == t => {
                merged.fields.insert(field.clone(), o.clone());
            }
            // Only ours changed from base
            (Some(o), Some(t), Some(b)) if t == b => {
                merged.fields.insert(field.clone(), o.clone());
            }
            // Only theirs changed from base
            (Some(o), Some(t), Some(b)) if o == b => {
                merged.fields.insert(field.clone(), t.clone());
            }
            // Both changed differently - conflict
            (Some(o), Some(t), _) => {
                conflicts.push(FieldConflict {
                    field: field.clone(),
                    ours: o.clone(),
                    theirs: t.clone(),
                });
                // Default to theirs
                merged.fields.insert(field.clone(), t.clone());
            }
            // Other cases...
        }
    }

    MergeResult { merged, conflicts }
}
```

#### 4. Concatenate Body
Merge fields, concatenate bodies with markers.

```rust
fn concatenate_body(ours: &Document, theirs: &Document) -> Document {
    let mut merged = resolve_merge_fields(None, ours, theirs).merged;

    merged.body = format!(
        "<<<<<<< LOCAL\n{}\n=======\n{}\n>>>>>>> REMOTE",
        ours.body,
        theirs.body
    );

    merged
}
```

#### 5. Manual Resolution
Mark file as conflicted, require user resolution.

---

## Task 5: Conflict Resolution UI

**Priority:** Medium
**Effort:** Medium

### Description
Interactive conflict resolution in REPL.

### Flow

```
mdql> SYNC

Pulling from origin...
Pulled 3 commits.

CONFLICT in todos/task-5.md

Local version:
  title: "Buy groceries (updated locally)"
  done: false

Remote version:
  title: "Buy groceries (updated remotely)"
  done: true

Resolution options:
  1. Keep local version
  2. Keep remote version
  3. Merge (keep local title, remote done)
  4. Edit manually

Choose [1-4]: 3

Resolved. 1 conflict resolved.

Pushing to origin...
Pushed 2 commits.
```

### Implementation Steps

1. Add conflict display formatting
2. Implement interactive prompts
3. Support batch resolution (apply same to all)
4. Add `RESOLVE CONFLICT <path> <strategy>` command

---

## Task 6: Branch Support

**Priority:** Low
**Effort:** Medium

### Description
Support git branches for isolated changes.

### Commands

```sql
-- Create branch
CREATE BRANCH feature-x

-- Switch branch
CHECKOUT feature-x

-- List branches
SHOW BRANCHES

-- Merge branch
MERGE BRANCH feature-x INTO main

-- Delete branch
DROP BRANCH feature-x
```

### Implementation Steps

1. Add AST nodes for branch commands
2. Implement git branch operations
3. Handle branch switching (reload collections)
4. Implement merge with conflict handling

---

## Task 7: Sync Status

**Priority:** Low
**Effort:** Low

### Description
Show sync status indicators.

### Status Command

```sql
SYNC STATUS
```

### Output

```
Remote: origin (git@github.com:user/db.git)
Branch: main

Local commits not pushed: 2
Remote commits not pulled: 1

Changed files (local):
  M collections/todos/task-1.md
  A collections/todos/task-6.md

Changed files (remote):
  M collections/todos/task-2.md
```

### Implementation Steps

1. Implement `git.local_ahead_count()`
2. Implement `git.remote_ahead_count()`
3. Implement `git.changed_files()`
4. Format status output

---

## Task 8: Auto-Sync Option

**Priority:** Low
**Effort:** Low

### Description
Optional automatic sync on operations.

### Configuration

```yaml
# .mdby/config.yaml
sync:
  auto_pull: true   # Pull before SELECT
  auto_push: true   # Push after INSERT/UPDATE/DELETE
```

### Implementation

```rust
async fn execute(&mut self, query: &str) -> Result<QueryResult> {
    let stmt = parse(query)?;

    // Auto-pull before read operations
    if self.config.auto_pull && is_read_operation(&stmt) {
        self.git.pull().await?;
    }

    let result = self.execute_ast(stmt).await?;

    // Auto-push after write operations
    if self.config.auto_push && is_write_operation(&stmt) {
        self.git.push().await?;
    }

    Ok(result)
}
```

---

## Error Handling

### Sync Errors

```rust
pub enum SyncError {
    RemoteNotConfigured,
    NetworkError(String),
    AuthenticationFailed,
    PushRejected { reason: String },
    ConflictsDetected { conflicts: Vec<Conflict> },
    MergeAborted,
}
```

### User Messages

```
Error: Push rejected - remote has new commits

Suggestion: Run PULL first to get remote changes, then push.
```

---

## Testing Plan

### Unit Tests
- [ ] Conflict detection logic
- [ ] Field merge algorithm
- [ ] Body concatenation
- [ ] Strategy selection

### Integration Tests
- [ ] Pull with no conflicts
- [ ] Pull with field conflicts
- [ ] Pull with body conflicts
- [ ] Push after pull
- [ ] Push rejection handling
- [ ] Branch operations

### Manual Testing
- [ ] Two instances modifying same doc
- [ ] Network failure recovery
- [ ] Large file sync performance

---

## Security Considerations

1. **Credentials** - Never store passwords in config, use git credential helpers
2. **Sensitive Data** - Warn about syncing .env or credentials files
3. **Remote Validation** - Validate remote URLs before use
4. **Hook Scripts** - Don't execute arbitrary git hooks

---

## File Changes

New files:
- `src/config.rs` - Configuration management
- `src/git/sync.rs` - Sync implementation (expand existing)
- `src/git/merge.rs` - Merge strategies

Modified files:
- `src/lib.rs` - Add Config to Database
- `src/git/mod.rs` - Add sync, branch operations
- `src/git/conflict.rs` - Expand resolution strategies
- `mdql/src/ast.rs` - Add sync-related statements
- `mdql/src/parser.rs` - Parse sync commands
- `src/main.rs` - Add sync command handling, conflict UI
