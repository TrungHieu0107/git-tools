
# IPC API Reference
## Version: 1.0.0
## Last updated: 2026-03-13 – Initial split from PROJECT.md
## Project: GitHelper

## Frontend IPC Facade (`GitService.ts`)
`GitService` là singleton class tĩnh, đóng vai trò **facade duy nhất** cho mọi IPC call từ frontend đến backend:
```typescript
class GitService {
  // Settings
  static getSettings(): Promise<AppSettings>
  static addRepo(name, path): Promise<AppSettings>
  // ... 90+ methods total mapped to Tauri commands
}
```

## Backend IPC Commands

### Settings & Repository Management
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

### AI Integration (Gemini)
| Command | Mô tả |
|---------|-------|
| `cmd_set_gemini_api_token` | Lưu API token |
| `cmd_set_gemini_model` | Chọn model |
| `cmd_get_gemini_models` | Liệt kê available models |
| `cmd_generate_commit_message` | Tạo commit message bằng AI (diff → Gemini API) |
| `cmd_get_default_ai_prompt` | Lấy default prompt template |
| `cmd_set_global_commit_prompt` | Đặt prompt tùy chỉnh global |
| `cmd_set_repo_commit_prompt` | Đặt prompt tùy chỉnh per-repo |

### Git Core Operations
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

### Branch Operations
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

### Conflict Resolution
| Command | Mô tả |
|---------|-------|
| `cmd_get_conflicts` | List conflicted file paths |
| `cmd_get_conflict_file` | Get base/ours/theirs content for a file |
| `cmd_resolve_ours` | Checkout --ours |
| `cmd_resolve_theirs` | Checkout --theirs |
| `cmd_mark_resolved` | `git add` to mark resolved |
| `cmd_check_conflict_state` | Check if conflicts exist |
| `cmd_get_operation_state` | Full operation state (merging/rebasing/cherry-picking) |
| `cmd_write_file` | Write manually merged content to file |

### Rebase Operations
| Command | Mô tả |
|---------|-------|
| `cmd_get_rebase_status` | Rebase state (idle/inProgress/conflicted/editingTodo) |
| `cmd_rebase_start` | Start rebase onto branch |
| `cmd_rebase_interactive_prepare` | Get todo items |
| `cmd_rebase_interactive_apply` | Apply interactive rebase with custom todo |
| `cmd_rebase_continue` / `cmd_rebase_abort` / `cmd_rebase_skip` | Continue/abort/skip |

### Diff & Staging
| Command | Mô tả |
|---------|-------|
| `cmd_get_status_files` | Get changed files with staged/unstaged status |
| `cmd_get_diff_file` | Get diff patch for a file |
| `cmd_get_file_base_content` | Get base (HEAD) content of file |
| `cmd_get_file_modified_content` | Get modified (working/staged) content |
| `cmd_git_stage_line` | Stage individual line |
| `cmd_git_unstage_line` | Unstage individual line |

### Stash Operations
| Command | Mô tả |
|---------|-------|
| `cmd_git_stash_file` / `cmd_git_stash_all` | Stash changes |
| `cmd_git_apply_stash` / `cmd_git_pop_stash` | Apply/pop stash |
| `cmd_git_delete_stash` | Delete stash entry |
| `cmd_git_edit_stash_message` | Edit message |
| `cmd_create_patch_from_stash` | Export stash as patch |

### File Operations
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

### Commit Details
| Command | Mô tả |
|---------|-------|
| `cmd_get_commit_diff` | Full diff for a commit |
| `cmd_get_file_at_commit` | File content at specific commit |
| `cmd_get_commit_changed_files` | List files changed in commit |
| `cmd_get_commit_file_diff` | Diff of specific file in commit |
| `cmd_get_pending_commits_count` | Commits ahead of remote |
| `cmd_abort_operation` | Abort current merge/rebase/cherry-pick |

### Terminal
| Command | Mô tả |
|---------|-------|
| `cmd_terminal_start` | Start PowerShell session |
| `cmd_terminal_write` | Write to terminal stdin |
| `cmd_terminal_stop` | Stop terminal session |

## Events & Real-time Updates
- **`git-event`**: Backend emit khi có thay đổi git (sau commit, rebase, merge, etc.) → Frontend listen và tự reload graph
- **`terminal-output`**: Backend emit output từ terminal session → Frontend hiển thị real-time
- **`repo-activated`**: Frontend CustomEvent khi user activate repo từ RepoManager
- **`close-repo-manager`**: Frontend CustomEvent đóng RepoManager overlay

## Changelog
- [2026-03-13] Extracted api.md from PROJECT.md
