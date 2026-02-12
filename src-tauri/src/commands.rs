use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::git::service::{TIMEOUT_LOCAL, TIMEOUT_NETWORK, TIMEOUT_QUICK};
use crate::git::{
    ConflictFile, DiagnosticInfo, GitCommandResult, GitCommandType, GitError, GitResponse,
    GitResult,
};
use crate::models::{CommitDiff, DiffFile, DiffHunk, DiffLine, DiffLineType, FileCommit};
use crate::settings::{save_settings, AppSettings, AppState, RepoEntry};
use glob::Pattern;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;

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

const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";
const GEMINI_MAX_DIFF_CHARS: usize = 40_000;
const GEMINI_MAX_FILE_SUMMARY_CHARS: usize = 4_000;
const GEMINI_LIST_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_MODELS_PAGE_SIZE: &str = "1000";

#[derive(Debug, Deserialize)]
struct GeminiModelsListResponse {
    #[serde(default)]
    models: Vec<GeminiModelEntry>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiModelEntry {
    name: Option<String>,
    #[serde(default, rename = "supportedGenerationMethods")]
    supported_generation_methods: Vec<String>,
}

fn normalize_gemini_model_name(raw_name: &str) -> Option<String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_prefix = trimmed.strip_prefix("models/").unwrap_or(trimmed);
    if without_prefix.is_empty() {
        return None;
    }

    if !without_prefix.starts_with("gemini") {
        return None;
    }

    Some(without_prefix.to_string())
}

fn truncate_for_prompt(input: &str, max_chars: usize) -> (String, bool) {
    let mut out = String::new();
    let mut truncated = false;

    for (idx, ch) in input.chars().enumerate() {
        if idx >= max_chars {
            truncated = true;
            break;
        }
        out.push(ch);
    }

    (out, truncated)
}

fn build_commit_message_prompt(
    staged_files: &str,
    staged_diff: &str,
    diff_was_truncated: bool,
) -> String {
    let mut prompt = String::from(
        "You are an expert software engineer writing Git commit messages.\n\
Task: Generate a single commit message from the staged changes.\n\
Rules:\n\
- Return plain text only (no markdown, no code fences).\n\
- Output format must be exactly:\n\
  <subject line>\n\
\n\
  <description/body>\n\
- Keep the subject line under 72 characters.\n\
- Use imperative voice.\n\
- Prefer Conventional Commit prefixes when clear (feat, fix, refactor, docs, test, chore).\n\
- Always include a short body (1-3 concise lines) explaining what changed and why.\n\
- Do not include labels like \"Subject:\" or \"Description:\".\n\n",
    );

    prompt.push_str("Staged files (name-status):\n");
    prompt.push_str(staged_files.trim());
    prompt.push_str("\n\nStaged diff:\n");
    prompt.push_str(staged_diff.trim());

    if diff_was_truncated {
        prompt.push_str("\n\n[NOTE] Diff content was truncated due to size.");
    }

    prompt
}

fn sanitize_commit_message(raw: &str) -> String {
    let mut text = raw.trim().to_string();

    if text.starts_with("```") {
        let mut lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            lines.remove(0);
        }
        if !lines.is_empty()
            && lines
                .last()
                .is_some_and(|line| line.trim().starts_with("```"))
        {
            lines.pop();
        }
        text = lines.join("\n").trim().to_string();
    }

    if let Some(rest) = text.strip_prefix("Commit message:") {
        text = rest.trim().to_string();
    }

    // Normalize optional labels if the model returns "Subject:" / "Description:".
    let mut normalized_lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.starts_with("subject:") {
            let rest = trimmed[8..].trim();
            if !rest.is_empty() {
                normalized_lines.push(rest.to_string());
            }
            continue;
        }

        if lower.starts_with("description:") {
            if !normalized_lines.is_empty()
                && normalized_lines
                    .last()
                    .is_some_and(|last| !last.trim().is_empty())
            {
                normalized_lines.push(String::new());
            }
            let rest = trimmed[12..].trim();
            if !rest.is_empty() {
                normalized_lines.push(rest.to_string());
            }
            continue;
        }

        normalized_lines.push(line.trim_end().to_string());
    }

    if !normalized_lines.is_empty() {
        text = normalized_lines.join("\n").trim().to_string();
    }

    text
}

fn ensure_commit_message_has_body(message: &str, staged_files: &str) -> String {
    let normalized = message.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let subject = lines.next().unwrap_or("").trim().to_string();

    if subject.is_empty() {
        return normalized.trim().to_string();
    }

    let has_body = lines.any(|line| !line.trim().is_empty());
    if has_body {
        return normalized.trim().to_string();
    }

    let file_count = staged_files
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let fallback_body = if file_count <= 1 {
        "Update staged changes in 1 file.".to_string()
    } else {
        format!("Update staged changes in {} files.", file_count)
    };

    format!("{}\n\n{}", subject, fallback_body)
}

fn extract_gemini_text(response_json: &serde_json::Value) -> Option<String> {
    let candidates = response_json.get("candidates")?.as_array()?;
    let first = candidates.first()?;
    let parts = first.get("content")?.get("parts")?.as_array()?;

    let mut out = String::new();
    for part in parts {
        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
            out.push_str(text);
        }
    }

    let trimmed = out.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageLineSelection {
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedPatchLineKind {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone)]
struct ParsedPatchLine {
    kind: ParsedPatchLineKind,
    content: String,
    old_line: Option<u32>,
    new_line: Option<u32>,
    old_anchor: u32,
    new_anchor: u32,
}

#[derive(Debug, Clone)]
struct ParsedPatchHunk {
    lines: Vec<ParsedPatchLine>,
}

#[derive(Debug, Clone)]
struct ParsedUnstagedPatch {
    header_lines: Vec<String>,
    hunks: Vec<ParsedPatchHunk>,
}

fn parse_hunk_range(token: &str, prefix: char) -> Result<(u32, u32), String> {
    if !token.starts_with(prefix) {
        return Err(format!("Invalid hunk token '{}'", token));
    }
    let range = &token[1..];
    let mut parts = range.splitn(2, ',');
    let start = parts
        .next()
        .unwrap_or("0")
        .parse::<u32>()
        .map_err(|_| format!("Invalid hunk range start '{}'", token))?;
    let count = match parts.next() {
        Some(value) => value
            .parse::<u32>()
            .map_err(|_| format!("Invalid hunk range count '{}'", token))?,
        None => 1,
    };
    Ok((start, count))
}

fn parse_unstaged_zero_context_diff(diff_output: &str) -> Result<ParsedUnstagedPatch, String> {
    let mut header_lines: Vec<String> = Vec::new();
    let mut hunks: Vec<ParsedPatchHunk> = Vec::new();
    let mut current_hunk: Option<ParsedPatchHunk> = None;
    let mut old_cursor: u32 = 0;
    let mut new_cursor: u32 = 0;

    for line in diff_output.lines() {
        if line.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                return Err(format!("Invalid hunk header '{}'", line));
            }
            let (old_start, _) = parse_hunk_range(parts[1], '-')?;
            let (new_start, _) = parse_hunk_range(parts[2], '+')?;

            old_cursor = old_start;
            new_cursor = new_start;
            current_hunk = Some(ParsedPatchHunk { lines: Vec::new() });
            continue;
        }

        if current_hunk.is_none() {
            header_lines.push(line.to_string());
            continue;
        }

        if line.starts_with("\\ No newline at end of file") {
            continue;
        }

        if line.starts_with("diff --git ") {
            break;
        }

        let hunk = current_hunk
            .as_mut()
            .ok_or("Internal patch parser error".to_string())?;

        if let Some(content) = line.strip_prefix('+') {
            hunk.lines.push(ParsedPatchLine {
                kind: ParsedPatchLineKind::Add,
                content: content.to_string(),
                old_line: None,
                new_line: Some(new_cursor),
                old_anchor: old_cursor,
                new_anchor: new_cursor,
            });
            new_cursor += 1;
            continue;
        }

        if let Some(content) = line.strip_prefix('-') {
            hunk.lines.push(ParsedPatchLine {
                kind: ParsedPatchLineKind::Remove,
                content: content.to_string(),
                old_line: Some(old_cursor),
                new_line: None,
                old_anchor: old_cursor,
                new_anchor: new_cursor,
            });
            old_cursor += 1;
            continue;
        }

        if let Some(content) = line.strip_prefix(' ') {
            hunk.lines.push(ParsedPatchLine {
                kind: ParsedPatchLineKind::Context,
                content: content.to_string(),
                old_line: Some(old_cursor),
                new_line: Some(new_cursor),
                old_anchor: old_cursor,
                new_anchor: new_cursor,
            });
            old_cursor += 1;
            new_cursor += 1;
        }
    }

    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    if hunks.is_empty() {
        return Err("No unstaged diff hunks available for selected file".to_string());
    }

    if header_lines.is_empty() {
        return Err("Unable to parse diff header".to_string());
    }

    Ok(ParsedUnstagedPatch {
        header_lines,
        hunks,
    })
}

fn find_patch_line_index(
    patch: &ParsedUnstagedPatch,
    kind: ParsedPatchLineKind,
    old_line: Option<u32>,
    new_line: Option<u32>,
) -> Option<(usize, usize)> {
    for (hunk_idx, hunk) in patch.hunks.iter().enumerate() {
        for (line_idx, line) in hunk.lines.iter().enumerate() {
            if line.kind != kind {
                continue;
            }
            if let Some(target_old) = old_line {
                if line.old_line != Some(target_old) {
                    continue;
                }
            }
            if let Some(target_new) = new_line {
                if line.new_line != Some(target_new) {
                    continue;
                }
            }
            return Some((hunk_idx, line_idx));
        }
    }
    None
}

fn build_stage_line_patch(
    patch: &ParsedUnstagedPatch,
    selection: &StageLineSelection,
) -> Result<String, String> {
    let mut patch_lines = patch.header_lines.clone();

    match (selection.old_line_number, selection.new_line_number) {
        (Some(old_line_number), Some(new_line_number)) => {
            let remove_idx = find_patch_line_index(
                patch,
                ParsedPatchLineKind::Remove,
                Some(old_line_number),
                None,
            )
            .ok_or(format!(
                "Unable to find removed line {} in unstaged diff",
                old_line_number
            ))?;
            let add_idx =
                find_patch_line_index(patch, ParsedPatchLineKind::Add, None, Some(new_line_number))
                    .ok_or(format!(
                        "Unable to find added line {} in unstaged diff",
                        new_line_number
                    ))?;

            if remove_idx.0 != add_idx.0 {
                return Err("Selected modified line pair is in different hunks".to_string());
            }

            let remove_line = &patch.hunks[remove_idx.0].lines[remove_idx.1];
            let add_line = &patch.hunks[add_idx.0].lines[add_idx.1];
            let old_start = remove_line
                .old_line
                .ok_or("Selected removed line is missing old line number".to_string())?;
            let new_start = add_line
                .new_line
                .ok_or("Selected added line is missing new line number".to_string())?;

            patch_lines.push(format!("@@ -{},1 +{},1 @@", old_start, new_start));
            patch_lines.push(format!("-{}", remove_line.content));
            patch_lines.push(format!("+{}", add_line.content));
        }
        (Some(old_line_number), None) => {
            let remove_idx = find_patch_line_index(
                patch,
                ParsedPatchLineKind::Remove,
                Some(old_line_number),
                None,
            )
            .ok_or(format!(
                "Unable to find removed line {} in unstaged diff",
                old_line_number
            ))?;

            let remove_line = &patch.hunks[remove_idx.0].lines[remove_idx.1];
            let old_start = remove_line
                .old_line
                .ok_or("Selected removed line is missing old line number".to_string())?;
            patch_lines.push(format!(
                "@@ -{},1 +{},0 @@",
                old_start, remove_line.new_anchor
            ));
            patch_lines.push(format!("-{}", remove_line.content));
        }
        (None, Some(new_line_number)) => {
            let add_idx =
                find_patch_line_index(patch, ParsedPatchLineKind::Add, None, Some(new_line_number))
                    .ok_or(format!(
                        "Unable to find added line {} in unstaged diff",
                        new_line_number
                    ))?;

            let add_line = &patch.hunks[add_idx.0].lines[add_idx.1];
            let new_start = add_line
                .new_line
                .ok_or("Selected added line is missing new line number".to_string())?;
            patch_lines.push(format!("@@ -{},0 +{},1 @@", add_line.old_anchor, new_start));
            patch_lines.push(format!("+{}", add_line.content));
        }
        (None, None) => {
            return Err("Stage-line selection is empty".to_string());
        }
    }

    let mut output = patch_lines.join("\n");
    output.push('\n');
    Ok(output)
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
        id: id.clone(),
        name,
        path,
    });

    // Auto-open on add
    if !settings.open_repo_ids.contains(&id) {
        settings.open_repo_ids.push(id);
    }

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
    settings.open_repo_ids.retain(|r_id| *r_id != id);

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

    settings.active_repo_id = Some(id.clone());

    // Auto-open on set active if not already open
    if !settings.open_repo_ids.contains(&id) {
        settings.open_repo_ids.push(id);
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_open_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if !settings.repos.iter().any(|r| r.id == id) {
        return Err("Repository ID not found".to_string());
    }

    if !settings.open_repo_ids.contains(&id) {
        settings.open_repo_ids.push(id);
        save_settings(&app_handle, &settings)?;
    }

    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_close_repo(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if let Some(pos) = settings.open_repo_ids.iter().position(|r_id| *r_id == id) {
        settings.open_repo_ids.remove(pos);

        // If closing active repo, switch to another one
        if settings.active_repo_id.as_ref() == Some(&id) {
            // Try to switch to right neighbor, else left neighbor, else None
            let next_active = if pos < settings.open_repo_ids.len() {
                Some(settings.open_repo_ids[pos].clone())
            } else if pos > 0 {
                Some(settings.open_repo_ids[pos - 1].clone())
            } else {
                None
            };
            settings.active_repo_id = next_active;
        }

        // Also stop terminal session for this repo to clean up resources
        let _ = state.terminal.stop_session(&id); // Note: terminal uses repo_path, resolving ID might be needed here.
                                                  // Wait, terminal manager uses repo_path. We need to find path from ID.
        if let Some(repo) = settings.repos.iter().find(|r| r.id == id) {
            let _ = state.terminal.stop_session(&repo.path);
        }

        save_settings(&app_handle, &settings)?;
    }

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

#[tauri::command]
pub fn cmd_set_repo_filter(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_id: String,
    filter: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if filter.is_empty() {
        settings.repo_filters.remove(&repo_id);
    } else {
        settings.repo_filters.insert(repo_id, filter);
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_set_gemini_api_token(
    app_handle: AppHandle,
    state: State<AppState>,
    token: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = token.trim().to_string();
    settings.gemini_api_token = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn cmd_set_gemini_model(
    app_handle: AppHandle,
    state: State<AppState>,
    model: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = model.trim().to_string();
    settings.gemini_model = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn cmd_get_gemini_models(
    state: State<'_, AppState>,
    token: Option<String>,
) -> Result<Vec<String>, String> {
    let provided_token = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    let api_token = if let Some(t) = provided_token {
        t
    } else {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings
            .gemini_api_token
            .clone()
            .ok_or("Gemini API token is missing. Set it in Settings first.")?
    };

    if api_token.trim().is_empty() {
        return Err("Gemini API token is missing. Set it in Settings first.".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_NETWORK))
        .build()
        .map_err(|e| format!("Failed to initialize Gemini client: {}", e))?;

    let mut next_page_token: Option<String> = None;
    let mut models = HashSet::new();

    loop {
        let mut request = client
            .get(GEMINI_LIST_MODELS_URL)
            .header("x-goog-api-key", &api_token)
            .query(&[("pageSize", GEMINI_MODELS_PAGE_SIZE)]);

        if let Some(page_token) = next_page_token.as_deref() {
            request = request.query(&[("pageToken", page_token)]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to call Gemini API: {}", e))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read Gemini response: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Gemini API error while listing models ({}): {}",
                status, body
            ));
        }

        let parsed: GeminiModelsListResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Invalid Gemini model list response: {}", e))?;

        for model in parsed.models {
            let Some(raw_name) = model.name else {
                continue;
            };

            if !model.supported_generation_methods.is_empty()
                && !model
                    .supported_generation_methods
                    .iter()
                    .any(|method| method == "generateContent")
            {
                continue;
            }

            if let Some(normalized_name) = normalize_gemini_model_name(&raw_name) {
                models.insert(normalized_name);
            }
        }

        next_page_token = parsed
            .next_page_token
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if next_page_token.is_none() {
            break;
        }
    }

    if models.is_empty() {
        return Err("No Gemini models found for this API key.".to_string());
    }

    let mut sorted_models: Vec<String> = models.into_iter().collect();
    sorted_models.sort_unstable();
    Ok(sorted_models)
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
    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(map_git_result(resp, GitCommandType::Commit))
}

#[tauri::command]
pub async fn cmd_generate_commit_message(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    let (token, model) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        let token = settings.gemini_api_token.clone();
        let model = settings
            .gemini_model
            .clone()
            .unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string());
        (token, model)
    };

    let token = token.ok_or("Gemini API token is missing. Set it in Settings first.")?;
    let model = if model.trim().is_empty() {
        DEFAULT_GEMINI_MODEL.to_string()
    } else {
        model.trim().to_string()
    };

    let staged_files_args: Vec<String> =
        vec!["diff".into(), "--cached".into(), "--name-status".into()];
    let staged_files_resp = state
        .git
        .run(Path::new(&path), &staged_files_args, TIMEOUT_QUICK)
        .await
        .map_err(|e| e.to_string())?;

    let staged_files = staged_files_resp.stdout.trim().to_string();
    if staged_files.is_empty() {
        return Err("No staged files found. Stage your changes first.".to_string());
    }

    let staged_diff_args: Vec<String> = vec![
        "diff".into(),
        "--cached".into(),
        "--patch".into(),
        "--no-color".into(),
        "--unified=3".into(),
    ];
    let staged_diff_resp = state
        .git
        .run(Path::new(&path), &staged_diff_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let (staged_files_for_prompt, _) =
        truncate_for_prompt(&staged_files, GEMINI_MAX_FILE_SUMMARY_CHARS);
    let (staged_diff_for_prompt, diff_was_truncated) =
        truncate_for_prompt(&staged_diff_resp.stdout, GEMINI_MAX_DIFF_CHARS);

    let prompt = build_commit_message_prompt(
        &staged_files_for_prompt,
        &staged_diff_for_prompt,
        diff_was_truncated,
    );

    let api_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let payload = json!({
        "contents": [
            {
                "parts": [
                    { "text": prompt }
                ]
            }
        ],
        "generationConfig": {
            "temperature": 0.2,
            "topP": 0.9,
            "maxOutputTokens": 320
        }
    });

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_NETWORK))
        .build()
        .map_err(|e| format!("Failed to initialize Gemini client: {}", e))?;

    let response = client
        .post(&api_url)
        .header("x-goog-api-key", token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call Gemini API: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Gemini response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Gemini API error ({}): {}", status, body));
    }

    let response_json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid Gemini response: {}", e))?;

    let generated = if let Some(text) = extract_gemini_text(&response_json) {
        text
    } else if let Some(message) = response_json
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
    {
        return Err(format!("Gemini API error: {}", message));
    } else {
        return Err("Gemini did not return any commit message text.".to_string());
    };

    let sanitized = sanitize_commit_message(&generated);
    let message = ensure_commit_message_has_body(&sanitized, &staged_files);
    if message.trim().is_empty() {
        return Err("Gemini returned an empty commit message.".to_string());
    }

    Ok(message)
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitChangedFile {
    pub path: String,
    pub status: String,
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
                let mut path = line[3..].trim().to_string();
                if path.starts_with('"') && path.ends_with('"') {
                    path = path[1..path.len() - 1].to_string();
                }

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
        if chars.len() < 2 {
            continue;
        }

        let x = chars[0];
        let y = chars[1];
        let mut file_path = line[3..].trim().to_string();

        if file_path.starts_with('"') && file_path.ends_with('"') {
            file_path = file_path[1..file_path.len() - 1].to_string();
        }

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
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    let mut args = vec!["diff".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    args.push("--".to_string());
    args.push(file_path.clone());

    let resp = state
        .git
        .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_LOCAL) // Use byte-oriented run
        .await
        .map_err(|e| e.to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    // Decode with configured encoding or fallback to UTF-8 lossy
    let content = crate::git::encoding::decode_bytes(
        &resp.stdout,
        Path::new(&file_path),
        &settings,
        encoding,
    );

    Ok(content)
}

#[tauri::command]
pub async fn cmd_get_file_base_content(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let show_arg = if staged {
        format!("HEAD:{}", file_path)
    } else {
        format!(":{}", file_path)
    };
    let args = vec!["show".to_string(), show_arg];

    // For staged diff base: HEAD, for unstaged diff base: index.
    // New/untracked files won't exist in either source -> return empty.
    match state
        .git
        .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_QUICK)
        .await
    {
        Ok(resp) => {
            let settings = state.settings.lock().map_err(|e| e.to_string())?;
            Ok(crate::git::encoding::decode_bytes(
                &resp.stdout,
                Path::new(&file_path),
                &settings,
                encoding,
            ))
        }
        Err(_) => Ok(String::new()),
    }
}

#[tauri::command]
pub async fn cmd_get_file_modified_content(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    if staged {
        // Staged content lives in the index (stage 0)
        let show_arg = format!(":{}", file_path);
        let args = vec!["show".to_string(), show_arg];
        match state
            .git
            .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_QUICK)
            .await
        {
            Ok(resp) => {
                let settings = state.settings.lock().map_err(|e| e.to_string())?;
                Ok(crate::git::encoding::decode_bytes(
                    &resp.stdout,
                    Path::new(&file_path),
                    &settings,
                    encoding,
                ))
            }
            Err(_) => Ok(String::new()),
        }
    } else {
        // Unstaged content: read directly from the working directory
        let full_path = Path::new(&path).join(&file_path);
        match std::fs::read(&full_path) {
            Ok(bytes) => {
                let settings = state.settings.lock().map_err(|e| e.to_string())?;
                Ok(crate::git::encoding::decode_bytes(
                    &bytes,
                    Path::new(&file_path),
                    &settings,
                    encoding,
                ))
            }
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
pub async fn cmd_git_stage_line(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    line: StageLineSelection,
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

    if path.contains(" -> ") {
        return Err("Stage-line is not supported for rename paths".to_string());
    }

    let diff_args: Vec<String> = vec![
        "diff".into(),
        "--no-color".into(),
        "--no-ext-diff".into(),
        "--unified=0".into(),
        "--".into(),
        path.clone(),
    ];
    let diff_resp = state
        .git
        .run(Path::new(&r_path), &diff_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    if diff_resp.stdout.trim().is_empty() {
        return Err("No unstaged diff available for selected file".to_string());
    }

    let parsed = parse_unstaged_zero_context_diff(&diff_resp.stdout)?;
    let patch = build_stage_line_patch(&parsed, &line)?;

    let temp_patch_path =
        std::env::temp_dir().join(format!("git-tools-stage-line-{}.patch", Uuid::new_v4()));
    std::fs::write(&temp_patch_path, patch.as_bytes())
        .map_err(|e| format!("Failed to write temporary patch file: {}", e))?;

    let apply_args: Vec<String> = vec![
        "apply".into(),
        "--cached".into(),
        "--unidiff-zero".into(),
        "--whitespace=nowarn".into(),
        temp_patch_path.to_string_lossy().to_string(),
    ];

    let apply_result = state
        .git
        .run(Path::new(&r_path), &apply_args, TIMEOUT_LOCAL)
        .await;

    let _ = std::fs::remove_file(&temp_patch_path);

    apply_result.map_err(|e| e.to_string())?;

    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_git_unstage_line(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    line: StageLineSelection,
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

    if path.contains(" -> ") {
        return Err("Unstage-line is not supported for rename paths".to_string());
    }

    let diff_args: Vec<String> = vec![
        "diff".into(),
        "--cached".into(),
        "--no-color".into(),
        "--no-ext-diff".into(),
        "--unified=0".into(),
        "--".into(),
        path.clone(),
    ];
    let diff_resp = state
        .git
        .run(Path::new(&r_path), &diff_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    if diff_resp.stdout.trim().is_empty() {
        return Err("No staged diff available for selected file".to_string());
    }

    let parsed = parse_unstaged_zero_context_diff(&diff_resp.stdout)?;
    let patch = build_stage_line_patch(&parsed, &line)?;

    let temp_patch_path =
        std::env::temp_dir().join(format!("git-tools-unstage-line-{}.patch", Uuid::new_v4()));
    std::fs::write(&temp_patch_path, patch.as_bytes())
        .map_err(|e| format!("Failed to write temporary patch file: {}", e))?;

    let apply_args: Vec<String> = vec![
        "apply".into(),
        "--cached".into(),
        "--reverse".into(),
        "--unidiff-zero".into(),
        "--whitespace=nowarn".into(),
        temp_patch_path.to_string_lossy().to_string(),
    ];

    let apply_result = state
        .git
        .run(Path::new(&r_path), &apply_args, TIMEOUT_LOCAL)
        .await;

    let _ = std::fs::remove_file(&temp_patch_path);

    apply_result.map_err(|e| e.to_string())?;

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

        if file.status.trim() == "??" {
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

    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
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

    let stash_path = if let Some((_old_path, new_path)) = split_rename_path(raw_path) {
        new_path
    } else {
        raw_path.to_string()
    };

    if is_excluded(&stash_path, &exclusions) {
        return Err(format!(
            "File {} is excluded from git operations",
            stash_path
        ));
    }

    let include_untracked = file.status.trim() == "??";
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

    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
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

    app.emit("git-event", json!({ "type": "change" }))
        .map_err(|e| e.to_string())?;
    Ok(())
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

    let target_path = if let Some((_old_path, new_path)) = split_rename_path(raw_path) {
        new_path
    } else {
        raw_path.to_string()
    };

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

// ---------------------------------------------------------------------------
// Conflict Resolution Commands (all async)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_get_conflicts(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    let resp = git_run(&state, repo_path, &["status", "--porcelain"], TIMEOUT_LOCAL).await?;

    let mut conflicts = Vec::new();
    for line in resp.stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let status = &line[0..2];
        match status {
            "UU" | "AA" | "DU" | "UD" => {
                let mut path = line[3..].trim().to_string();
                if path.starts_with('"') && path.ends_with('"') {
                    path = path[1..path.len() - 1].to_string();
                }
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

    Ok(ConflictFile { base, ours, theirs })
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

    fs::write(&full_path, content).map_err(|e| format!("Failed to write file {}: {}", path, e))?;

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
    let args: Vec<String> = vec!["checkout".into(), "-b".into(), name, start_point];
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

fn parse_diff_output(stdout: &str) -> Vec<DiffFile> {
    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    let mut old_ln: u32 = 0;
    let mut new_ln: u32 = 0;

    for line in stdout.lines() {
        if line.starts_with("diff --git") {
            if let Some(mut f) = current_file.take() {
                if let Some(h) = current_hunk.take() {
                    f.hunks.push(h);
                }
                files.push(f);
            }
            // diff --git a/path b/path
            // Parse path from " b/" to end
            let path = if let Some(idx) = line.find(" b/") {
                line[idx + 3..].trim().to_string()
            } else {
                // Fallback
                line.split_whitespace().last().unwrap_or("").to_string()
            };

            current_file = Some(DiffFile {
                path,
                status: "M".to_string(),
                hunks: Vec::new(),
            });
            current_hunk = None;
            continue;
        }

        if let Some(ref mut file) = current_file {
            if line.starts_with("new file mode") {
                file.status = "A".to_string();
            } else if line.starts_with("deleted file mode") {
                file.status = "D".to_string();
            } else if line.starts_with("rename from") {
                file.status = "R".to_string();
            } else if line.starts_with("index")
                || line.starts_with("---")
                || line.starts_with("+++")
            {
                // Skip headers
                continue;
            } else if line.starts_with("Binary files") {
                // Handle binary - for now just leave hunks empty, maybe status is impacted
            } else if line.starts_with("@@") {
                // Push previous hunk
                if let Some(h) = current_hunk.take() {
                    file.hunks.push(h);
                }

                // Parse ranges: @@ -old,len +new,len @@
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    // -old,len
                    let old_part = &parts[1][1..];
                    old_ln = old_part
                        .split(',')
                        .next()
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0);

                    // +new,len
                    let new_part = &parts[2][1..];
                    new_ln = new_part
                        .split(',')
                        .next()
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0);

                    current_hunk = Some(DiffHunk {
                        id: Uuid::new_v4().to_string(),
                        old_start: old_ln,
                        new_start: new_ln,
                        lines: Vec::new(),
                    });
                }
            } else if let Some(ref mut hunk) = current_hunk {
                if line.starts_with('+') {
                    hunk.lines.push(DiffLine {
                        type_: DiffLineType::Add,
                        content: line[1..].to_string(),
                        old_line_number: None,
                        new_line_number: Some(new_ln),
                    });
                    new_ln += 1;
                } else if line.starts_with('-') {
                    // Removed line
                    hunk.lines.push(DiffLine {
                        type_: DiffLineType::Remove,
                        content: line[1..].to_string(),
                        old_line_number: Some(old_ln),
                        new_line_number: None,
                    });
                    old_ln += 1;
                } else if line.starts_with(' ') {
                    // Context line
                    hunk.lines.push(DiffLine {
                        type_: DiffLineType::Context,
                        content: line[1..].to_string(),
                        old_line_number: Some(old_ln),
                        new_line_number: Some(new_ln),
                    });
                    old_ln += 1;
                    new_ln += 1;
                }
            }
        }
    }

    // Flush last
    if let Some(mut f) = current_file.take() {
        if let Some(h) = current_hunk.take() {
            f.hunks.push(h);
        }
        files.push(f);
    }

    files
}

#[tauri::command]
pub async fn cmd_get_commit_changed_files(
    state: State<'_, AppState>,
    commit_hash: String,
    repo_path: Option<String>,
) -> Result<Vec<CommitChangedFile>, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    // Include:
    // - merge commits (-m): list files changed against each parent
    // - root commit (--root): list files introduced by the initial commit
    // Then deduplicate to return a clean file list + normalized status for the UI.
    // git diff-tree --no-commit-id --name-status -r -m --root <commit_hash>
    let args = vec![
        "diff-tree".to_string(),
        "--no-commit-id".to_string(),
        "--name-status".to_string(),
        "-r".to_string(),
        "-m".to_string(),
        "--root".to_string(),
        commit_hash,
    ];

    let mut command = std::process::Command::new(state.git.binary_path());
    command.args(&args).current_dir(&path);
    hide_console_window(&mut command);

    let output = command.output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff-tree failed: {}", stderr));
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
            Some('R') => "R".to_string(), // rename (R100, R090...)
            Some('C') => "C".to_string(), // copy (C100...)
            Some('T') => "T".to_string(), // type change
            Some('U') => "U".to_string(), // unmerged
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

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Keep insertion order stable while allowing us to merge duplicate rows from merge commits.
    let mut ordered_paths: Vec<String> = Vec::new();
    let mut by_path_status: HashMap<String, String> = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // --name-status output:
        // M\tpath
        // A\tpath
        // D\tpath
        // R100\told\tnew
        // C100\told\tnew
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }

        let normalized_status = normalize_diff_tree_status(parts[0]);
        let file_path = if parts.len() >= 3 {
            parts[2].trim()
        } else {
            parts[1].trim()
        };

        if file_path.is_empty() {
            continue;
        }

        let file_path = file_path.to_string();
        if let Some(existing_status) = by_path_status.get(&file_path) {
            if status_priority(&normalized_status) > status_priority(existing_status) {
                by_path_status.insert(file_path, normalized_status);
            }
        } else {
            ordered_paths.push(file_path.clone());
            by_path_status.insert(file_path, normalized_status);
        }
    }

    let files = ordered_paths
        .into_iter()
        .map(|path| CommitChangedFile {
            status: by_path_status
                .remove(&path)
                .unwrap_or_else(|| "M".to_string()),
            path,
        })
        .collect();

    Ok(files)
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
// Terminal Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cmd_terminal_start(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    state.terminal.start_session(app, repo_path)
}

#[tauri::command]
pub async fn cmd_terminal_write(
    state: State<'_, AppState>,
    repo_path: String,
    input: String,
) -> Result<(), String> {
    state.terminal.write_input(&repo_path, &input)
}

#[tauri::command]
pub async fn cmd_terminal_stop(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    state.terminal.stop_session(&repo_path)
}
