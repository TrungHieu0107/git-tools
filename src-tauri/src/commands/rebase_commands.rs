use super::*;
use std::path::Path;
use crate::git::{GitCommandResult, GitCommandType, GitError, RebaseStatus, RebaseStepInfo, RebaseTodoItem, FullRebaseStatus};

/// Helper: run a git command and return a `GitCommandResult` even when Git
/// reports conflicts or a non-zero exit code.  Only truly fatal errors
/// (e.g. spawning the process failed) are propagated as `Err`.
async fn git_run_rebase(
    state: &State<'_, AppState>,
    repo_path: &str,
    args: &[String],
    timeout: u64,
) -> Result<GitCommandResult, String> {
    match state
        .git
        .run(Path::new(repo_path), args, timeout)
        .await
    {
        Ok(resp) => {
            Ok(GitCommandResult {
                success: resp.exit_code == 0,
                stdout: resp.stdout,
                stderr: resp.stderr,
                exit_code: resp.exit_code,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(GitError::MergeConflict) => {
            // Conflict is NOT a fatal error for rebase — return it as a result
            Ok(GitCommandResult {
                success: false,
                stdout: String::new(),
                stderr: "CONFLICT: merge conflicts detected during rebase".into(),
                exit_code: 1,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(GitError::CommandError(msg)) => {
            // Git exited non-zero without CONFLICT keyword — still return as result
            // because the rebase may actually be in progress (paused, needing input, etc.)
            Ok(GitCommandResult {
                success: false,
                stdout: String::new(),
                stderr: msg,
                exit_code: 1,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Helper: run a git command with environment variables and return a
/// `GitCommandResult` even on conflicts / non-zero exit.
async fn git_run_rebase_with_env(
    state: &State<'_, AppState>,
    repo_path: &str,
    args: &[String],
    envs: Vec<(String, String)>,
    timeout: u64,
) -> Result<GitCommandResult, String> {
    match state
        .git
        .run_with_env(Path::new(repo_path), args, envs, timeout)
        .await
    {
        Ok(resp) => {
            Ok(GitCommandResult {
                success: resp.exit_code == 0,
                stdout: resp.stdout,
                stderr: resp.stderr,
                exit_code: resp.exit_code,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(GitError::MergeConflict) => {
            Ok(GitCommandResult {
                success: false,
                stdout: String::new(),
                stderr: "CONFLICT: merge conflicts detected during rebase".into(),
                exit_code: 1,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(GitError::CommandError(msg)) => {
            Ok(GitCommandResult {
                success: false,
                stdout: String::new(),
                stderr: msg,
                exit_code: 1,
                command_type: GitCommandType::Rebase,
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Rebase Status
// ---------------------------------------------------------------------------

pub async fn cmd_get_rebase_status_impl(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<FullRebaseStatus, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let p = Path::new(&path);
    let git_dir = p.join(".git");

    let rebase_merge = git_dir.join("rebase-merge");
    let rebase_apply = git_dir.join("rebase-apply");

    let is_rebasing = git_dir.join("REBASE_HEAD").exists() || rebase_merge.exists() || rebase_apply.exists();

    if !is_rebasing {
        return Ok(FullRebaseStatus {
            status: RebaseStatus::Idle,
            step: None,
            onto_branch: None,
            upstream_branch: None,
        });
    }

    let has_conflicts = cmd_check_conflict_state_internal(&state, &path).await?;
    
    let status = if has_conflicts {
        RebaseStatus::Conflicted
    } else {
        RebaseStatus::InProgress
    };

    let mut step_info = None;
    let mut onto_branch = None;
    let mut upstream_branch = None;

    if rebase_merge.exists() {
        let current = read_git_file(&git_dir, "rebase-merge/msg-num")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let total = read_git_file(&git_dir, "rebase-merge/end")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let commit_hash = read_git_file(&git_dir, "rebase-merge/stopped-sha")
            .unwrap_or_default();
        
        let commit_message = if !commit_hash.is_empty() {
             git_run(&state, Some(path.clone()), &["log", "-1", "--format=%s", &commit_hash], TIMEOUT_QUICK)
                .await
                .ok()
                .map(|r| r.stdout.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        step_info = Some(RebaseStepInfo {
            current,
            total,
            commit_hash,
            commit_message,
        });

        onto_branch = read_git_file(&git_dir, "rebase-merge/onto");
        upstream_branch = read_git_file(&git_dir, "rebase-merge/head-name")
            .and_then(|s| s.strip_prefix("refs/heads/").map(|b| b.to_string()));
    } else if rebase_apply.exists() {
        let current = read_git_file(&git_dir, "rebase-apply/next")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let total = read_git_file(&git_dir, "rebase-apply/last")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        step_info = Some(RebaseStepInfo {
            current,
            total,
            commit_hash: String::new(),
            commit_message: String::new(),
        });
    }

    Ok(FullRebaseStatus {
        status,
        step: step_info,
        onto_branch,
        upstream_branch,
    })
}

async fn cmd_check_conflict_state_internal(
    state: &State<'_, AppState>,
    repo_path: &str,
) -> Result<bool, String> {
    let resp = git_run(
        state,
        Some(repo_path.to_string()),
        &["status", "--porcelain"],
        TIMEOUT_LOCAL,
    )
    .await?;

    for line in resp.stdout.lines() {
        if line.len() >= 2 {
            let status = &line[0..2];
            if matches!(status, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn read_git_file(git_dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(git_dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Rebase Operations (all return GitCommandResult, never Err on conflicts)
// ---------------------------------------------------------------------------

pub async fn cmd_rebase_start_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    base: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args = vec!["rebase".into(), base];
    let result = git_run_rebase(&state, &path, &args, TIMEOUT_LOCAL).await?;
    let _ = emit_git_change_event(&app);
    Ok(result)
}

pub async fn cmd_rebase_interactive_prepare_impl(
    state: State<'_, AppState>,
    base_commit: String,
    repo_path: Option<String>,
) -> Result<Vec<RebaseTodoItem>, String> {
    let args = vec!["log".into(), format!("{}..HEAD", base_commit), "--reverse".into(), "--format=%h\t%s".into()];
    let resp = git_run(&state, repo_path, &args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(), TIMEOUT_LOCAL).await?;
    
    let mut items = Vec::new();
    for line in resp.stdout.lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            items.push(RebaseTodoItem {
                action: "pick".to_string(),
                hash: parts[0].to_string(),
                message: parts[1].to_string(),
            });
        }
    }
    
    Ok(items)
}

pub async fn cmd_rebase_interactive_apply_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    base_commit: String,
    todo_items: Vec<RebaseTodoItem>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    
    // Create the todo content
    let mut todo_content = String::new();
    for item in todo_items {
        todo_content.push_str(&format!("{} {} {}\n", item.action, item.hash, item.message));
    }

    // Write the todo content to a temporary file
    let temp_dir = std::env::temp_dir();
    let todo_file = temp_dir.join(format!("git-rebase-todo-{}", uuid::Uuid::new_v4()));
    std::fs::write(&todo_file, todo_content).map_err(|e| e.to_string())?;

    // Create a script that replaces the todo file git provides with our one
    #[cfg(target_os = "windows")]
    let script_content = format!("copy /y \"{}\" \"%1\"", todo_file.to_string_lossy().replace("/", "\\"));
    #[cfg(not(target_os = "windows"))]
    let script_content = format!("cp \"{}\" \"$1\"", todo_file.to_string_lossy());

    let script_file = temp_dir.join(format!("git-rebase-editor-{}", uuid::Uuid::new_v4()));
    #[cfg(target_os = "windows")]
    let script_file = script_file.with_extension("bat");
    
    std::fs::write(&script_file, &script_content).map_err(|e| e.to_string())?;

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }

    let env_name = "GIT_SEQUENCE_EDITOR";
    let env_value = script_file.to_string_lossy().to_string();
    let args = vec!["rebase".into(), "-i".into(), base_commit];
    
    let result = git_run_rebase_with_env(
        &state,
        &path,
        &args,
        vec![(env_name.to_string(), env_value)],
        TIMEOUT_LOCAL,
    ).await;
        
    // Cleanup temp files regardless of result
    let _ = std::fs::remove_file(&todo_file);
    let _ = std::fs::remove_file(&script_file);

    let result = result?;
    let _ = emit_git_change_event(&app);
    Ok(result)
}

pub async fn cmd_rebase_continue_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args = vec!["rebase".into(), "--continue".into()];
    let envs = vec![
        ("GIT_EDITOR".to_string(), "true".to_string()),
        ("GIT_SEQUENCE_EDITOR".to_string(), "true".to_string()),
    ];
    
    let result = git_run_rebase_with_env(&state, &path, &args, envs, TIMEOUT_LOCAL).await?;
    let _ = emit_git_change_event(&app);
    Ok(result)
}

pub async fn cmd_rebase_abort_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args = vec!["rebase".into(), "--abort".into()];
    let result = git_run_rebase(&state, &path, &args, TIMEOUT_LOCAL).await?;
    let _ = emit_git_change_event(&app);
    Ok(result)
}

pub async fn cmd_rebase_skip_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args = vec!["rebase".into(), "--skip".into()];
    let result = git_run_rebase(&state, &path, &args, TIMEOUT_LOCAL).await?;
    let _ = emit_git_change_event(&app);
    Ok(result)
}
