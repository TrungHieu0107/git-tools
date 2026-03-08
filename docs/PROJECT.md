# GitHelper — Tài liệu Mô tả Dự án Chi tiết

> **Mục đích**: File này cung cấp mô tả chi tiết nhất về toàn bộ dự án GitHelper (git-tools), giúp AI Agent hoặc developer mới có thể hiểu nhanh kiến trúc, cấu trúc code, data flow, IPC contract, và quy tắc phát triển.

---

## 1. Tổng quan Dự án

**GitHelper** (tên nội bộ: `git-kraken-mini` hoặc `git-tools`) là một ứng dụng desktop GUI cho Git, tương tự GitKraken nhưng nhẹ hơn. Ứng dụng tập trung vào **tốc độ**, **tiêu thụ ít bộ nhớ**, và **hành vi có thể dự đoán được**.

### Nguyên tắc cốt lõi
- **Git chỉ được truy cập qua CLI hệ thống** (`git` binary) — KHÔNG dùng libgit2, KHÔNG dùng API
- **Không background daemon** — mọi thao tác Git được thực thi on-demand
- **Frontend không trực tiếp truy cập filesystem hoặc chạy git** — mọi thứ thông qua Tauri IPC
- **Mọi lỗi phải visible và traceable** — không swallow error, không silent failure

---

## 2. Technology Stack

| Layer | Công nghệ | Phiên bản |
|-------|-----------|-----------|
| **Frontend Framework** | Svelte 5 (Next) + TypeScript | ^5.0.0-next.1 |
| **Styling** | TailwindCSS | ^3.4.1 |
| **Bundler** | Vite | ^5.1.0 |
| **Backend/Desktop** | Rust + Tauri v2 | tauri 2.x |
| **IPC/Shell** | tauri-plugin-shell, tauri-plugin-dialog | ^2.0.0 |
| **Async Runtime** | Tokio (process, rt-multi-thread, time) | 1.x |
| **Git Execution** | `std::process::Command` (sync) + `tokio::process::Command` (async) | - |
| **Serialization** | Serde + serde_json | 1.x |
| **HTTP Client** | reqwest (for Gemini AI API) | 0.12 |
| **Encoding** | encoding_rs | 0.8 |
| **Error Handling** | thiserror | 1.x |
| **UUID** | uuid (v4) | 1.x |

---

## 3. Cấu trúc Thư mục Chi tiết

```
d:\learn\git-tools\
│
├── src/                          # Frontend Source (Svelte + TypeScript)
│   ├── main.ts                   # Entry point - mount App.svelte
│   ├── app.css                   # Global CSS
│   ├── App.svelte                # Root component - tab bar, repo manager, workspace routing
│   │
│   ├── components/               # UI Components
│   │   ├── Workspace.svelte      # Per-repo workspace (sidebar + main content area)
│   │   ├── CommitGraph.svelte    # [LỚN NHẤT ~3500 LOC] Commit graph visualization + commit details + diff
│   │   ├── CommitPanel.svelte    # Commit details panel (author, message, files)
│   │   ├── BranchExplorer.svelte # Branch list sidebar with search
│   │   ├── RepoManager.svelte    # Add/remove/select repository UI
│   │   ├── RepoSelector.svelte   # Dropdown-style repo quick-select
│   │   ├── ConflictEditor.svelte # Manual conflict editing (3-pane merge)
│   │   ├── ConflictList.svelte   # List of conflicted files
│   │   ├── Conflicts.svelte      # Parent component for conflict resolution flow
│   │   ├── GitCommandCenter.svelte # Raw git command console
│   │   ├── FileHistoryPanel.svelte # Git log for specific file
│   │   ├── SettingsView.svelte   # App settings (AI config, encoding, exclusions)
│   │   ├── TabBar.svelte         # Multi-repo tab bar at top
│   │   ├── TerminalPanel.svelte  # Embedded terminal (PowerShell)
│   │   ├── ToastContainer.svelte # Toast notification system
│   │   ├── GlobalConfirmation.svelte # Modal confirmation dialog
│   │   ├── CreateBranchDialog.svelte # Create/checkout new branch dialog
│   │   │
│   │   ├── commit/               # Commit-related sub-components
│   │   │   ├── CommitActions.svelte         # Stage/commit/push action buttons
│   │   │   ├── CommitFileList.svelte        # Changed files tree with staging controls
│   │   │   ├── ConflictResolveModal.svelte  # Full conflict resolution modal (Ours/Theirs/Manual)
│   │   │   └── GraphWipPanel.svelte         # Work-in-progress panel (unstaged/staged changes)
│   │   │
│   │   ├── diff/                 # Diff viewing components
│   │   │   ├── DiffView.svelte              # Diff view orchestrator (mode switch)
│   │   │   ├── DiffToolbar.svelte           # Diff mode toggle toolbar
│   │   │   ├── InlineDiffViewer.svelte      # Inline (unified) diff renderer
│   │   │   ├── SideBySideDiffViewer.svelte  # Side-by-side diff renderer
│   │   │   └── diff-viewer-types.ts         # Shared types for diff viewers
│   │   │
│   │   ├── rebase/               # Rebase operation components
│   │   │   ├── RebaseEditor.svelte          # Interactive rebase todo editor (pick/reword/squash/drop)
│   │   │   └── RebaseProgress.svelte        # Rebase progress overlay (step X/Y)
│   │   │
│   │   ├── blame/                # Git blame components
│   │   │   └── BlameView.svelte             # Git blame viewer for files
│   │   │
│   │   ├── common/               # Shared/reusable components
│   │   │   ├── BranchContextMenu.svelte     # Right-click context menu for branches
│   │   │   ├── CommitContextMenu.svelte     # Right-click context menu for commits
│   │   │   ├── StashCommitContextMenu.svelte # Context menu for stash entries
│   │   │   ├── FileChangeStatusBadge.svelte # M/A/D/R badge for file status
│   │   │   ├── PromptDialog.svelte          # Generic text input prompt dialog
│   │   │   ├── branch-context-menu-types.ts
│   │   │   ├── commit-context-menu-types.ts
│   │   │   └── stash-commit-context-menu-types.ts
│   │   │
│   │   └── resize/               # Resizable panel system
│   │       ├── ResizablePanel.svelte
│   │       ├── ResizablePanes.svelte
│   │       └── ResizableSection.svelte
│   │
│   └── lib/                      # Shared logic & services
│       ├── GitService.ts         # [CRITICAL] Unified IPC facade - 90+ methods mapping to Tauri commands
│       ├── GitCommandService.ts  # Low-level command executor wrapper
│       ├── types.ts              # TypeScript type definitions (mirrors Rust models)
│       ├── rebaseStore.ts        # Svelte store for rebase state machine
│       ├── toast.svelte.ts       # Toast notification store
│       ├── confirmation.svelte.ts # Confirmation dialog state
│       ├── create-branch-dialog.svelte.ts # Branch dialog state
│       ├── prompt.svelte.ts      # Prompt dialog state
│       ├── graph-layout.ts       # Graph topology calculation (lanes, connections, positions)
│       ├── graph-config.ts       # Graph rendering constants (ROW_HEIGHT, COLUMN_WIDTH, etc.)
│       ├── graph-colors.ts       # Color palette for graph lanes
│       ├── commit-graph-helpers.ts # Graph data transformation helpers
│       ├── diff.ts               # Diff parsing and computation logic
│       ├── diff-types.ts         # Diff-related type definitions
│       ├── diff-utils.ts         # Diff utility functions
│       ├── git-diff-parser.ts    # Raw git diff output parser
│       ├── lcs.ts                # Longest Common Subsequence algorithm (for inline diff)
│       ├── file-change.ts        # File change status parsing
│       ├── branch-utils.ts       # Branch name utilities
│       ├── action-utils.ts       # Action helper functions
│       ├── git.ts                # Git-related utilities
│       ├── git-errors.ts         # Error type definitions
│       │
│       ├── services/             # Service layer (thin IPC wrappers)
│       │   ├── RepositoryService.ts  # Repo CRUD operations
│       │   ├── BranchService.ts      # Branch operations (create, switch, merge, etc.)
│       │   ├── CommitService.ts      # Commit operations
│       │   ├── FileService.ts        # File operations (diff, stage, unstage, discard)
│       │   ├── ConflictService.ts    # Conflict detection and resolution
│       │   ├── TerminalService.ts    # Terminal session management
│       │   ├── command-executor.ts   # Generic command execution
│       │   └── invoke-shared.ts      # Shared invoke helpers
│       │
│       ├── stores/               # Svelte stores
│       │   └── (graph reload trigger, etc.)
│       │
│       └── components/           # Shared component utilities
│
├── src-tauri/                    # Backend Source (Rust/Tauri)
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri app config (window, permissions, etc.)
│   ├── build.rs                  # Tauri build script
│   │
│   ├── src/
│   │   ├── main.rs               # App entry point - Tauri setup, state init, command registration
│   │   ├── commands.rs           # [LỚN NHẤT ~3000 LOC] Core IPC command handlers + helpers
│   │   ├── settings.rs           # AppState, AppSettings, RepoEntry - persistence logic
│   │   ├── models.rs             # Shared data models (GitCommandOutput, DiffFile, etc.)
│   │   ├── git_engine.rs         # Legacy GitCommandService (async, uses tokio)
│   │   ├── terminal.rs           # TerminalManager - PowerShell session management
│   │   │
│   │   ├── commands/             # Modular command implementations
│   │   │   ├── ai_commands.rs        # Gemini AI commit message generation
│   │   │   ├── conflict_commands.rs  # Conflict detection, resolution, operation state
│   │   │   ├── diff_commands.rs      # Diff retrieval, line-level staging/unstaging
│   │   │   ├── rebase_commands.rs    # Rebase operations (start, continue, abort, skip, interactive)
│   │   │   ├── settings_commands.rs  # Settings CRUD (repos, encodings, filters, AI config)
│   │   │   └── terminal_commands.rs  # Terminal session IPC wrappers
│   │   │
│   │   └── git/                  # Git execution layer
│   │       ├── mod.rs            # Module exports
│   │       ├── service.rs        # GitExecutor - async git runner with timeout
│   │       ├── types.rs          # Git types (GitResponse, GitError, GitCommandResult, RebaseStatus, etc.)
│   │       └── encoding.rs       # File encoding detection and conversion
│   │
│   ├── capabilities/            # Tauri v2 security capabilities
│   └── icons/                   # App icons
│
├── index.html                   # Vite entry HTML
├── package.json                 # Frontend deps and scripts
├── vite.config.ts               # Vite configuration
├── tailwind.config.js           # TailwindCSS config
├── postcss.config.js            # PostCSS config
├── tsconfig.json                # TypeScript config
└── README.md                    # Project README
```

---

## 4. Kiến trúc Backend (Rust/Tauri)

### 4.1. Entry Point: `main.rs`

```
main() →
  1. GitExecutor::resolve_git_binary() — tìm git binary trên hệ thống
  2. tauri::Builder
     → plugin(tauri_plugin_shell)
     → plugin(tauri_plugin_dialog)
     → setup: tạo AppState, load settings từ JSON file
     → invoke_handler: đăng ký 100+ IPC commands
     → run()
```

### 4.2. State Management: `settings.rs`

```rust
struct AppState {
    settings: Mutex<AppSettings>,  // Thread-safe settings
    git: GitExecutor,              // Git command runner
    terminal: TerminalManager,     // PowerShell sessions
}

struct AppSettings {
    repos: Vec<RepoEntry>,                        // Danh sách repo đã thêm
    active_repo_id: Option<String>,               // Repo đang active
    open_repo_ids: Vec<String>,                    // Các repo đang mở (tabs)
    excluded_files: Vec<String>,                   // File patterns bị loại trừ
    repo_filters: HashMap<String, String>,         // Filter per repo
    file_encodings: HashMap<String, String>,       // Override encoding per file
    gemini_api_token: Option<String>,              // Gemini AI API key
    gemini_model: Option<String>,                  // AI model selection
    global_commit_prompt: Option<String>,           // Custom AI commit prompt
    repo_commit_prompts: HashMap<String, String>,  // Per-repo AI prompt
    repo_default_encodings: HashMap<String, String>, // Default encoding per repo
}

struct RepoEntry {
    id: String,     // UUID
    name: String,   // Display name
    path: String,   // Absolute filesystem path
}
```

**Persistence**: Settings được lưu dưới dạng JSON file tại `%APPDATA%/git-tools/settings.json`. Load khi khởi động, save sau mỗi thay đổi.

### 4.3. Git Execution Layer: `git/service.rs`

`GitExecutor` là lớp thực thi git chính:

| Method | Mô tả |
|--------|-------|
| `resolve_git_binary()` | Tìm git binary (PATH → well-known Windows paths) |
| `run(repo_path, args, timeout_secs)` | Chạy git async với timeout, trả về `GitResponse` |
| `run_with_env(repo_path, args, envs, timeout)` | Chạy git với biến môi trường (dùng cho rebase) |
| `run_with_output_bytes(repo_path, args, timeout)` | Trả raw bytes (dùng cho file content encoding) |
| `run_bare(args, timeout)` | Chạy git không cần repo dir (diagnostics) |
| `diagnostics()` | Thu thập thông tin git version, path, platform |

**Quan trọng**:
- Mọi lệnh git đều async, không block UI thread
- Có timeout protection (process bị kill nếu quá thời gian)
- Trên Windows: ẩn console window (`CREATE_NO_WINDOW` flag)
- stdout/stderr luôn được trả về client để xử lý

### 4.4. Commands Layer: `commands.rs` + `commands/`

Đây là API layer chính, expose các function cho frontend qua Tauri IPC. Hàm helper quan trọng:

```rust
fn resolve_repo_path(state, explicit_path) → Result<String, String>
fn git_run(state, repo_path, args, timeout) → Result<GitResponse, String>
fn git_run_result_with_event(app, state, ...) → Result<GitCommandResult, String>
fn emit_git_change_event(app) → Result<(), String>  // Notify frontend to refresh
```

### 4.5. Danh sách IPC Commands (100+ commands)

#### Settings & Repository Management
| Command | Mô tả |
|---------|-------|
| `cmd_get_settings` | Lấy toàn bộ settings |
| `cmd_add_repo` | Thêm repo mới (name + path) |
| `cmd_remove_repo` | Xóa repo khỏi danh sách |
| `cmd_set_active_repo` | Đặt repo active |
| `cmd_open_repo` | Mở repo (thêm vào tabs) |
| `cmd_close_repo` | Đóng repo (xóa khỏi tabs) |
| `cmd_get_active_repo` | Lấy repo đang active |
| `cmd_set_excluded_files` | Đặt danh sách file loại trừ |
| `cmd_set_repo_filter` | Đặt filter cho repo |
| `cmd_set_repo_default_encoding` | Đặt encoding mặc định cho repo |
| `cmd_set_file_encoding_override` | Override encoding cho file cụ thể |

#### AI Integration (Gemini)
| Command | Mô tả |
|---------|-------|
| `cmd_set_gemini_api_token` | Lưu API token |
| `cmd_set_gemini_model` | Chọn model |
| `cmd_get_gemini_models` | Liệt kê available models |
| `cmd_generate_commit_message` | Tạo commit message bằng AI (diff → Gemini API) |
| `cmd_get_default_ai_prompt` | Lấy default prompt template |
| `cmd_set_global_commit_prompt` | Đặt prompt tùy chỉnh global |
| `cmd_set_repo_commit_prompt` | Đặt prompt tùy chỉnh per-repo |

#### Git Core Operations
| Command | Mô tả |
|---------|-------|
| `run_git` | Chạy git command tùy ý (raw passthrough) |
| `cmd_git_status` | `git status` |
| `cmd_git_add` / `cmd_git_add_all` | Stage file / all |
| `cmd_git_unstage` / `cmd_git_unstage_all` | Unstage file / all |
| `cmd_git_commit` | Commit với message |
| `cmd_git_pull` / `cmd_git_push` / `cmd_git_fetch` | Sync operations |
| `cmd_git_checkout` | Checkout file/commit |
| `cmd_git_discard_changes` | Discard working tree changes |
| `cmd_git_log` | Raw git log |
| `cmd_get_commit_graph` | Parse commit graph → structured JSON |
| `cmd_diagnostics` | System diagnostics |

#### Branch Operations
| Command | Mô tả |
|---------|-------|
| `cmd_git_branch_list` / `cmd_get_git_branches` | List branches |
| `cmd_get_current_branch` | Current branch name |
| `cmd_git_switch_branch` | Switch branch |
| `cmd_git_create_branch` | Create new branch |
| `cmd_git_checkout_new_branch` | Create + checkout |
| `cmd_git_delete_branch` / `cmd_git_delete_remote_branch` | Delete branch |
| `cmd_git_rename_branch` | Rename branch |
| `cmd_git_merge` | Merge branch |
| `cmd_git_cherry_pick` | Cherry-pick commit |
| `cmd_git_revert` / `cmd_git_reset` | Revert/reset commit |
| `cmd_git_create_tag` | Create tag |
| `cmd_git_set_upstream` | Set upstream tracking |
| `cmd_get_branch_tip` | Get branch tip commit |

#### Conflict Resolution
| Command | Mô tả |
|---------|-------|
| `cmd_get_conflicts` | List conflicted file paths |
| `cmd_get_conflict_file` | Get base/ours/theirs content for a file |
| `cmd_resolve_ours` | Checkout --ours |
| `cmd_resolve_theirs` | Checkout --theirs |
| `cmd_mark_resolved` | `git add` to mark resolved |
| `cmd_check_conflict_state` | Check if conflicts exist |
| `cmd_get_operation_state` | Full operation state (merging/rebasing/cherry-picking + metadata) |
| `cmd_write_file` | Write manually merged content to file |

#### Rebase Operations
| Command | Mô tả |
|---------|-------|
| `cmd_get_rebase_status` | Rebase state (idle/inProgress/conflicted/editingTodo) |
| `cmd_rebase_start` | Start rebase onto branch |
| `cmd_rebase_interactive_prepare` | Get todo items for interactive rebase |
| `cmd_rebase_interactive_apply` | Apply interactive rebase with custom todo |
| `cmd_rebase_continue` / `cmd_rebase_abort` / `cmd_rebase_skip` | Continue/abort/skip |

#### Diff & Staging
| Command | Mô tả |
|---------|-------|
| `cmd_get_status_files` | Get changed files with staged/unstaged status |
| `cmd_get_diff_file` | Get diff patch for a file |
| `cmd_get_file_base_content` | Get base (HEAD) content of file |
| `cmd_get_file_modified_content` | Get modified (working/staged) content |
| `cmd_git_stage_line` | Stage individual line (partial staging) |
| `cmd_git_unstage_line` | Unstage individual line |

#### Stash Operations
| Command | Mô tả |
|---------|-------|
| `cmd_git_stash_file` / `cmd_git_stash_all` | Stash changes |
| `cmd_git_apply_stash` / `cmd_git_pop_stash` | Apply/pop stash |
| `cmd_git_delete_stash` | Delete stash entry |
| `cmd_git_edit_stash_message` | Edit stash message |
| `cmd_create_patch_from_stash` | Export stash as patch |

#### File Operations
| Command | Mô tả |
|---------|-------|
| `cmd_open_repo_file` | Open file in OS default app |
| `cmd_git_ignore_file` | Add to .gitignore |
| `cmd_show_in_folder` | Show in file explorer |
| `cmd_open_in_editor` | Open in configured editor |
| `cmd_open_in_diff_tool` | Open in configured diff tool |
| `cmd_create_patch` / `cmd_create_patch_from_commit` | Create patch files |
| `cmd_delete_file` | Delete file |
| `cmd_git_blame` | Git blame |
| `cmd_get_file_history` | File commit history |
| `cmd_search_repo_files` | Search files in repo |

#### Commit Details
| Command | Mô tả |
|---------|-------|
| `cmd_get_commit_diff` | Full diff for a commit |
| `cmd_get_file_at_commit` | File content at specific commit |
| `cmd_get_commit_changed_files` | List files changed in commit |
| `cmd_get_commit_file_diff` | Diff of specific file in commit |
| `cmd_get_pending_commits_count` | Commits ahead of remote |
| `cmd_abort_operation` | Abort current merge/rebase/cherry-pick |

#### Terminal
| Command | Mô tả |
|---------|-------|
| `cmd_terminal_start` | Start PowerShell session |
| `cmd_terminal_write` | Write to terminal stdin |
| `cmd_terminal_stop` | Stop terminal session |

---

## 5. Kiến trúc Frontend (Svelte 5)

### 5.1. Component Hierarchy

```
App.svelte
├── TabBar.svelte                    # Multi-repo tabs
├── RepoManager.svelte               # Overlay khi không có repo nào mở
├── Workspace.svelte (per repo)      # Mỗi repo mở = 1 Workspace
│   ├── BranchExplorer.svelte        # Sidebar trái - danh sách branch
│   ├── CommitGraph.svelte           # Tab "Graph" - commit history + details
│   │   ├── GraphWipPanel.svelte     # WIP (uncommitted changes) panel
│   │   ├── CommitFileList.svelte    # Changed files in selected commit
│   │   ├── CommitActions.svelte     # Stage/commit/push buttons
│   │   ├── DiffView.svelte          # Diff viewer orchestrator
│   │   │   ├── InlineDiffViewer.svelte
│   │   │   └── SideBySideDiffViewer.svelte
│   │   ├── CommitContextMenu.svelte # Right-click on commit
│   │   ├── BranchContextMenu.svelte # Right-click on branch
│   │   ├── StashCommitContextMenu.svelte
│   │   ├── ConflictResolveModal.svelte
│   │   ├── RebaseEditor.svelte      # Interactive rebase UI
│   │   └── RebaseProgress.svelte    # Rebase status overlay
│   ├── FileHistoryPanel.svelte      # Tab "History" - file log
│   ├── BlameView.svelte             # Tab "Blame"
│   ├── TerminalPanel.svelte         # Tab "Terminal"
│   └── SettingsView.svelte          # Tab "Settings"
├── GlobalConfirmation.svelte        # Global confirmation modal
├── CreateBranchDialog.svelte        # Global branch creation dialog
├── PromptDialog.svelte              # Global text prompt dialog
└── ToastContainer.svelte            # Global toast notifications
```

### 5.2. Tabs trong Workspace

Mỗi `Workspace.svelte` có 5 tab:
1. **Graph** (`"graph"`) — Commit graph visualization, commit details, diff view, WIP panel
2. **History** (`"history"`) — File-specific commit history
3. **Blame** (`"blame"`) — Git blame view
4. **Terminal** (`"terminal"`) — Embedded PowerShell
5. **Settings** (`"settings"`) — App settings

### 5.3. Service Layer: `GitService.ts`

`GitService` là singleton class tĩnh, đóng vai trò **facade duy nhất** cho mọi IPC call từ frontend đến backend:

```typescript
class GitService {
  // Settings
  static getSettings(): Promise<AppSettings>
  static addRepo(name, path): Promise<AppSettings>
  static removeRepo(id): Promise<AppSettings>
  static setActiveRepo(id): Promise<AppSettings>
  static openRepo(id): Promise<AppSettings>
  static closeRepo(id): Promise<AppSettings>
  
  // Git Operations
  static runGitCommand(args, repoPath?): Promise<GitCommandOutput>
  static getConflicts(repoPath?): Promise<string[]>
  static getConflictFile(path, repoPath?, encoding?): Promise<ConflictFile>
  static commit(message, repoPath?): Promise<GitCommandResult>
  static pull/push/fetch(repoPath?): Promise<GitCommandResult>
  static merge/rebase/cherryPick(target, repoPath?): Promise<GitCommandResult>
  
  // File Operations
  static getStatusFiles(repoPath?): Promise<FileStatus[]>
  static getDiffFile(path, staged, repoPath?, encoding?): Promise<string>
  static stageFile/unstageFile/discardChanges(path, repoPath?)
  static stageLine/unstageLine(path, line, repoPath?)
  
  // Graph
  static getCommitGraph(limit, filter?, repoPath?): Promise<string>  // Returns raw git log
  
  // AI
  static generateCommitMessage(repoPath?): Promise<string>
  
  // ... 90+ methods total
}
```

### 5.4. State Management

Frontend sử dụng nhiều cơ chế state:

| Store/State | File | Mô tả |
|-------------|------|-------|
| **Svelte 5 `$state`** | Component-local | Biến reactive trong component |
| **`rebaseStore`** | `rebaseStore.ts` | Svelte writable store - rebase state machine (idle → inProgress → conflicted → completed) |
| **`toast`** | `toast.svelte.ts` | Toast notification queue |
| **`confirmation`** | `confirmation.svelte.ts` | Global confirmation dialog state |
| **`createBranchDialog`** | `create-branch-dialog.svelte.ts` | Branch dialog open/close |
| **`prompt`** | `prompt.svelte.ts` | Generic prompt dialog |

### 5.5. Events & Real-time Updates

- **`git-event`**: Backend emit khi có thay đổi git (sau commit, rebase, merge, etc.) → Frontend listen và tự reload graph
- **`terminal-output`**: Backend emit output từ terminal session → Frontend hiển thị real-time
- **`repo-activated`**: Frontend CustomEvent khi user activate repo từ RepoManager
- **`close-repo-manager`**: Frontend CustomEvent đóng RepoManager overlay

---

## 6. Data Flow Chi tiết

### 6.1. Load Graph
```
User clicks "Graph" tab
  → Workspace.svelte: loadGraph()
    → GitService.getCommitGraph(limit, filter, repoPath)
      → invoke("cmd_get_commit_graph", { commitLimit, filter, repoPath })
        → commands.rs: cmd_get_commit_graph()
          → git log --oneline --graph --all --decorate --format=...
          → Parse output → return JSON
    → graph-layout.ts: calculateLayout(rawData)
      → Tính toán nodes, lanes, connections
    → CommitGraph.svelte: render SVG graph
```

### 6.2. Commit Changes
```
User writes message, clicks "Commit"
  → CommitActions.svelte: handleCommit()
    → GitService.commit(message, repoPath)
      → invoke("cmd_git_commit", { message, repoPath })
        → commands.rs: cmd_git_commit()
          → git commit -m "message"
          → emit_git_change_event() → triggers "git-event"
    → Frontend listens "git-event" → reload graph
```

### 6.3. Conflict Resolution
```
Merge/Rebase triggers conflicts
  → Backend: cmd_check_conflict_state → returns true
  → Workspace.svelte: hasConflicts = true → show conflict button
  → User clicks "Resolve Conflicts"
    → ConflictResolveModal opens
      → GitService.getConflicts() → list conflicted files
      → GitService.getConflictFile(path) → get base/ours/theirs
      → User chooses: resolveOurs() / resolveTheirs() / manual edit
      → GitService.markResolved(path) → git add
      → Repeat for all files, then continue/commit
```

### 6.4. Interactive Rebase
```
User right-clicks commit → "Interactive Rebase"
  → rebaseStore.prepareInteractive(baseCommit, repoPath)
    → invoke("cmd_rebase_interactive_prepare") → get todo items
    → rebaseStore.status = "editingTodo"
  → RebaseEditor.svelte renders todo list
  → User reorders/changes actions (pick/squash/drop/reword)
  → rebaseStore.applyInteractive()
    → invoke("cmd_rebase_interactive_apply", { baseCommit, todoItems })
      → Writes custom todo file → sets GIT_SEQUENCE_EDITOR → runs git rebase -i
    → checkRebaseStateAfterCommand() → sync status
```

---

## 7. Các Kiểu Dữ liệu Quan trọng

### Backend (Rust)

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

### Frontend (TypeScript) — mirrors backend types sử dụng `camelCase`

```typescript
type FileStatus = { path: string; status: string; staged: boolean; }
type ConflictFile = { base: string; ours: string; theirs: string; }
type AppSettings = { repos, active_repo_id, open_repo_ids, excluded_files, ... }
type GitCommandResult = { success, stdout, stderr, exitCode, commandType }
```

---

## 8. Graph Rendering System

### Pipeline
1. **Backend**: `cmd_get_commit_graph` → `git log --format=<custom>` → parse → return raw string
2. **Frontend parse**: `commit-graph-helpers.ts` → parse raw string into node objects
3. **Layout**: `graph-layout.ts` → calculate X/Y positions, lane assignments, connections
4. **Render**: `CommitGraph.svelte` → SVG rendering with virtual scrolling

### Graph Node Structure (Frontend)
```typescript
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

### Key Constants (`graph-config.ts`)
- `ROW_HEIGHT`: Height per commit row
- `COLUMN_WIDTH`: Width per lane column
- `PADDING_TOP/LEFT`: Graph padding
- `AVATAR_SIZE`: Author avatar size
- `STROKE_WIDTH`: Line thickness

---

## 9. Encoding System

### File: `git/encoding.rs`

Hỗ trợ custom encoding per-file và per-repo:
1. **Priority**: File override → Repo default → UTF-8
2. **Implementation**: Sử dụng `encoding_rs` crate
3. **Usage**: Khi đọc file content (conflict, diff, blame), encoding được áp dụng từ raw bytes

---

## 10. AI Integration (Gemini)

### File: `commands/ai_commands.rs`

- **Provider**: Google Gemini API
- **Default Model**: `gemini-2.5-flash`
- **Flow**: Staged diff → Truncate (40K chars max) → Build prompt → Call API → Sanitize response
- **Customization**: Global prompt hoặc per-repo prompt
- **Model Discovery**: List available models từ API

---

## 11. Terminal System

### File: `terminal.rs`

- Spawns `powershell -NoLogo -NoExit` per repo
- stdin/stdout/stderr piped
- Real-time output via Tauri events (`terminal-output`)
- Sessions keyed by repo path
- Console window hidden trên Windows

---

## 12. Quy tắc Phát triển (PHẢI TUÂN THỦ)

### KHÔNG ĐƯỢC:
- ❌ Tự tạo tính năng mới ngoài README
- ❌ Refactor kiến trúc, đổi framework
- ❌ Xóa logic, đổi tên command/event
- ❌ Dùng `unwrap()` trong production path
- ❌ Frontend trực tiếp truy cập filesystem hoặc chạy git
- ❌ Dùng stderr = error hoặc exit_code != 0 = fatal
- ❌ Swallow errors hoặc silent failures
- ❌ Auto-load data khi không có active repo

### PHẢI:
- ✅ Mọi git operation qua `std::process::Command` (Rust)
- ✅ Async execution, không block UI
- ✅ Return stdout + stderr + exit_code
- ✅ Validate repo path trước khi chạy lệnh
- ✅ Handle loading/empty/error states trong UI
- ✅ Mọi thay đổi phải minimal, localized, reversible
- ✅ Khi fix bug: xác định root cause → giải thích → minimum fix → verify

### Build & Run

```bash
# Development (hot-reload)
npm run tauri dev

# Production build
npm run tauri build
# Output: src-tauri/target/release/bundle/

# Type check
npm run check
```

---

## 13. File Size Reference

| File | Lines | Vai trò |
|------|-------|---------|
| `CommitGraph.svelte` | ~3,500 | Component phức tạp nhất — graph view, commit details, diff, context menus |
| `commands.rs` | ~3,000 | Backend command hub — 160+ functions |
| `Workspace.svelte` | ~440 | Per-repo workspace orchestrator |
| `CommitFileList.svelte` | ~1,800 | File tree with staging controls |
| `GraphWipPanel.svelte` | ~1,400 | WIP (uncommitted changes) panel |
| `ConflictResolveModal.svelte` | ~1,100 | Full conflict resolution UI |
| `GitService.ts` | ~440 | Frontend IPC facade |
| `git/service.rs` | ~490 | Async git executor |
| `ai_commands.rs` | ~510 | AI commit message generation |
| `diff_commands.rs` | ~530 | Diff & line-level staging |
| `rebase_commands.rs` | ~340 | Rebase operations |
| `conflict_commands.rs` | ~330 | Conflict detection & resolution |
| `rebaseStore.ts` | ~370 | Rebase state machine |
| `graph-layout.ts` | ~350 | Graph layout calculation |

---

## 14. Cách Thêm Feature Mới (Pattern)

### Backend:
1. Thêm function trong `commands/` hoặc `commands.rs`
2. Đăng ký trong `main.rs` → `invoke_handler`
3. Thêm type nếu cần trong `git/types.rs` hoặc `models.rs`

### Frontend:
1. Thêm method trong `GitService.ts` (hoặc service tương ứng)
2. Thêm/sửa component trong `components/`
3. Nếu cần state phức tạp → tạo store trong `lib/`

### IPC Contract:
- Backend sử dụng `#[tauri::command]` macro
- Frontend gọi via `invoke("command_name", { args })`
- Tất cả args dùng `camelCase` (serde auto-convert từ `snake_case`)
