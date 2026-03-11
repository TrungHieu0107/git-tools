# Frontend Component Structure

## Component Documentation

### App
**Path:** `src/App.svelte`
**Type:** Main Container / Layout
**Props:** N/A
**Responsibilities:** Orchestrates the multi-repository tab system, manages global dialogs (GlobalConfirmation, Toast), and switches between `Workspace` instances.
**Uses:** `TabBar`, `RepoManager`, `Workspace`, `GlobalConfirmation`, `ToastContainer`
**Tauri calls:** `cmd_get_settings`, `cmd_open_repo`, `cmd_set_active_repo`

### Workspace
**Path:** `src/components/Workspace.svelte`
**Type:** Page Controller
**Props:** `repoId (string)`, `repoPath (string)`, `isActive (boolean)`
**Responsibilities:** Manages the view-level routing (Graph vs. Terminal vs. History). Handles conflict detection and graph reloading orchestration.
**Uses:** `BranchExplorer`, `CommitGraph`, `TerminalPanel`, `BlameView`, `SettingsView`, `ResizablePanel`
**Tauri calls:** `cmd_check_conflict_state`, `cmd_get_commit_graph`

### CommitGraph
**Path:** `src/components/CommitGraph.svelte`
**Type:** UI Component
**Props:** `nodes (GraphNode[])`, `repoPath (string)`, `isActive (boolean)`, `pendingPushCount (number)`
**Responsibilities:** Renders the interactive Git log and graph. Handles commit selection and scrolls to specific hashes.
**Uses:** `CommitPanel`, `GraphWipPanel`
**Tauri calls:** `cmd_get_commit_changed_files`

### CommitPanel
**Path:** `src/components/CommitPanel.svelte`
**Type:** UI Component
**Props:** `repoPath (string)`, `selectedFile (FileStatus | null)`, `isActive (boolean)`
**Responsibilities:** The core of the staging and committing workflow. Manages diff loading, file staging, and commit action execution.
**Uses:** `CommitFileList`, `DiffView`, `ConflictResolveModal`
**Tauri calls:** `cmd_git_add`, `cmd_git_commit`, `cmd_get_diff_file`, `cmd_git_stage_line`

### BranchExplorer
**Path:** `src/components/BranchExplorer.svelte`
**Type:** UI Component
**Props:** `repoPath (string)`, `isActive (boolean)`
**Responsibilities:** Lists and filters branches. Provides context menus for branch-specific actions (checkout, merge, rebase).
**Uses:** `CreateBranchDialog`
**Tauri calls:** `cmd_get_git_branches`, `cmd_git_checkout`, `cmd_git_switch_branch`

### TerminalPanel
**Path:** `src/components/TerminalPanel.svelte`
**Type:** UI Component
**Props:** `repoPath (string)`, `isActive (boolean)`
**Responsibilities:** Provides an interactive shell interface for the active repository.
**Uses:** Monaco-based terminal emulator.
**Tauri calls:** `cmd_terminal_start`, `cmd_terminal_write`, `cmd_terminal_stop`

## Component Tree

- **App**
  - **TabBar**
  - **RepoManager**
    - RepoSelector
    - GlobalConfirmation
  - **Workspace** (multi-instance)
    - **ResizablePanel**
      - **BranchExplorer**
        - CreateBranchDialog
    - **CommitGraph**
      - **CommitPanel**
        - CommitFileList
        - CommitActions
        - DiffView
          - DiffToolbar
          - FileChangeStatusBadge
        - ConflictResolveModal
          - ConflictEditor
      - **GraphWipPanel**
    - **TerminalPanel**
    - **FileHistoryPanel**
    - **BlameView**
    - **SettingsView**
      - EncodingSelector
- **ToastContainer**
- **GlobalConfirmation**
- **PromptDialog**
