use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::git::service::{TIMEOUT_LOCAL, TIMEOUT_NETWORK, TIMEOUT_QUICK};
use crate::git::{ConflictFile, DiagnosticInfo, GitError, GitResponse, GitResult, GitCommandType, GitCommandResult};
use crate::settings::{save_settings, AppSettings, AppState, RepoEntry};
use tauri::Emitter;
use serde_json::json;
use serde::{Deserialize, Serialize};
use glob::Pattern;

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

fn map_git_result(resp: GitResponse, command_type: GitCommandType) -> GitCommandResult {
    GitCommandResult {
        success: resp.exit_code == 0,
        stdout: resp.stdout,
        stderr: resp.stderr,
        exit_code: resp.exit_code,
        command_type,
    }
}


fn is_excluded(path: &str, exclusions: &[String]) -> bool {
    if exclusions.is_empty() {
        return false;
    }

    // Normalize path to use forward slashes for glob matching
    let normalized_path = path.replace('\\', "/");

    for pattern_str in exclusions {
        let pattern_str = pattern_str.trim();
        if pattern_str.is_empty() {
            continue;
        }

        if let Ok(pattern) = Pattern::new(pattern_str) {
            if pattern.matches(&normalized_path) {
                return true;
            }
        }
    }
    false
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

#[tauri::command]
pub fn cmd_set_excluded_files(
    app_handle: AppHandle,
    state: State<AppState>,
    exclusions: Vec<String>,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.excluded_files = exclusions;
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
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
) -> Result<GitCommandResult, String> {
    let resp = git_run(&state, repo_path, &["pull"], TIMEOUT_NETWORK).await?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Pull))
}

#[tauri::command]
pub async fn cmd_git_push(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let resp = git_run(&state, repo_path, &["push"], TIMEOUT_NETWORK).await?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Push))
}

#[tauri::command]
pub async fn cmd_git_fetch(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let resp = git_run(&state, repo_path, &["fetch"], TIMEOUT_NETWORK).await?;
    Ok(map_git_result(resp, GitCommandType::Fetch))
}

#[tauri::command]
pub async fn cmd_git_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // Safety: unstage any excluded files before committing so they are never
    // included, even if staged externally (CLI, IDE, etc.)
    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    if !exclusions.is_empty() {
        let diff_args: Vec<String> =
            vec!["diff".into(), "--cached".into(), "--name-only".into()];
        let diff_resp = state
            .git
            .run(Path::new(&path), &diff_args, TIMEOUT_QUICK)
            .await
            .map_err(|e| e.to_string())?;

        for file in diff_resp.stdout.lines() {
            let file = file.trim();
            if !file.is_empty() && is_excluded(file, &exclusions) {
                let unstage_args: Vec<String> =
                    vec!["restore".into(), "--staged".into(), file.to_string()];
                let _ = state
                    .git
                    .run(Path::new(&path), &unstage_args, TIMEOUT_QUICK)
                    .await;
            }
        }
    }

    let args: Vec<String> = vec!["commit".into(), "-m".into(), message];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Commit))
}

#[tauri::command]
pub async fn cmd_git_add_all(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    let mut args = vec!["add".to_string(), ".".to_string()];
    for exc in exclusions {
        if !exc.trim().is_empty() {
            args.push(format!(":!{}", exc));
        }
    }

    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_git_unstage_all(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    // git restore --staged .
    let args: Vec<String> = vec!["restore".into(), "--staged".into(), ".".into()];
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
pub async fn cmd_git_checkout(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
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
    Ok(map_git_result(resp, GitCommandType::Checkout))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[tauri::command]
pub async fn cmd_get_status_files(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<FileStatus>, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args = vec!["status".to_string(), "--porcelain".to_string()];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    let mut results = Vec::new();

    for line in resp.stdout.lines() {
        if line.len() < 4 {
            // "?? file" is 3+chars but usually safe.
            if line.starts_with("?? ") {
                let path = line[3..].trim().to_string();
                if is_excluded(&path, &exclusions) {
                    continue;
                }
                results.push(FileStatus {
                    path,
                    status: "??".to_string(),
                    staged: false,
                });
            }
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        if chars.len() < 2 { continue; }
        
        let x = chars[0];
        let y = chars[1];
        let file_path = line[3..].trim().to_string();

        if is_excluded(&file_path, &exclusions) {
            continue;
        }

        // Staged status (X)
        if x != ' ' && x != '?' {
            results.push(FileStatus {
                path: file_path.clone(),
                status: x.to_string(),
                staged: true,
            });
        }

        // Unstaged status (Y)
        if y != ' ' {
            results.push(FileStatus {
                path: file_path.clone(),
                status: y.to_string(),
                staged: false,
            });
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn cmd_get_diff_file(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    
    let mut args = vec!["diff".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(file_path);

    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_get_file_base_content(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let show_arg = format!("HEAD:{}", file_path);
    let args = vec!["show".to_string(), show_arg];

    // New/untracked files won't exist at HEAD -- return empty content
    match state
        .git
        .run(Path::new(&path), &args, TIMEOUT_QUICK)
        .await
    {
        Ok(resp) => Ok(resp.stdout),
        Err(_) => Ok(String::new()),
    }
}

#[tauri::command]
pub async fn cmd_get_file_modified_content(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    if staged {
        // Staged content lives in the index (stage 0)
        let show_arg = format!(":{}", file_path);
        let args = vec!["show".to_string(), show_arg];
        match state
            .git
            .run(Path::new(&path), &args, TIMEOUT_QUICK)
            .await
        {
            Ok(resp) => Ok(resp.stdout),
            Err(_) => Ok(String::new()),
        }
    } else {
        // Unstaged content: read directly from the working directory
        let full_path = Path::new(&path).join(&file_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => Ok(content),
            Err(_) => Ok(String::new()),
        }
    }
}

#[tauri::command]
pub async fn cmd_git_add(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;

    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    if is_excluded(&path, &exclusions) {
         return Err(format!("File {} is excluded from git operations", path));
    }

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
pub async fn cmd_git_unstage(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    // git restore --staged <path>
    let args: Vec<String> = vec!["restore".into(), "--staged".into(), path];
    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
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
) -> Result<GitCommandResult, String> {
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
    Ok(map_git_result(resp, GitCommandType::Checkout))
}

#[tauri::command]
pub async fn cmd_git_checkout_new_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    start_point: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
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
    Ok(map_git_result(resp, GitCommandType::Checkout))
}

#[tauri::command]
pub async fn cmd_git_merge(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["merge".into(), branch];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Merge))
}
