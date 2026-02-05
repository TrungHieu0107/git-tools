use crate::git::{GitCommandService, GitResponse, GitResult, ConflictFile};
use crate::settings::{AppSettings, AppState, RepoEntry, save_settings};
use std::path::PathBuf;
use tauri::{AppHandle, State};
use uuid::Uuid;

// --- Settings Commands ---

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

// --- Git Commands ---

// Helper to get active repo path or explicit override
fn resolve_repo_path(state: &State<AppState>, explicit_path: Option<String>) -> Result<String, String> {
    if let Some(path) = explicit_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
             return Ok(trimmed.to_string());
        }
    }
    get_active_repo_path(state)
}

// Helper to get active repo path (legacy/internal)
fn get_active_repo_path(state: &State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    // Only error if we actually need the active repo and none is set
    let active_id = settings.active_repo_id.as_ref().ok_or("No active repository selected")?;
    let repo = settings.repos.iter().find(|r| &r.id == active_id).ok_or("Active repository not found in settings")?;
    Ok(repo.path.clone())
}

// Helper for synchronous git execution with logging
fn execute_git_command(path: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    println!("[GIT SYNC START] Command: git {:?} Cwd: {}", args, path);

    let output = std::process::Command::new("git")
        .current_dir(path)
        .args(args)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .output()
        .map_err(|e| format!("Failed to execute git command: {} (args: {:?})", e, args))?;

    let duration = start.elapsed();
    println!("[GIT SYNC END] Code: {:?} Duration: {:?} Stdout: {}b Stderr: {}b", 
        output.status.code(), duration, output.stdout.len(), output.stderr.len());

    Ok(output)
}

#[tauri::command]
pub async fn run_git(
    state: State<'_, AppState>,
    subcommand: Vec<String>,
    repo_path: Option<String>,
) -> GitResult<GitResponse> {
    // Use the resolve helper
    let path = resolve_repo_path(&state, repo_path).map_err(|e| crate::git::GitError::CommandError(e))?;
    let service = GitCommandService::new(PathBuf::from(path));
    service.run(subcommand).await
}

#[tauri::command]
pub fn cmd_get_conflicts(state: State<AppState>, repo_path: Option<String>) -> Result<Vec<String>, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["status", "--porcelain"])?;

    if !output.status.success() {
        return Err(format!(
            "Git command failed with status code: {}",
            output.status
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse command output: {}", e))?;

    let mut conflicts = Vec::new();

    for line in stdout.lines() {
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
pub fn cmd_get_conflict_file(state: State<AppState>, path: String, repo_path: Option<String>) -> Result<ConflictFile, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;

    let run_show = |stage: &str| -> Result<String, String> {
        let arg = format!(":{}:{}", stage, path);
        let output = execute_git_command(&r_path, &["show", &arg])?;

        if !output.status.success() {
             return Err(format!(
                "Git show :{}:{} failed with status code: {}",
                stage, path, output.status
            ));
        }
        
        String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse output for stage {}: {}", stage, e))
    };

    let base = run_show("1")?;
    let ours = run_show("2")?;
    let theirs = run_show("3")?;

    Ok(ConflictFile {
        base,
        ours,
        theirs,
    })
}

#[tauri::command]
pub fn cmd_resolve_ours(state: State<AppState>, path: String, repo_path: Option<String>) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&r_path, &["checkout", "--ours", &path])?;

    if !output.status.success() {
        return Err(format!(
            "Git resolve ours failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(())
}

#[tauri::command]
pub fn cmd_resolve_theirs(state: State<AppState>, path: String, repo_path: Option<String>) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&r_path, &["checkout", "--theirs", &path])?;

    if !output.status.success() {
         return Err(format!(
            "Git resolve theirs failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[tauri::command]
pub fn cmd_mark_resolved(state: State<AppState>, path: String, repo_path: Option<String>) -> Result<(), String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&r_path, &["add", &path])?;

    if !output.status.success() {
         return Err(format!(
            "Git mark resolved failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(())
}

#[tauri::command]
pub fn cmd_write_file(state: State<AppState>, path: String, content: String, repo_path: Option<String>) -> Result<(), String> {
    use std::fs;
    use std::path::Path;
    let r_path = resolve_repo_path(&state, repo_path)?;

    let full_path = Path::new(&r_path).join(&path);

    if !full_path.starts_with(&r_path) {
         return Err("Invalid path: cannot write outside of repository".to_string());
    }

    fs::write(&full_path, content)
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(())
}

#[tauri::command]
pub fn cmd_git_status(state: State<AppState>, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["status"])?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_pull(state: State<AppState>, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["pull"])?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_push(state: State<AppState>, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["push"])?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_fetch(state: State<AppState>, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["fetch"])?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_commit(state: State<AppState>, message: String, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["commit", "-m", &message])?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_add_all(state: State<AppState>, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["add", "."])?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_checkout(state: State<AppState>, branch: String, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["checkout", &branch])?;

    if output.status.success() {
         // checkout output often goes to stderr even on success
         let out = String::from_utf8_lossy(&output.stdout).to_string();
         let err = String::from_utf8_lossy(&output.stderr).to_string();
         Ok(format!("{}{}", out, err))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_branch_list(state: State<AppState>, repo_path: Option<String>) -> Result<Vec<String>, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let output = execute_git_command(&path, &["branch", "--format=%(refname:short)"])?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout.lines().map(|s| s.trim().to_string()).collect())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_log(state: State<AppState>, limit: usize, repo_path: Option<String>) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let limit_str = format!("-n{}", limit);
    let output = execute_git_command(&path, &["log", &limit_str, "--oneline", "--graph", "--decorate", "--no-pager"])?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_check_conflict_state(state: State<AppState>, repo_path: Option<String>) -> Result<bool, String> {
    use std::path::Path;
    
    let path = resolve_repo_path(&state, repo_path)?;
    let p = Path::new(&path);
    let git_dir = p.join(".git");

    // 1. Check for merge/rebase/cherry-pick heads
    let is_merging = git_dir.join("MERGE_HEAD").exists();
    let is_rebasing = git_dir.join("REBASE_HEAD").exists() || 
                      git_dir.join("rebase-merge").exists() || 
                      git_dir.join("rebase-apply").exists(); 
    let is_cherry_picking = git_dir.join("CHERRY_PICK_HEAD").exists();
    let is_reverting = git_dir.join("REVERT_HEAD").exists();

    if !is_merging && !is_rebasing && !is_cherry_picking && !is_reverting {
        return Ok(false);
    }

    // 2. If in state, check for unmerged files
    let output = execute_git_command(&path, &["status", "--porcelain"])?;

    if !output.status.success() {
        return Err("Failed to check git status".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.len() >= 2 {
            let status = &line[0..2];
            // Unmerged status codes: DD, AU, UD, UA, DU, AA, UU
            if matches!(status, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU") {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
