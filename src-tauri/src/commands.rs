use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::git::service::{TIMEOUT_LOCAL, TIMEOUT_NETWORK, TIMEOUT_QUICK};
use crate::git::{ConflictFile, DiagnosticInfo, GitError, GitResponse, GitResult};
use crate::settings::{save_settings, AppSettings, AppState, RepoEntry};
use tauri::Emitter;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the repository path: prefer an explicit override, fall back to the
/// active repository stored in settings.
fn resolve_repo_path(state: &State<AppState>, explicit_path: Option<String>) -> Result<String, String> {
    if let Some(path) = explicit_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    get_active_repo_path(state)
}

fn get_active_repo_path(state: &State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let active_id = settings
        .active_repo_id
        .as_ref()
        .ok_or("No active repository selected")?;
    let repo = settings
        .repos
        .iter()
        .find(|r| &r.id == active_id)
        .ok_or("Active repository not found in settings")?;
    Ok(repo.path.clone())
}

/// Shorthand: resolve path → PathBuf, run git, return GitResponse.
async fn git_run(
    state: &State<'_, AppState>,
    repo_path: Option<String>,
    args: &[&str],
    timeout: u64,
) -> Result<GitResponse, String> {
    let path = resolve_repo_path(state, repo_path)?;
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    state
        .git
        .run(Path::new(&path), &args, timeout)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Settings Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn cmd_get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_add_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    name: String,
    path: String,
) -> Result<AppSettings, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    if !path_buf.join(".git").exists() {
        return Err("Path is not a valid git repository (missing .git)".to_string());
    }

    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    settings.repos.push(RepoEntry {
        id,
        name,
        path,
    });

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_remove_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    settings.repos.retain(|r| r.id != id);

    if let Some(active_id) = &settings.active_repo_id {
        if active_id == &id {
            settings.active_repo_id = None;
        }
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_set_active_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if !settings.repos.iter().any(|r| r.id == id) {
        return Err("Repository ID not found".to_string());
    }

    settings.active_repo_id = Some(id);
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_get_active_repo(state: State<AppState>) -> Result<Option<RepoEntry>, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    if let Some(id) = &settings.active_repo_id {
        Ok(settings.repos.iter().find(|r| r.id == *id).cloned())
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Generic async git command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn run_git(
    state: State<'_, AppState>,
    subcommand: Vec<String>,
    repo_path: Option<String>,
) -> GitResult<GitResponse> {
    let path = resolve_repo_path(&state, repo_path)
        .map_err(|e| GitError::CommandError(e))?;
    state
        .git
        .run(Path::new(&path), &subcommand, TIMEOUT_LOCAL)
        .await
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticInfo, String> {
    Ok(state.git.diagnostics().await)
}

// ---------------------------------------------------------------------------
// Git Commands (all async)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_git_status(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["status"], TIMEOUT_LOCAL).await?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_pull(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["pull"], TIMEOUT_NETWORK).await?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_push(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["push"], TIMEOUT_NETWORK).await?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_fetch(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["fetch"], TIMEOUT_NETWORK).await?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["commit".into(), "-m".into(), message];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_add_all(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["add", "."], TIMEOUT_LOCAL).await?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_checkout(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["checkout".into(), branch];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    // checkout output often goes to stderr even on success
    Ok(format!("{}{}", resp.stdout, resp.stderr))
}

#[tauri::command]
pub async fn cmd_git_branch_list(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    let resp = git_run(
        &state,
        repo_path,
        &["branch", "--format=%(refname:short)"],
        TIMEOUT_LOCAL,
    )
    .await?;
    Ok(resp
        .stdout
        .lines()
        .map(|s| s.trim().to_string())
        .collect())
}

#[tauri::command]
pub async fn cmd_git_log(
    state: State<'_, AppState>,
    limit: usize,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let limit_str = format!("-n{}", limit);
    let args: Vec<String> = vec![
        "log".into(),
        limit_str,
        "--oneline".into(),
        "--graph".into(),
        "--decorate".into(),
    ];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_get_pending_commits_count(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<u32, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    
    // git rev-list --count @{u}..HEAD
    let args = vec![
        "rev-list".to_string(),
        "--count".to_string(),
        "@{u}..HEAD".to_string(),
    ];
    
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_QUICK)
        .await;

    match resp {
        Ok(output) => {
            let count = output.stdout.trim().parse::<u32>().unwrap_or(0);
            Ok(count)
        }
        Err(_) => {
            // Likely no upstream configured or other error. 
            // In either case, we can't push to upstream, so pending count is effectively 0 or irrelevant for the push button state (disabled).
            Ok(0)
        }
    }
}

#[tauri::command]
pub async fn cmd_get_commit_graph(
    state: State<'_, AppState>,
    limit: usize,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let limit_str = format!("--max-count={}", limit);
    let args: Vec<String> = vec![
        "log".into(),
        limit_str,
        "--all".into(),
        "--pretty=format:%H|%P|%d|%an|%cI|%s".into(),
        "--date=local".into(),
    ];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

// ---------------------------------------------------------------------------
// Conflict Resolution Commands (all async)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_conflicts(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    let resp = git_run(
        &state,
        repo_path,
        &["status", "--porcelain"],
        TIMEOUT_LOCAL,
    )
    .await?;

    let mut conflicts = Vec::new();
    for line in resp.stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let status = &line[0..2];
        match status {
            "UU" | "AA" | "DU" | "UD" => {
                let path = line[3..].trim().to_string();
                conflicts.push(path);
            }
            _ => {}
        }
    }
    Ok(conflicts)
}

#[tauri::command]
pub async fn cmd_get_conflict_file(
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<ConflictFile, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let repo = PathBuf::from(&r_path);

    // Run all three stages concurrently
    let (base, ours, theirs) = tokio::try_join!(
        git_show_stage(&state.git, &repo, "1", &path),
        git_show_stage(&state.git, &repo, "2", &path),
        git_show_stage(&state.git, &repo, "3", &path),
    )?;

    Ok(ConflictFile {
        base,
        ours,
        theirs,
    })
}

/// Helper to fetch a single conflict stage via `git show :<stage>:<path>`.
async fn git_show_stage(
    executor: &crate::git::GitExecutor,
    repo: &Path,
    stage: &str,
    file: &str,
) -> Result<String, String> {
    let arg = format!(":{}:{}", stage, file);
    let args = vec!["show".to_string(), arg];
    let resp = executor
        .run(repo, &args, TIMEOUT_QUICK)
        .await
        .map_err(|e| format!("git show :{}:{} failed: {}", stage, file, e))?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_resolve_ours(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["checkout".into(), "--ours".into(), path];
    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_resolve_theirs(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["checkout".into(), "--theirs".into(), path];
    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_mark_resolved(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["add".into(), path];
    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_check_conflict_state(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<bool, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let p = Path::new(&path);
    let git_dir = p.join(".git");

    // 1. Check for merge/rebase/cherry-pick heads
    let is_merging = git_dir.join("MERGE_HEAD").exists();
    let is_rebasing = git_dir.join("REBASE_HEAD").exists()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("rebase-apply").exists();
    let is_cherry_picking = git_dir.join("CHERRY_PICK_HEAD").exists();
    let is_reverting = git_dir.join("REVERT_HEAD").exists();

    if !is_merging && !is_rebasing && !is_cherry_picking && !is_reverting {
        return Ok(false);
    }

    // 2. If in a state, check for unmerged files
    let resp = git_run(
        &state,
        Some(path),
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

// ---------------------------------------------------------------------------
// File Operations
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn cmd_write_file(
    state: State<AppState>,
    path: String,
    content: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    use std::fs;

    let r_path = resolve_repo_path(&state, repo_path)?;
    let full_path = Path::new(&r_path).join(&path);

    if !full_path.starts_with(&r_path) {
        return Err("Invalid path: cannot write outside of repository".to_string());
    }

    fs::write(&full_path, content)
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(())
}
// ---------------------------------------------------------------------------
// Branch Management Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_git_branches(
    state: State<'_, AppState>,
    include_remote: bool,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    // Always use format=%(refname) for reliable parsing
    // User requested "ALL branches", so we default to -a if include_remote is true,
    // but the prompt implies we should ALWAYS do it or feature flag it.
    // The previous implementation took a bool. The user said "The application must display ALL branches".
    // I will respect the bool but default the frontend to pass true.
    
    let mut args = vec!["branch".to_string(), "--format=%(refname)".to_string()];
    if include_remote {
        args.push("-a".to_string());
    }

    let resp = git_run(&state, repo_path, &args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(), TIMEOUT_LOCAL).await?;
    
    let branches = resp.stdout
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|line| {
            if line.starts_with("refs/heads/") {
                Some(line.replace("refs/heads/", ""))
            } else if line.starts_with("refs/remotes/") {
                // formatted as "remotes/origin/main"
                Some(line.replace("refs/remotes/", "remotes/"))
            } else {
                // HEAD or other refs we might not want to show in the tree root
                None
            }
        })
        .collect();
        
    Ok(branches)
}

#[tauri::command]
pub async fn cmd_get_current_branch(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["branch", "--show-current"], TIMEOUT_QUICK).await?;
    Ok(resp.stdout.trim().to_string())
}

#[tauri::command]
pub async fn cmd_git_switch_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    branch_name: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let mut target = branch_name.as_str();

    // Handle remote branches (e.g., "remotes/origin/main" -> "main")
    if target.starts_with("remotes/") {
        let without_prefix = target.trim_start_matches("remotes/");
        if let Some(idx) = without_prefix.find('/') {
            target = &without_prefix[idx + 1..];
        } else {
            // Fallback: strictly shouldn't happen for valid remote refs, but robust fallback
            target = without_prefix;
        }
    }

    let resp = git_run(&state, repo_path, &["switch", target], TIMEOUT_LOCAL).await?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_checkout_new_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    start_point: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec![
        "checkout".into(),
        "-b".into(),
        name,
        start_point,
    ];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_merge(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["merge".into(), branch];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(format!("{}{}", resp.stdout, resp.stderr))
}
