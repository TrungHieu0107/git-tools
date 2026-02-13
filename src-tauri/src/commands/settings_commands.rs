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
