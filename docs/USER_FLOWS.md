# User Flows & Use Cases

## 1. Repository Management
**Actor:** Developer
**Trigger:** Opening the application for the first time or wanting to work on a new project.
**Steps:**
1. App launches into `RepoManager` view if no repos are open.
2. User clicks "Add Repository" and selects a local folder via `tauri-plugin-dialog`.
3. `GitService.addRepo()` is called, which invokes `cmd_add_repo`.
4. User selects the repository from the list.
5. `GitService.openRepo()` adds the repo to the active tabs.
**Outcome:** A new tab is created in `TabBar`, and `Workspace` is initialized for the repo.
**Components involved:** `RepoManager`, `RepoSelector`, `TabBar`
**Tauri commands:** `cmd_add_repo`, `cmd_open_repo`, `cmd_get_settings`

## 2. Commit Workflow
**Actor:** Developer
**Trigger:** Saving changes in an external editor and wanting to commit.
**Steps:**
1. User selects the "Graph" tab in `Workspace`.
2. Developer clicks the "WIP" (Work In Progress) row in `CommitGraph`.
3. `CommitPanel` displays "Working Changes".
4. User reviews diffs in `DiffView`.
5. User clicks "+" on a file or "Stage All" to move changes to "Staged Changes".
6. User enters a commit message (or clicks "Generate" for AI help).
7. User clicks "Commit" or "Commit & Push".
**Outcome:** A new commit is created in the Git history; `CommitGraph` reloads.
**Components involved:** `CommitGraph`, `CommitPanel`, `CommitFileList`, `DiffView`
**Tauri commands:** `cmd_git_add`, `cmd_git_commit`, `cmd_generate_commit_message`, `cmd_git_push`

## 3. Conflict Resolution
**Actor:** Developer
**Trigger:** A "Merge" or "Rebase" operation results in conflicts.
**Steps:**
1. `Workspace` detects conflict state via `GitService.checkConflictState()`.
2. UI shows a "⚠ Conflicts" warning button in the sidebar.
3. User clicks the warning to focus the `CommitPanel` in conflict mode.
4. User selects a conflicted file (status "U").
5. `ConflictResolveModal` opens with a 3-way merge view (Base, Ours, Theirs).
6. User selects hunks from either side or edits the result manually.
7. User clicks "Mark as Resolved".
8. Once all files are resolved, user clicks "Commit and Continue".
**Outcome:** Conflict is resolved, and the original git operation completes.
**Components involved:** `Conflicts`, `ConflictResolveModal`, `ConflictEditor`, `CommitPanel`
**Tauri commands:** `cmd_get_conflicts`, `cmd_get_conflict_file`, `cmd_mark_resolved`, `cmd_rebase_continue`

## 4. Feature Branching
**Actor:** Developer
**Trigger:** Starting a new task.
**Steps:**
1. User opens `BranchExplorer` in the sidebar.
2. User clicks the "+" icon next to "Branches".
3. `CreateBranchDialog` appears.
4. User enters branch name and selects a base (e.g., `main`).
5. `GitService.createBranch()` invokes `cmd_git_create_branch`.
6. User switches to the new branch via `GitService.switchBranch()`.
**Outcome:** A new branch is created and checked out; `CommitGraph` updates its active leaf.
**Components involved:** `BranchExplorer`, `CreateBranchDialog`
**Tauri commands:** `cmd_git_create_branch`, `cmd_git_checkout`, `cmd_get_git_branches`
