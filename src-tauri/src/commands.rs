use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::git::service::{TIMEOUT_LOCAL, TIMEOUT_NETWORK, TIMEOUT_QUICK};
use crate::git::{
    ConflictFile, DiagnosticInfo, FullRebaseStatus, GitCommandResult, GitCommandType, GitError,
    GitResponse, GitResult, RebaseStepInfo, RebaseTodoItem,
};
use crate::models::{CommitDiff, DiffFile, DiffHunk, DiffLine, DiffLineType, FileCommit};
use crate::settings::{save_settings, AppSettings, AppState, RepoEntry};
use glob::Pattern;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;

mod ai_commands;
mod conflict_commands;
mod diff_commands;
mod rebase_commands;
mod settings_commands;
mod terminal_commands;

pub use diff_commands::StageLineSelection;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the repository path: prefer an explicit override, fall back to the
/// active repository stored in settings.
fn resolve_repo_path(
    state: &State<AppState>,
    explicit_path: Option<String>,
) -> Result<String, String> {
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

fn emit_git_change_event(app: &AppHandle) -> Result<(), String> {
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())
}

async fn git_run_vec(
    state: &State<'_, AppState>,
    repo_path: Option<String>,
    args: Vec<String>,
    timeout: u64,
) -> Result<GitResponse, String> {
    let path = resolve_repo_path(state, repo_path)?;
    git_run_vec_at_path(state, &path, args, timeout).await
}

async fn git_run_vec_at_path(
    state: &State<'_, AppState>,
    repo_path: &str,
    args: Vec<String>,
    timeout: u64,
) -> Result<GitResponse, String> {
    state
        .git
        .run(Path::new(repo_path), &args, timeout)
        .await
        .map_err(|e| e.to_string())
}

async fn git_run_result_with_event(
    app: &AppHandle,
    state: &State<'_, AppState>,
    repo_path: Option<String>,
    args: Vec<String>,
    timeout: u64,
    command_type: GitCommandType,
) -> Result<GitCommandResult, String> {
    let resp = git_run_vec(state, repo_path, args, timeout).await?;
    emit_git_change_event(app)?;
    Ok(map_git_result(resp, command_type))
}

async fn git_run_result_at_path_with_event(
    app: &AppHandle,
    state: &State<'_, AppState>,
    repo_path: &str,
    args: Vec<String>,
    timeout: u64,
    command_type: GitCommandType,
) -> Result<GitCommandResult, String> {
    let resp = git_run_vec_at_path(state, repo_path, args, timeout).await?;
    emit_git_change_event(app)?;
    Ok(map_git_result(resp, command_type))
}

async fn git_run_void_with_event(
    app: &AppHandle,
    state: &State<'_, AppState>,
    repo_path: Option<String>,
    args: Vec<String>,
    timeout: u64,
) -> Result<(), String> {
    git_run_vec(state, repo_path, args, timeout).await?;
    emit_git_change_event(app)?;
    Ok(())
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

#[cfg(target_os = "windows")]
fn hide_console_window(cmd: &mut std::process::Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_cmd: &mut std::process::Command) {}

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

fn split_rename_path(path: &str) -> Option<(String, String)> {
    let mut parts = path.splitn(2, " -> ");
    let old_path = parts.next()?.trim();
    let new_path = parts.next()?.trim();
    if old_path.is_empty() || new_path.is_empty() {
        return None;
    }
    Some((old_path.to_string(), new_path.to_string()))
}

fn is_untracked_status(status: &str) -> bool {
    matches!(status.trim(), "??" | "?")
}

fn resolve_file_target_path(raw_path: &str) -> String {
    if let Some((_old_path, new_path)) = split_rename_path(raw_path) {
        new_path
    } else {
        raw_path.to_string()
    }
}

async fn resolve_stash_ref_by_commit_hash(
    state: &State<'_, AppState>,
    repo_path: &str,
    commit_hash: &str,
) -> Result<String, String> {
    let target_hash = commit_hash.trim().to_lowercase();
    if target_hash.is_empty() {
        return Err("No stash commit hash provided".to_string());
    }

    let args = vec![
        "stash".to_string(),
        "list".to_string(),
        "--format=%gd|%H".to_string(),
    ];

    let resp = state
        .git
        .run(Path::new(repo_path), &args, TIMEOUT_QUICK)
        .await
        .map_err(|e| e.to_string())?;

    for line in resp.stdout.lines() {
        let entry = line.trim();
        if entry.is_empty() {
            continue;
        }

        let mut parts = entry.splitn(2, '|');
        let stash_ref = parts.next().unwrap_or("").trim();
        let stash_hash = parts.next().unwrap_or("").trim().to_lowercase();
        if stash_ref.is_empty() || stash_hash.is_empty() {
            continue;
        }

        if stash_hash == target_hash || stash_hash.starts_with(&target_hash) {
            return Ok(stash_ref.to_string());
        }
    }

    Err(format!(
        "Stash entry not found for commit {}",
        commit_hash.trim()
    ))
}

#[cfg(target_os = "windows")]
fn quote_windows_arg(value: &str) -> String {
    let escaped = value.replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

#[cfg(not(target_os = "windows"))]
fn quote_shell_arg(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{}'", escaped)
}

async fn get_configured_editor_command(
    state: &State<'_, AppState>,
    repo_path: &str,
) -> Option<String> {
    if let Ok(resp) = state
        .git
        .run(
            Path::new(repo_path),
            &[
                "config".to_string(),
                "--get".to_string(),
                "core.editor".to_string(),
            ],
            TIMEOUT_QUICK,
        )
        .await
    {
        let editor = resp.stdout.trim();
        if !editor.is_empty() {
            return Some(editor.to_string());
        }
    }

    if let Ok(editor) = std::env::var("VISUAL") {
        if !editor.trim().is_empty() {
            return Some(editor);
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.trim().is_empty() {
            return Some(editor);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Settings Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn cmd_get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    settings_commands::cmd_get_settings_impl(state)
}

#[tauri::command]
pub fn cmd_add_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    name: String,
    path: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_add_repo_impl(app_handle, state, name, path)
}

#[tauri::command]
pub fn cmd_remove_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_remove_repo_impl(app_handle, state, id)
}

#[tauri::command]
pub fn cmd_set_active_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_active_repo_impl(app_handle, state, id)
}

#[tauri::command]
pub fn cmd_open_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_open_repo_impl(app_handle, state, id)
}

#[tauri::command]
pub fn cmd_close_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_close_repo_impl(app_handle, state, id)
}

#[tauri::command]
pub fn cmd_get_active_repo(state: State<AppState>) -> Result<Option<RepoEntry>, String> {
    settings_commands::cmd_get_active_repo_impl(state)
}

#[tauri::command]
pub fn cmd_set_excluded_files(
    app_handle: AppHandle,
    state: State<AppState>,
    exclusions: Vec<String>,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_excluded_files_impl(app_handle, state, exclusions)
}

#[tauri::command]
pub fn cmd_set_repo_filter(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_id: String,
    filter: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_repo_filter_impl(app_handle, state, repo_id, filter)
}

#[tauri::command]
pub fn cmd_set_gemini_api_token(
    app_handle: AppHandle,
    state: State<AppState>,
    token: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_gemini_api_token_impl(app_handle, state, token)
}

#[tauri::command]
pub fn cmd_set_gemini_model(
    app_handle: AppHandle,
    state: State<AppState>,
    model: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_gemini_model_impl(app_handle, state, model)
}

#[tauri::command]
pub fn cmd_set_global_commit_prompt(
    app_handle: AppHandle,
    state: State<AppState>,
    prompt: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_global_commit_prompt_impl(app_handle, state, prompt)
}

#[tauri::command]
pub fn cmd_set_repo_commit_prompt(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_path: String,
    prompt: String,
) -> Result<AppSettings, String> {
    settings_commands::cmd_set_repo_commit_prompt_impl(app_handle, state, repo_path, prompt)
}

#[tauri::command]
pub async fn cmd_get_gemini_models(
    state: State<'_, AppState>,
    token: Option<String>,
) -> Result<Vec<String>, String> {
    ai_commands::cmd_get_gemini_models_impl(state, token).await
}

#[tauri::command]
pub fn cmd_get_default_ai_prompt() -> String {
    ai_commands::cmd_get_default_ai_prompt_impl()
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
    let path = resolve_repo_path(&state, repo_path).map_err(|e| GitError::CommandError(e))?;
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
    emit_git_change_event(&app)?;
    Ok(map_git_result(resp, GitCommandType::Pull))
}

#[tauri::command]
pub async fn cmd_git_push(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // Check if the current branch has an upstream configured
    let upstream_check = state
        .git
        .run(
            Path::new(&path),
            &[
                "rev-parse".to_string(),
                "--abbrev-ref".to_string(),
                "--symbolic-full-name".to_string(),
                "@{u}".to_string(),
            ],
            TIMEOUT_LOCAL,
        )
        .await;

    let has_upstream = upstream_check
        .as_ref()
        .map(|r| r.exit_code == 0)
        .unwrap_or(false);

    let resp = if has_upstream {
        // Normal push — upstream already set
        state
            .git
            .run(Path::new(&path), &["push".to_string()], TIMEOUT_NETWORK)
            .await
            .map_err(|e| e.to_string())?
    } else {
        // Get current branch name for -u push
        let branch_resp = state
            .git
            .run(
                Path::new(&path),
                &[
                    "rev-parse".to_string(),
                    "--abbrev-ref".to_string(),
                    "HEAD".to_string(),
                ],
                TIMEOUT_LOCAL,
            )
            .await
            .map_err(|e| e.to_string())?;
        let branch = branch_resp.stdout.trim().to_string();

        state
            .git
            .run(
                Path::new(&path),
                &[
                    "push".to_string(),
                    "-u".to_string(),
                    "origin".to_string(),
                    branch,
                ],
                TIMEOUT_NETWORK,
            )
            .await
            .map_err(|e| e.to_string())?
    };

    emit_git_change_event(&app)?;
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
        let diff_args: Vec<String> = vec!["diff".into(), "--cached".into(), "--name-only".into()];
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
    emit_git_change_event(&app)?;
    Ok(map_git_result(resp, GitCommandType::Commit))
}

#[tauri::command]
pub async fn cmd_generate_commit_message(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    ai_commands::cmd_generate_commit_message_impl(state, repo_path).await
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
    // git restore --staged .
    let args: Vec<String> = vec!["restore".into(), "--staged".into(), ".".into()];
    git_run_void_with_event(&app, &state, repo_path, args, TIMEOUT_LOCAL).await
}

#[tauri::command]
pub async fn cmd_git_checkout(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let args: Vec<String> = vec!["checkout".into(), branch];
    // checkout output often goes to stderr even on success
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Checkout,
    )
    .await
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
    Ok(resp.stdout.lines().map(|s| s.trim().to_string()).collect())
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

    let resp = state.git.run(Path::new(&path), &args, TIMEOUT_QUICK).await;

    match resp {
        Ok(output) if output.exit_code == 0 => {
            let count = output.stdout.trim().parse::<u32>().unwrap_or(0);
            Ok(count)
        }
        _ => {
            // No upstream configured — the branch has never been pushed.
            // Count commits ahead of the default remote branch (origin/HEAD or origin/main)
            // so the Push button stays enabled.
            let fallback_args = vec![
                "rev-list".to_string(),
                "--count".to_string(),
                "HEAD".to_string(),
                "--not".to_string(),
                "--remotes=origin".to_string(),
            ];
            let fallback = state
                .git
                .run(Path::new(&path), &fallback_args, TIMEOUT_QUICK)
                .await;
            match fallback {
                Ok(output) if output.exit_code == 0 => {
                    let count = output.stdout.trim().parse::<u32>().unwrap_or(0);
                    // If no remote branches exist at all, show at least 1 to indicate the branch needs pushing
                    if count == 0 {
                        Ok(1)
                    } else {
                        Ok(count)
                    }
                }
                _ => Ok(1), // Fallback: indicate at least 1 commit to push
            }
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
    let args = build_commit_graph_args(limit);
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

fn build_commit_graph_args(limit: usize) -> Vec<String> {
    vec![
        "log".to_string(),
        format!("--max-count={}", limit),
        "--all".to_string(),
        "--pretty=format:%H|%P|%d|%an|%cI|%s".to_string(),
        "--date=local".to_string(),
    ]
}

// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitChangedFile {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    pub commit_hash: String,
    pub author: String,
    pub date: String,
    pub line_number: u32,
    pub content: String,
}

fn strip_surrounding_quotes(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_untracked_status_line(line: &str) -> Option<FileStatus> {
    if !line.starts_with("?? ") {
        return None;
    }
    let path = strip_surrounding_quotes(&line[3..]);
    if path.is_empty() {
        return None;
    }
    Some(FileStatus {
        path,
        status: "??".to_string(),
        staged: false,
    })
}

fn parse_status_line(line: &str) -> Vec<FileStatus> {
    if line.len() < 4 {
        return parse_untracked_status_line(line).into_iter().collect();
    }

    let mut entries = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    if chars.len() < 2 {
        return entries;
    }

    let x = chars[0];
    let y = chars[1];
    let file_path = strip_surrounding_quotes(&line[3..]);
    if file_path.is_empty() {
        return entries;
    }

    if x == '?' && y == '?' {
        entries.push(FileStatus {
            path: file_path,
            status: "??".to_string(),
            staged: false,
        });
        return entries;
    }

    if x != ' ' && x != '?' {
        entries.push(FileStatus {
            path: file_path.clone(),
            status: x.to_string(),
            staged: true,
        });
    }
    if y != ' ' {
        entries.push(FileStatus {
            path: file_path,
            status: y.to_string(),
            staged: false,
        });
    }
    entries
}

fn parse_status_entries(output: &str) -> Vec<FileStatus> {
    output.lines().flat_map(parse_status_line).collect()
}

fn filter_excluded_status_entries(entries: Vec<FileStatus>, exclusions: &[String]) -> Vec<FileStatus> {
    entries
        .into_iter()
        .filter(|entry| !is_excluded(&entry.path, exclusions))
        .collect()
}

fn load_exclusion_patterns(state: &State<'_, AppState>) -> Result<Vec<String>, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.excluded_files.clone())
}

async fn fetch_raw_status_output(
    state: &State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(state, repo_path)?;
    let args = vec!["status".to_string(), "--porcelain".to_string()];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_get_status_files(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<FileStatus>, String> {
    let raw_output = fetch_raw_status_output(&state, repo_path).await?;
    let exclusions = load_exclusion_patterns(&state)?;
    let entries = parse_status_entries(&raw_output);
    Ok(filter_excluded_status_entries(entries, &exclusions))
}

#[tauri::command]
pub async fn cmd_get_diff_file(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    diff_commands::cmd_get_diff_file_impl(state, file_path, staged, encoding, repo_path).await
}

#[tauri::command]
pub async fn cmd_get_file_base_content(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    diff_commands::cmd_get_file_base_content_impl(state, file_path, staged, encoding, repo_path)
        .await
}

#[tauri::command]
pub async fn cmd_get_file_modified_content(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    diff_commands::cmd_get_file_modified_content_impl(state, file_path, staged, encoding, repo_path)
        .await
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
    git_run_vec_at_path(&state, &r_path, args, TIMEOUT_LOCAL).await?;
    emit_git_change_event(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_git_stage_line(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    line: StageLineSelection,
    repo_path: Option<String>,
) -> Result<(), String> {
    diff_commands::cmd_git_stage_line_impl(app, state, path, line, repo_path).await
}

#[tauri::command]
pub async fn cmd_git_unstage_line(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    line: StageLineSelection,
    repo_path: Option<String>,
) -> Result<(), String> {
    diff_commands::cmd_git_unstage_line_impl(app, state, path, line, repo_path).await
}

#[tauri::command]
pub async fn cmd_git_unstage(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    // git restore --staged <path>
    let args: Vec<String> = vec!["restore".into(), "--staged".into(), path];
    git_run_void_with_event(&app, &state, repo_path, args, TIMEOUT_LOCAL).await
}

#[tauri::command]
pub async fn cmd_git_discard_changes(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<FileStatus>,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    let mut tracked_paths = HashSet::<String>::new();
    let mut untracked_paths = HashSet::<String>::new();

    for file in files {
        let path = file.path.trim().to_string();
        if path.is_empty() || is_excluded(&path, &exclusions) {
            continue;
        }

        if is_untracked_status(&file.status) {
            untracked_paths.insert(path);
            continue;
        }

        if let Some((old_path, new_path)) = split_rename_path(&path) {
            tracked_paths.insert(old_path);
            tracked_paths.insert(new_path);
        } else {
            tracked_paths.insert(path);
        }
    }

    if !tracked_paths.is_empty() {
        let mut args: Vec<String> = vec![
            "restore".into(),
            "--source=HEAD".into(),
            "--staged".into(),
            "--worktree".into(),
            "--".into(),
        ];
        args.extend(tracked_paths.into_iter());
        state
            .git
            .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
            .await
            .map_err(|e| e.to_string())?;
    }

    if !untracked_paths.is_empty() {
        let mut args: Vec<String> = vec!["clean".into(), "-fd".into(), "--".into()];
        args.extend(untracked_paths.into_iter());
        state
            .git
            .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
            .await
            .map_err(|e| e.to_string())?;
    }

    emit_git_change_event(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_git_stash_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file: FileStatus,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let exclusions = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.excluded_files.clone()
    };

    let raw_path = file.path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let stash_path = resolve_file_target_path(raw_path);

    if is_excluded(&stash_path, &exclusions) {
        return Err(format!(
            "File {} is excluded from git operations",
            stash_path
        ));
    }

    let include_untracked = is_untracked_status(&file.status);
    let stash_message = format!("stash {}", stash_path);

    let mut args: Vec<String> = vec!["stash".into(), "push".into(), "-m".into(), stash_message];
    if include_untracked {
        args.push("-u".into());
    }
    args.push("--".into());
    args.push(stash_path);

    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    emit_git_change_event(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_git_stash_all(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec![
        "stash".into(),
        "push".into(),
        "-u".into(),
        "-m".into(),
        "stash all".into(),
    ];

    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    emit_git_change_event(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_git_apply_stash(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let stash_ref = resolve_stash_ref_by_commit_hash(&state, &r_path, &commit_hash).await?;

    let args = vec!["stash".to_string(), "apply".to_string(), stash_ref];
    git_run_result_at_path_with_event(
        &app,
        &state,
        &r_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_pop_stash(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let stash_ref = resolve_stash_ref_by_commit_hash(&state, &r_path, &commit_hash).await?;

    let args = vec!["stash".to_string(), "pop".to_string(), stash_ref];
    git_run_result_at_path_with_event(
        &app,
        &state,
        &r_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_delete_stash(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let stash_ref = resolve_stash_ref_by_commit_hash(&state, &r_path, &commit_hash).await?;

    let args = vec!["stash".to_string(), "drop".to_string(), stash_ref];
    git_run_result_at_path_with_event(
        &app,
        &state,
        &r_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_edit_stash_message(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    message: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let stash_ref = resolve_stash_ref_by_commit_hash(&state, &r_path, &commit_hash).await?;

    let new_message = message.trim();
    if new_message.is_empty() {
        return Err("Stash message cannot be empty".to_string());
    }

    // Resolve object id before dropping so we can restore with a new message.
    let rev_parse_args = vec!["rev-parse".to_string(), stash_ref.clone()];
    let rev_parse_resp = state
        .git
        .run(Path::new(&r_path), &rev_parse_args, TIMEOUT_QUICK)
        .await
        .map_err(|e| e.to_string())?;

    let stash_object_id = rev_parse_resp.stdout.trim();
    if stash_object_id.is_empty() {
        return Err("Unable to resolve stash object id".to_string());
    }

    let drop_args = vec!["stash".to_string(), "drop".to_string(), stash_ref];
    let drop_resp = state
        .git
        .run(Path::new(&r_path), &drop_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    if drop_resp.exit_code != 0 {
        return Ok(map_git_result(drop_resp, GitCommandType::Other));
    }

    let store_args = vec![
        "stash".to_string(),
        "store".to_string(),
        "-m".to_string(),
        new_message.to_string(),
        stash_object_id.to_string(),
    ];
    let store_resp = state
        .git
        .run(Path::new(&r_path), &store_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    emit_git_change_event(&app)?;

    Ok(map_git_result(store_resp, GitCommandType::Other))
}

#[tauri::command]
pub async fn cmd_create_patch_from_stash(
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let stash_ref = resolve_stash_ref_by_commit_hash(&state, &r_path, &commit_hash).await?;

    let args = vec![
        "stash".to_string(),
        "show".to_string(),
        "-p".to_string(),
        stash_ref,
    ];
    let resp = state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_open_repo_file(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);

    let candidate = PathBuf::from(&target_path);
    let full_path = if candidate.is_absolute() {
        candidate
    } else {
        Path::new(&r_path).join(candidate)
    };

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let path_str = full_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg("start").arg("").arg(&path_str);
        hide_console_window(&mut cmd);
        cmd.spawn().map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn cmd_git_ignore_file(
    state: State<'_, AppState>,
    pattern: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let trimmed_pattern = pattern.trim();
    if trimmed_pattern.is_empty() {
        return Err("Ignore pattern cannot be empty".to_string());
    }

    let gitignore_path = Path::new(&r_path).join(".gitignore");
    let mut content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    if content.lines().any(|line| line.trim() == trimmed_pattern) {
        return Ok(());
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(trimmed_pattern);
    content.push('\n');

    std::fs::write(&gitignore_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_show_in_folder(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);
    let candidate = PathBuf::from(&target_path);
    let full_path = if candidate.is_absolute() {
        candidate
    } else {
        Path::new(&r_path).join(candidate)
    };

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let path_str = full_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg("/select,").arg(&path_str);
        hide_console_window(&mut cmd);
        cmd.spawn().map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(parent) = full_path.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn cmd_open_in_editor(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);
    let candidate = PathBuf::from(&target_path);
    let full_path = if candidate.is_absolute() {
        candidate
    } else {
        Path::new(&r_path).join(candidate)
    };

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let editor = get_configured_editor_command(&state, &r_path)
        .await
        .unwrap_or_else(|| "code".to_string());
    let path_str = full_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        let command_line = format!("{} {}", editor, quote_windows_arg(&path_str));
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg(command_line);
        hide_console_window(&mut cmd);
        cmd.spawn()
            .map_err(|e| format!("Failed to open editor '{}': {}", editor, e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let command_line = format!("{} {}", editor, quote_shell_arg(&path_str));
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command_line)
            .spawn()
            .map_err(|e| format!("Failed to open editor '{}': {}", editor, e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn cmd_open_in_diff_tool(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let configured_diff_tool = state
        .git
        .run(
            Path::new(&r_path),
            &[
                "config".to_string(),
                "--get".to_string(),
                "diff.tool".to_string(),
            ],
            TIMEOUT_QUICK,
        )
        .await
        .ok()
        .map(|resp| resp.stdout.trim().to_string())
        .unwrap_or_default();

    if configured_diff_tool.is_empty() {
        return Err(
            "No external diff tool configured. Run `git config diff.tool <tool>` first."
                .to_string(),
        );
    }

    let target_path = resolve_file_target_path(raw_path);
    let mut args = vec!["difftool".to_string(), "--no-prompt".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(target_path);

    state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn cmd_create_patch(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    repo_path: Option<String>,
) -> Result<String, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);
    let mut args = vec!["diff".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(target_path);

    let resp = state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_create_patch_from_commit(
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let target_commit = commit_hash.trim();
    if target_commit.is_empty() {
        return Err("No commit hash provided".to_string());
    }

    let args = vec![
        "format-patch".to_string(),
        "-1".to_string(),
        target_commit.to_string(),
        "--stdout".to_string(),
    ];

    let resp = state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.stdout)
}

#[tauri::command]
pub async fn cmd_delete_file(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);
    let repo_root = PathBuf::from(&r_path);
    let candidate = PathBuf::from(&target_path);
    let full_path = if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    };

    if !full_path.exists() {
        return Err(format!("File not found: {}", full_path.display()));
    }

    let canonical_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let canonical_target = full_path.canonicalize().map_err(|e| e.to_string())?;

    if !canonical_target.starts_with(&canonical_repo) {
        return Err("Invalid path: cannot delete outside of repository".to_string());
    }

    if canonical_target.is_dir() {
        std::fs::remove_dir_all(&canonical_target)
            .map_err(|e| format!("Failed to delete {}: {}", target_path, e))?;
    } else {
        std::fs::remove_file(&canonical_target)
            .map_err(|e| format!("Failed to delete {}: {}", target_path, e))?;
    }

    Ok(())
}

fn parse_blame_header(line: &str) -> Option<(String, u32)> {
    let mut parts = line.split_whitespace();
    let commit_hash = parts.next()?;
    let _orig_line = parts.next()?;
    let final_line = parts.next()?.parse::<u32>().ok()?;
    if commit_hash.is_empty() {
        return None;
    }
    Some((commit_hash.to_string(), final_line))
}

fn parse_blame_output(stdout: &str) -> Vec<BlameLine> {
    let mut lines = Vec::new();
    let mut current_commit = String::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_line_number: u32 = 0;

    for line in stdout.lines() {
        if let Some(content) = line.strip_prefix('\t') {
            lines.push(BlameLine {
                commit_hash: current_commit.clone(),
                author: current_author.clone(),
                date: current_date.clone(),
                line_number: current_line_number,
                content: content.to_string(),
            });
            continue;
        }

        if let Some((commit_hash, line_number)) = parse_blame_header(line) {
            current_commit = commit_hash;
            current_line_number = line_number;
            current_author.clear();
            current_date.clear();
            continue;
        }

        if let Some(author) = line.strip_prefix("author ") {
            current_author = author.to_string();
            continue;
        }

        if let Some(date) = line.strip_prefix("author-time ") {
            current_date = date.to_string();
        }
    }

    lines
}

#[tauri::command]
pub async fn cmd_git_blame(
    state: State<'_, AppState>,
    file_path: String,
    repo_path: Option<String>,
) -> Result<Vec<BlameLine>, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let raw_path = file_path.trim();
    if raw_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    let target_path = resolve_file_target_path(raw_path);
    let args = vec![
        "blame".to_string(),
        "--line-porcelain".to_string(),
        "--".to_string(),
        target_path,
    ];

    let resp = state
        .git
        .run(Path::new(&r_path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_blame_output(&resp.stdout))
}

// ---------------------------------------------------------------------------
// Conflict Resolution Commands (all async)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_conflicts(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    conflict_commands::cmd_get_conflicts_impl(state, repo_path).await
}

#[tauri::command]
pub async fn cmd_get_conflict_file(
    state: State<'_, AppState>,
    path: String,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<ConflictFile, String> {
    conflict_commands::cmd_get_conflict_file_impl(state, path, encoding, repo_path).await
}

#[tauri::command]
pub async fn cmd_resolve_ours(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    conflict_commands::cmd_resolve_ours_impl(app, state, path, repo_path).await
}

#[tauri::command]
pub async fn cmd_resolve_theirs(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    conflict_commands::cmd_resolve_theirs_impl(app, state, path, repo_path).await
}

#[tauri::command]
pub async fn cmd_mark_resolved(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    conflict_commands::cmd_mark_resolved_impl(app, state, path, repo_path).await
}

#[tauri::command]
pub async fn cmd_check_conflict_state(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<bool, String> {
    conflict_commands::cmd_check_conflict_state_impl(state, repo_path).await
}

#[tauri::command]
pub async fn cmd_get_operation_state(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<conflict_commands::GitOperationState, String> {
    conflict_commands::cmd_get_operation_state_impl(state, repo_path).await
}

// ---------------------------------------------------------------------------
// File Operations
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn cmd_write_file(
    state: State<AppState>,
    path: String,
    content: String,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<(), String> {
    use std::fs;

    let r_path = resolve_repo_path(&state, repo_path)?;
    let full_path = Path::new(&r_path).join(&path);

    if !full_path.starts_with(&r_path) {
        return Err("Invalid path: cannot write outside of repository".to_string());
    }

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let bytes = crate::git::encoding::encode_string(
        &content,
        Path::new(&path),
        &settings,
        encoding,
    );

    fs::write(&full_path, bytes).map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(())
}
// ---------------------------------------------------------------------------
// Branch Management Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_branch_tip(
    state: State<'_, AppState>,
    branch_name: String,
    repo_path: Option<String>,
) -> Result<String, String> {
    let resp = git_run(&state, repo_path, &["rev-parse", &branch_name], TIMEOUT_QUICK).await?;
    if resp.exit_code == 0 {
        Ok(resp.stdout.trim().to_string())
    } else {
        Err(resp.stderr)
    }
}

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

    let resp = git_run(
        &state,
        repo_path,
        &args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
        TIMEOUT_LOCAL,
    )
    .await?;

    let branches = resp
        .stdout
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
    let resp = git_run(
        &state,
        repo_path,
        &["branch", "--show-current"],
        TIMEOUT_QUICK,
    )
    .await?;
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

    let args: Vec<String> = vec!["switch".into(), target.to_string()];
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Checkout,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_checkout_new_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    start_point: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let args: Vec<String> = vec!["checkout".into(), "-b".into(), name, start_point];
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Checkout,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_create_branch(
    state: State<'_, AppState>,
    name: String,
    base: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let args: Vec<String> = vec!["branch".into(), name, base];
    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Branch))
}

#[tauri::command]
pub async fn cmd_git_merge(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let args: Vec<String> = vec!["merge".into(), branch];
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Merge,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_rebase(
    app: AppHandle,
    state: State<'_, AppState>,
    branch: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let args: Vec<String> = vec!["rebase".into(), branch];
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Rebase,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_cherry_pick(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let args: Vec<String> = vec!["cherry-pick".into(), commit_hash];
    git_run_result_with_event(
        &app,
        &state,
        repo_path,
        args,
        TIMEOUT_LOCAL,
        GitCommandType::CherryPick,
    )
    .await
}

#[tauri::command]
pub async fn cmd_abort_operation(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let git_dir = Path::new(&path).join(".git");

    let is_merging = git_dir.join("MERGE_HEAD").exists();
    let is_rebasing = git_dir.join("REBASE_HEAD").exists()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("rebase-apply").exists();
    let is_cherry_picking = git_dir.join("CHERRY_PICK_HEAD").exists();
    let is_reverting = git_dir.join("REVERT_HEAD").exists();

    let args: Vec<String> = if is_rebasing {
        vec!["rebase".into(), "--abort".into()]
    } else if is_merging {
        vec!["merge".into(), "--abort".into()]
    } else if is_cherry_picking {
        vec!["cherry-pick".into(), "--abort".into()]
    } else if is_reverting {
        vec!["revert".into(), "--abort".into()]
    } else {
        return Err("No merge/rebase/cherry-pick/revert operation is in progress.".to_string());
    };

    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_revert(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let target_commit = commit_hash.trim();
    if target_commit.is_empty() {
        return Err("No commit hash provided".to_string());
    }

    let args: Vec<String> = vec![
        "revert".into(),
        "--no-edit".into(),
        target_commit.to_string(),
    ];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_reset(
    app: AppHandle,
    state: State<'_, AppState>,
    commit_hash: String,
    mode: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let target_commit = commit_hash.trim();
    if target_commit.is_empty() {
        return Err("No commit hash provided".to_string());
    }

    let normalized_mode = mode.trim().to_lowercase();
    if !matches!(normalized_mode.as_str(), "soft" | "mixed" | "hard") {
        return Err("Invalid reset mode. Expected soft, mixed, or hard.".to_string());
    }

    let mode_flag = format!("--{}", normalized_mode);
    let args: Vec<String> = vec!["reset".into(), mode_flag, target_commit.to_string()];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_create_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    tag_name: String,
    commit_hash: String,
    message: Option<String>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let target_tag = tag_name.trim();
    let target_commit = commit_hash.trim();

    if target_tag.is_empty() {
        return Err("No tag name provided".to_string());
    }
    if target_commit.is_empty() {
        return Err("No commit hash provided".to_string());
    }

    let trimmed_message = message
        .as_ref()
        .map(|msg| msg.trim())
        .filter(|msg| !msg.is_empty())
        .map(|msg| msg.to_string());

    let mut args: Vec<String> = vec!["tag".into()];
    match trimmed_message {
        Some(msg) => {
            args.push("-a".into());
            args.push(target_tag.to_string());
            args.push(target_commit.to_string());
            args.push("-m".into());
            args.push(msg);
        }
        None => {
            args.push(target_tag.to_string());
            args.push(target_commit.to_string());
        }
    }

    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Branch,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_delete_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    branch_name: String,
    force: bool,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let target_branch = branch_name.trim();
    if target_branch.is_empty() {
        return Err("No branch name provided".to_string());
    }

    let delete_flag = if force { "-D" } else { "-d" };
    let args: Vec<String> = vec!["branch".into(), delete_flag.into(), target_branch.to_string()];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Branch,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_delete_remote_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
    branch_name: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let target_remote = remote.trim();
    let target_branch = branch_name.trim();

    if target_remote.is_empty() || target_branch.is_empty() {
        return Err("Remote and branch name are required".to_string());
    }

    let args: Vec<String> = vec![
        "push".into(),
        target_remote.to_string(),
        format!(":{}", target_branch),
    ];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_NETWORK,
        GitCommandType::Push,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_rename_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let old_branch = old_name.trim();
    let new_branch = new_name.trim();
    if old_branch.is_empty() || new_branch.is_empty() {
        return Err("Both old and new branch names are required".to_string());
    }

    let args: Vec<String> = vec![
        "branch".into(),
        "-m".into(),
        old_branch.to_string(),
        new_branch.to_string(),
    ];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Branch,
    )
    .await
}

#[tauri::command]
pub async fn cmd_git_set_upstream(
    app: AppHandle,
    state: State<'_, AppState>,
    branch_name: String,
    upstream: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let local_branch = branch_name.trim();
    let upstream_ref = upstream.trim();
    if local_branch.is_empty() || upstream_ref.is_empty() {
        return Err("Branch and upstream are required".to_string());
    }

    let args: Vec<String> = vec![
        "branch".into(),
        "--set-upstream-to".into(),
        upstream_ref.to_string(),
        local_branch.to_string(),
    ];
    git_run_result_with_event(
        &app,
        &state,
        Some(path),
        args,
        TIMEOUT_LOCAL,
        GitCommandType::Other,
    )
    .await
}

#[tauri::command]
pub async fn cmd_get_file_history(
    state: State<'_, AppState>,
    file_path: String,
    limit: Option<u32>,
    repo_path: Option<String>,
) -> Result<Vec<FileCommit>, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let limit = limit.unwrap_or(100);

    // git log --follow --format="%H|%an|%ad|%s" --date=short -n <limit> -- <file>
    let args = vec![
        "log".to_string(),
        "--follow".to_string(),
        format!("--format=%H|%an|%ad|%s"),
        "--date=short".to_string(),
        format!("-n{}", limit),
        "--".to_string(),
        file_path,
    ];

    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let mut commits = Vec::new();

    for line in resp.stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            commits.push(FileCommit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3..].join("|"), // Rejoin message in case it contained pipes
            });
        }
    }

    Ok(commits)
}

#[tauri::command]
pub async fn cmd_search_repo_files(
    state: State<'_, AppState>,
    pattern: Option<String>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // git ls-files lists all tracked files
    let args = vec!["ls-files".to_string()];

    let resp = state
        .git
        .run(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let pattern_lower = pattern.as_ref().map(|p| p.to_lowercase());

    let files: Vec<String> = resp
        .stdout
        .lines()
        .filter(|line| {
            let line = line.trim();
            if line.is_empty() {
                return false;
            }
            // If pattern provided, filter by case-insensitive match
            if let Some(ref pat) = pattern_lower {
                line.to_lowercase().contains(pat)
            } else {
                true
            }
        })
        .take(100) // Limit results to avoid overwhelming UI
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

// ---------------------------------------------------------------------------
// Diff Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_commit_diff(
    state: State<'_, AppState>,
    commit_hash: String,
    file_path: Option<String>,
    repo_path: Option<String>,
    encoding: Option<String>,
) -> Result<CommitDiff, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // 1. Get diff patch
    // git show --format= --first-parent --patch <commit> [-- <file_path>]
    let mut args = vec![
        "show".to_string(),
        "--format=".to_string(),
        "--first-parent".to_string(),
        "--patch".to_string(),
        commit_hash.clone(),
    ];
    if let Some(ref fp) = file_path {
        args.push("--".to_string());
        args.push(fp.clone());
    }

    let resp = state
        .git
        .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    // Decode with override or default
    // Decode with override or default
    let decoded_stdout = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if let Some(ref fp) = file_path {
            // If specific file, use its path context for config resolution
            crate::git::encoding::decode_bytes(&resp.stdout, Path::new(fp), &settings, encoding)
        } else {
            // If no file path (entire commit), use root path logic (likely just UTF-8 unless simple override)
            // For mixed files, applying one encoding is risky, but if user overrides, they want it.
            crate::git::encoding::decode_bytes(&resp.stdout, Path::new(""), &settings, encoding)
        }
    };

    // 2. Parse output
    let files = parse_diff_output(&decoded_stdout);

    // 3. Get parent hash
    let parent_hash_args = vec!["rev-parse".to_string(), format!("{}^", commit_hash)];
    let parent_hash = match state
        .git
        .run(Path::new(&path), &parent_hash_args, TIMEOUT_QUICK)
        .await
    {
        Ok(out) => Some(out.stdout.trim().to_string()),
        Err(_) => None, // Likely root commit
    };

    Ok(CommitDiff {
        commit_hash,
        parent_hash,
        files,
    })
}

#[tauri::command]
pub async fn cmd_get_file_at_commit(
    state: State<'_, AppState>,
    commit_hash: String,
    file_path: String,
    repo_path: Option<String>,
    encoding: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let object = format!("{}:{}", commit_hash, file_path);
    let args = vec!["show".to_string(), object];
    let resp = state
        .git
        .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(crate::git::encoding::decode_bytes(
        &resp.stdout,
        Path::new(&file_path),
        &settings,
        encoding,
    ))
}

fn parse_diff_file_path(line: &str) -> String {
    if let Some(idx) = line.find(" b/") {
        return line[idx + 3..].trim().to_string();
    }
    line.split_whitespace().last().unwrap_or("").to_string()
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    if !line.starts_with("@@") {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let old_ln = parts[1][1..]
        .split(',')
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    let new_ln = parts[2][1..]
        .split(',')
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    Some((old_ln, new_ln))
}

fn parse_hunk_line(line: &str, old_ln: &mut u32, new_ln: &mut u32) -> Option<DiffLine> {
    if line.starts_with('+') {
        let parsed = DiffLine {
            type_: DiffLineType::Add,
            content: line[1..].to_string(),
            old_line_number: None,
            new_line_number: Some(*new_ln),
        };
        *new_ln += 1;
        return Some(parsed);
    }
    if line.starts_with('-') {
        let parsed = DiffLine {
            type_: DiffLineType::Remove,
            content: line[1..].to_string(),
            old_line_number: Some(*old_ln),
            new_line_number: None,
        };
        *old_ln += 1;
        return Some(parsed);
    }
    if line.starts_with(' ') {
        let parsed = DiffLine {
            type_: DiffLineType::Context,
            content: line[1..].to_string(),
            old_line_number: Some(*old_ln),
            new_line_number: Some(*new_ln),
        };
        *old_ln += 1;
        *new_ln += 1;
        return Some(parsed);
    }
    None
}

fn push_hunk_if_present(file: &mut DiffFile, current_hunk: &mut Option<DiffHunk>) {
    if let Some(hunk) = current_hunk.take() {
        file.hunks.push(hunk);
    }
}

fn flush_current_file(
    files: &mut Vec<DiffFile>,
    current_file: &mut Option<DiffFile>,
    current_hunk: &mut Option<DiffHunk>,
) {
    if let Some(mut file) = current_file.take() {
        push_hunk_if_present(&mut file, current_hunk);
        files.push(file);
    }
}

fn parse_diff_output(stdout: &str) -> Vec<DiffFile> {
    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_ln: u32 = 0;
    let mut new_ln: u32 = 0;

    for line in stdout.lines() {
        if line.starts_with("diff --git") {
            flush_current_file(&mut files, &mut current_file, &mut current_hunk);
            current_file = Some(DiffFile {
                path: parse_diff_file_path(line),
                status: "M".to_string(),
                hunks: Vec::new(),
            });
            continue;
        }

        let Some(file) = current_file.as_mut() else {
            continue;
        };

        if line.starts_with("new file mode") {
            file.status = "A".to_string();
            continue;
        }
        if line.starts_with("deleted file mode") {
            file.status = "D".to_string();
            continue;
        }
        if line.starts_with("rename from") {
            file.status = "R".to_string();
            continue;
        }
        if line.starts_with("index")
            || line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with("Binary files")
        {
            continue;
        }

        if let Some((old_start, new_start)) = parse_hunk_header(line) {
            push_hunk_if_present(file, &mut current_hunk);
            old_ln = old_start;
            new_ln = new_start;
            current_hunk = Some(DiffHunk {
                id: Uuid::new_v4().to_string(),
                old_start,
                new_start,
                lines: Vec::new(),
            });
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            if let Some(parsed_line) = parse_hunk_line(line, &mut old_ln, &mut new_ln) {
                hunk.lines.push(parsed_line);
            }
        }
    }

    flush_current_file(&mut files, &mut current_file, &mut current_hunk);
    files
}

fn normalize_diff_tree_status(raw: &str) -> String {
    let s = raw.trim();
    if s == "??" {
        return "??".to_string();
    }
    match s.chars().next() {
        Some('A') => "A".to_string(),
        Some('M') => "M".to_string(),
        Some('D') => "D".to_string(),
        Some('R') => "R".to_string(),
        Some('C') => "C".to_string(),
        Some('T') => "T".to_string(),
        Some('U') => "U".to_string(),
        _ => "M".to_string(),
    }
}

fn status_priority(status: &str) -> u8 {
    match status {
        "U" => 70,
        "D" => 60,
        "A" => 50,
        "R" => 40,
        "C" => 35,
        "M" => 30,
        "T" => 20,
        "??" => 10,
        _ => 0,
    }
}

fn parse_diff_tree_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parts: Vec<&str> = trimmed.split('\t').collect();
    if parts.len() < 2 {
        return None;
    }

    let status = normalize_diff_tree_status(parts[0]);
    let file_path = if parts.len() >= 3 {
        parts[2].trim()
    } else {
        parts[1].trim()
    };
    if file_path.is_empty() {
        return None;
    }
    Some((file_path.to_string(), status))
}

fn merge_changed_file_status(
    ordered_paths: &mut Vec<String>,
    by_path_status: &mut HashMap<String, String>,
    file_path: String,
    normalized_status: String,
) {
    if let Some(existing_status) = by_path_status.get(&file_path) {
        if status_priority(&normalized_status) > status_priority(existing_status) {
            by_path_status.insert(file_path, normalized_status);
        }
        return;
    }
    ordered_paths.push(file_path.clone());
    by_path_status.insert(file_path, normalized_status);
}

fn parse_commit_changed_files_output(stdout: &str) -> Vec<CommitChangedFile> {
    let mut ordered_paths: Vec<String> = Vec::new();
    let mut by_path_status: HashMap<String, String> = HashMap::new();

    for line in stdout.lines() {
        let Some((file_path, normalized_status)) = parse_diff_tree_line(line) else {
            continue;
        };
        merge_changed_file_status(
            &mut ordered_paths,
            &mut by_path_status,
            file_path,
            normalized_status,
        );
    }

    ordered_paths
        .into_iter()
        .map(|path| CommitChangedFile {
            status: by_path_status
                .remove(&path)
                .unwrap_or_else(|| "M".to_string()),
            path,
        })
        .collect()
}

fn fetch_commit_changed_files_output(
    state: &State<'_, AppState>,
    repo_path: Option<String>,
    commit_hash: &str,
) -> Result<String, String> {
    let path = resolve_repo_path(state, repo_path)?;
    let args = vec![
        "diff-tree".to_string(),
        "--no-commit-id".to_string(),
        "--name-status".to_string(),
        "-r".to_string(),
        "-m".to_string(),
        "--root".to_string(),
        commit_hash.to_string(),
    ];

    let mut command = std::process::Command::new(state.git.binary_path());
    command.args(&args).current_dir(&path);
    hide_console_window(&mut command);

    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff-tree failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
pub async fn cmd_get_commit_changed_files(
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<Vec<CommitChangedFile>, String> {
    let stdout = fetch_commit_changed_files_output(&state, repo_path, &commit_hash)?;
    Ok(parse_commit_changed_files_output(&stdout))
}

#[tauri::command]
pub async fn cmd_get_commit_file_diff(
    state: State<'_, AppState>,
    commit_hash: String,
    file_path: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // git show <commit> -- <path>
    let mut command = std::process::Command::new(state.git.binary_path());
    command
        .args(&["show", &commit_hash, "--", &file_path])
        .current_dir(&path);
    hide_console_window(&mut command);

    let output = command.output().map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(GitCommandResult {
        success: output.status.success(),
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        command_type: GitCommandType::Other,
    })
}

// ---------------------------------------------------------------------------
// Rebase Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_rebase_status(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<FullRebaseStatus, String> {
    rebase_commands::cmd_get_rebase_status_impl(state, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_start(
    app: AppHandle,
    state: State<'_, AppState>,
    base: String,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    rebase_commands::cmd_rebase_start_impl(app, state, base, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_interactive_prepare(
    state: State<'_, AppState>,
    base_commit: String,
    repo_path: Option<String>,
) -> Result<Vec<RebaseTodoItem>, String> {
    rebase_commands::cmd_rebase_interactive_prepare_impl(state, base_commit, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_interactive_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    base_commit: String,
    todo_items: Vec<RebaseTodoItem>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    rebase_commands::cmd_rebase_interactive_apply_impl(app, state, base_commit, todo_items, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_continue(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    rebase_commands::cmd_rebase_continue_impl(app, state, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_abort(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    rebase_commands::cmd_rebase_abort_impl(app, state, repo_path).await
}

#[tauri::command]
pub async fn cmd_rebase_skip(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitCommandResult, String> {
    rebase_commands::cmd_rebase_skip_impl(app, state, repo_path).await
}

// ---------------------------------------------------------------------------
// Terminal Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_terminal_start(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    terminal_commands::cmd_terminal_start_impl(app, state, repo_path).await
}

#[tauri::command]
pub async fn cmd_terminal_write(
    state: State<'_, AppState>,
    repo_path: String,
    input: String,
) -> Result<(), String> {
    terminal_commands::cmd_terminal_write_impl(state, repo_path, input).await
}

#[tauri::command]
pub async fn cmd_terminal_stop(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    terminal_commands::cmd_terminal_stop_impl(state, repo_path).await
}
