use super::*;

pub fn cmd_get_settings_impl(state: State<AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

pub fn cmd_add_repo_impl(
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

    if !settings.open_repo_ids.contains(&id) {
        settings.open_repo_ids.push(id);
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_remove_repo_impl(
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

pub fn cmd_set_active_repo_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if !settings.repos.iter().any(|r| r.id == id) {
        return Err("Repository ID not found".to_string());
    }

    settings.active_repo_id = Some(id.clone());

    if !settings.open_repo_ids.contains(&id) {
        settings.open_repo_ids.push(id);
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_open_repo_impl(
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

pub fn cmd_close_repo_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if let Some(pos) = settings.open_repo_ids.iter().position(|r_id| *r_id == id) {
        settings.open_repo_ids.remove(pos);

        if settings.active_repo_id.as_ref() == Some(&id) {
            let next_active = if pos < settings.open_repo_ids.len() {
                Some(settings.open_repo_ids[pos].clone())
            } else if pos > 0 {
                Some(settings.open_repo_ids[pos - 1].clone())
            } else {
                None
            };
            settings.active_repo_id = next_active;
        }

        let _ = state.terminal.stop_session(&id);
        if let Some(repo) = settings.repos.iter().find(|r| r.id == id) {
            let _ = state.terminal.stop_session(&repo.path);
        }

        save_settings(&app_handle, &settings)?;
    }

    Ok(settings.clone())
}

pub fn cmd_get_active_repo_impl(state: State<AppState>) -> Result<Option<RepoEntry>, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    if let Some(id) = &settings.active_repo_id {
        Ok(settings.repos.iter().find(|r| r.id == *id).cloned())
    } else {
        Ok(None)
    }
}

pub fn cmd_set_excluded_files_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    exclusions: Vec<String>,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.excluded_files = exclusions;
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_repo_filter_impl(
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

pub fn cmd_set_gemini_api_token_impl(
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

pub fn cmd_set_gemini_model_impl(
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

pub fn cmd_set_open_router_api_token_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    token: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = token.trim().to_string();
    settings.open_router_api_token = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_open_router_model_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    model: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = model.trim().to_string();
    settings.open_router_model = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_active_ai_provider_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    provider: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = provider.trim().to_string();
    settings.active_ai_provider = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_global_commit_prompt_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    prompt: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = prompt.trim().to_string();
    settings.global_commit_prompt = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_repo_commit_prompt_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_path: String,
    prompt: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = prompt.trim().to_string();
    if trimmed.is_empty() {
        settings.repo_commit_prompts.remove(&repo_path);
    } else {
        settings.repo_commit_prompts.insert(repo_path, trimmed);
    }
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_repo_default_encoding_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_path: String,
    encoding: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    let trimmed = encoding.trim().to_string();
    let normalized_repo_path = repo_path.replace('\\', "/");
    
    // Empty string means clear the setting (default back to UTF-8 without override)
    if trimmed.is_empty() || trimmed == "utf-8" {
        settings.repo_default_encodings.remove(&normalized_repo_path);
    } else {
        settings.repo_default_encodings.insert(normalized_repo_path, trimmed);
    }
    
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_file_encoding_override_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    repo_path: String,
    file_path: String,
    encoding: String,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    
    // Normalize paths
    let normalized_repo = repo_path.replace('\\', "/");
    let normalized_file = file_path.replace('\\', "/");
    
    // Construct the exact path lookup key that encoding.rs globs or exact matches against.
    // For file_encodings, the app expects either a glob or exact path.
    // The previous implementation of `resolve_file_encoding` just checks `pattern.matches(&path_str)`
    // where path_str is the absolute path.
    
    let path = std::path::Path::new(&normalized_repo).join(&normalized_file);
    let path_str = path.to_string_lossy().replace('\\', "/");
    
    let trimmed = encoding.trim().to_string();

    if trimmed.is_empty() || trimmed.to_lowercase() == "default" {
        settings.file_encodings.remove(&path_str);
    } else {
        settings.file_encodings.insert(path_str, trimmed);
    }

    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}

pub fn cmd_set_show_ignored_files_impl(
    app_handle: AppHandle,
    state: State<AppState>,
    show: bool,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.show_ignored_files = show;
    save_settings(&app_handle, &settings)?;
    Ok(settings.clone())
}
