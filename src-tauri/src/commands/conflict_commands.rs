use super::*;

pub async fn cmd_get_conflicts_impl(
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

pub async fn cmd_get_conflict_file_impl(
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<ConflictFile, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let repo = PathBuf::from(&r_path);

    let (base, ours, theirs) = tokio::try_join!(
        git_show_stage(&state.git, &repo, "1", &path),
        git_show_stage(&state.git, &repo, "2", &path),
        git_show_stage(&state.git, &repo, "3", &path),
    )?;

    Ok(ConflictFile { base, ours, theirs })
}

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

pub async fn cmd_resolve_ours_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let args: Vec<String> = vec!["checkout".into(), "--ours".into(), path];
    git_run_void_with_event(&app, &state, repo_path, args, TIMEOUT_LOCAL).await
}

pub async fn cmd_resolve_theirs_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let args: Vec<String> = vec!["checkout".into(), "--theirs".into(), path];
    git_run_void_with_event(&app, &state, repo_path, args, TIMEOUT_LOCAL).await
}

pub async fn cmd_mark_resolved_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    repo_path: Option<String>,
) -> Result<(), String> {
    let args: Vec<String> = vec!["add".into(), path];
    git_run_void_with_event(&app, &state, repo_path, args, TIMEOUT_LOCAL).await
}

pub async fn cmd_check_conflict_state_impl(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<bool, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let p = Path::new(&path);
    let git_dir = p.join(".git");

    let is_merging = git_dir.join("MERGE_HEAD").exists();
    let is_rebasing = git_dir.join("REBASE_HEAD").exists()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("rebase-apply").exists();
    let is_cherry_picking = git_dir.join("CHERRY_PICK_HEAD").exists();
    let is_reverting = git_dir.join("REVERT_HEAD").exists();

    if !is_merging && !is_rebasing && !is_cherry_picking && !is_reverting {
        return Ok(false);
    }

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
