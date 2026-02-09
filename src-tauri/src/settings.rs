use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::Manager;

use crate::git::GitExecutor;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoEntry {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppSettings {
    pub repos: Vec<RepoEntry>,
    pub active_repo_id: Option<String>,
    #[serde(default)]
    pub open_repo_ids: Vec<String>,
    #[serde(default)]
    pub excluded_files: Vec<String>,
    #[serde(default)]
    pub repo_filters: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub file_encodings: std::collections::HashMap<String, String>,
}

use crate::terminal::TerminalManager;

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub git: GitExecutor,
    pub terminal: TerminalManager,
}

impl AppState {
    pub fn new(git_binary: PathBuf) -> Self {
        Self {
            settings: Mutex::new(AppSettings::default()),
            git: GitExecutor::new(git_binary),
            terminal: TerminalManager::new(),
        }
    }
}

pub fn get_settings_path(app_handle: &AppHandle) -> PathBuf {
    app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("settings.json")
}

pub fn load_settings(app_handle: &AppHandle) -> AppSettings {
    let path = get_settings_path(app_handle);
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppSettings::default()
    }
}

pub fn save_settings(app_handle: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path(app_handle);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
