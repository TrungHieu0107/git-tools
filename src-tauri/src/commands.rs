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

// Helper to get active repo path
fn get_active_repo_path(state: &State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let active_id = settings.active_repo_id.as_ref().ok_or("No active repository selected")?;
    let repo = settings.repos.iter().find(|r| &r.id == active_id).ok_or("Active repository not found in settings")?;
    Ok(repo.path.clone())
}

#[tauri::command]
pub async fn run_git(
    state: State<'_, AppState>,
    subcommand: Vec<String>,
) -> GitResult<GitResponse> {
    let repo_path = get_active_repo_path(&state).map_err(|e| crate::git::GitError::CommandError(e))?;
    let service = GitCommandService::new(PathBuf::from(repo_path));
    service.run(subcommand).await
}

#[tauri::command]
pub fn cmd_get_conflicts(state: State<AppState>) -> Result<Vec<String>, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git command within {}: {}", repo_path, e))?;

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
pub fn cmd_get_conflict_file(state: State<AppState>, path: String) -> Result<ConflictFile, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let run_show = |stage: &str| -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["show", &format!(":{}:{}", stage, path)])
            .output()
            .map_err(|e| format!("Failed to execute git show :{:?}:{}: {}", stage, path, e))?;

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
pub fn cmd_resolve_ours(state: State<AppState>, path: String) -> Result<(), String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", "--ours", &path])
        .output()
        .map_err(|e| format!("Failed to execute git checkout --ours {}: {}", path, e))?;

    if !output.status.success() {
        return Err(format!(
            "Git resolve ours failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(())
}

#[tauri::command]
pub fn cmd_resolve_theirs(state: State<AppState>, path: String) -> Result<(), String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", "--theirs", &path])
        .output()
        .map_err(|e| format!("Failed to execute git checkout --theirs {}: {}", path, e))?;

    if !output.status.success() {
         return Err(format!(
            "Git resolve theirs failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[tauri::command]
pub fn cmd_mark_resolved(state: State<AppState>, path: String) -> Result<(), String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["add", &path])
        .output()
        .map_err(|e| format!("Failed to execute git add {}: {}", path, e))?;

    if !output.status.success() {
         return Err(format!(
            "Git mark resolved failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(())
}

#[tauri::command]
pub fn cmd_write_file(state: State<AppState>, path: String, content: String) -> Result<(), String> {
    use std::fs;
    use std::path::Path;
    let repo_path = get_active_repo_path(&state)?;

    let full_path = Path::new(&repo_path).join(&path);

    if !full_path.starts_with(&repo_path) {
         return Err("Invalid path: cannot write outside of repository".to_string());
    }

    fs::write(&full_path, content)
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;

    Ok(())
}

#[tauri::command]
pub fn cmd_git_status(state: State<AppState>) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .arg("status")
        .output()
        .map_err(|e| format!("Failed to execute git status: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_pull(state: State<AppState>) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .arg("pull")
        .output()
        .map_err(|e| format!("Failed to execute git pull: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_push(state: State<AppState>) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .arg("push")
        .output()
        .map_err(|e| format!("Failed to execute git push: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_fetch(state: State<AppState>) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .arg("fetch")
        .output()
        .map_err(|e| format!("Failed to execute git fetch: {}", e))?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_commit(state: State<AppState>, message: String) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", &message])
        .output()
        .map_err(|e| format!("Failed to execute git commit: {}", e))?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_add_all(state: State<AppState>) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "."])
        .output()
        .map_err(|e| format!("Failed to execute git add .: {}", e))?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_checkout(state: State<AppState>, branch: String) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", &branch])
        .output()
        .map_err(|e| format!("Failed to execute git checkout {}: {}", branch, e))?;

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
pub fn cmd_git_branch_list(state: State<AppState>) -> Result<Vec<String>, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|e| format!("Failed to execute git branch: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout.lines().map(|s| s.trim().to_string()).collect())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub fn cmd_git_log(state: State<AppState>, limit: usize) -> Result<String, String> {
    use std::process::Command;
    let repo_path = get_active_repo_path(&state)?;

    let output = Command::new("git")
        .current_dir(&repo_path)
        .args(["log", &format!("-n{}", limit), "--oneline", "--graph", "--decorate"])
         .output()
        .map_err(|e| format!("Failed to execute git log: {}", e))?;

    if output.status.success() {
         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
