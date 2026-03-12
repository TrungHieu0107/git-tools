
# Data Models
## Version: 1.0.0
## Last updated: 2026-03-13 – Initial split from PROJECT.md
## Project: GitHelper

## Backend (Rust) Models

```rust
// Git execution response
struct GitResponse {
    stdout: String,
    stderr: String,
    exit_code: i32,
    duration_ms: u64,
}

// Typed command result (for UI display)
struct GitCommandResult {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: i32,
    command_type: GitCommandType,  // Checkout/Merge/Rebase/etc.
}

// Conflict file content
struct ConflictFile {
    base: String,
    ours: String,
    theirs: String,
}

// Operation state
struct GitOperationState {
    is_merging: bool,
    is_rebasing: bool,
    is_cherry_picking: bool,
    is_reverting: bool,
    has_conflicts: bool,
    conflict_paths: Vec<String>,
    ours_commit/ours_branch/theirs_commit/theirs_branch: Option<String>,
    rebase_current/rebase_total: Option<usize>,
    rebase_message: Option<String>,
}

// Rebase types
enum RebaseStatus { Idle, InProgress, Conflicted, EditingTodo, Completed, Aborted }
struct FullRebaseStatus { status, step: Option<RebaseStepInfo>, onto_branch, upstream_branch }
struct RebaseTodoItem { action: String, hash: String, message: String }

// Diff types
struct DiffFile { path, status, hunks: Vec<DiffHunk> }
struct DiffHunk { id, old_start, new_start, lines: Vec<DiffLine> }
struct DiffLine { type_: DiffLineType, content, old_line_number, new_line_number }
enum DiffLineType { Context, Add, Remove }
```

## Frontend (TypeScript) Models

These models mirror the backend types using `camelCase`.

```typescript
type FileStatus = { path: string; status: string; staged: boolean; }
type ConflictFile = { base: string; ours: string; theirs: string; }
type AppSettings = { repos, active_repo_id, open_repo_ids, excluded_files, ... }
type GitCommandResult = { success, stdout, stderr, exitCode, commandType }

interface GraphNode {
    hash: string;
    abbreviatedHash: string;
    author: string;
    date: string;
    message: string;
    refs: string[];        // Branch/tag names
    parents: string[];     // Parent commit hashes
    x: number; y: number;  // Calculated positions
    color: string;         // Lane color
    lane: number;          // Lane assignment
}
```

## Changelog
- [2026-03-13] Extracted data_model.md from PROJECT.md
