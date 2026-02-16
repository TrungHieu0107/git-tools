use super::*;
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitOperationState {
    pub is_merging: bool,
    pub is_rebasing: bool,
    pub is_cherry_picking: bool,
    pub is_reverting: bool,
    pub has_conflicts: bool,
    pub conflict_paths: Vec<String>,
    pub ours_commit: Option<String>,
    pub ours_branch: Option<String>,
    pub theirs_commit: Option<String>,
    pub theirs_branch: Option<String>,
}

fn is_unmerged_status(status: &str) -> bool {
    matches!(status, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU")
}

fn parse_status_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }

    let mut path = line[3..].trim().to_string();
    if path.starts_with('"') && path.ends_with('"') && path.len() >= 2 {
        path = path[1..path.len() - 1].to_string();
    }
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn collect_conflict_paths(porcelain_status: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = HashSet::<String>::new();

    for line in porcelain_status.lines() {
        if line.len() < 2 {
            continue;
        }

        let status = &line[0..2];
        if !is_unmerged_status(status) {
            continue;
        }

        let Some(path) = parse_status_path(line) else {
            continue;
        };

        if seen.insert(path.clone()) {
            paths.push(path);
        }
    }

    paths
}

fn detect_operation_flags(git_dir: &Path) -> (bool, bool, bool, bool) {
    let is_merging = git_dir.join("MERGE_HEAD").exists();
    let is_rebasing = git_dir.join("REBASE_HEAD").exists()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("rebase-apply").exists();
    let is_cherry_picking = git_dir.join("CHERRY_PICK_HEAD").exists();
    let is_reverting = git_dir.join("REVERT_HEAD").exists();
    (is_merging, is_rebasing, is_cherry_picking, is_reverting)
}

pub async fn cmd_get_conflicts_impl(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<Vec<String>, String> {
    let resp = git_run(&state, repo_path, &["status", "--porcelain"], TIMEOUT_LOCAL).await?;
    Ok(collect_conflict_paths(&resp.stdout))
}

pub async fn cmd_get_conflict_file_impl(
    state: State<'_, AppState>,
    path: String,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<ConflictFile, String> {
    let r_path = resolve_repo_path(&state, repo_path)?;
    let repo = PathBuf::from(&r_path);
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();

    let stages = git_list_file_stages(&state.git, &repo, &path).await?;
    let base = git_show_stage_if_present_bytes(&state.git, &repo, "1", &path, &stages, &settings, &encoding).await?;
    let ours = git_show_stage_if_present_bytes(&state.git, &repo, "2", &path, &stages, &settings, &encoding).await?;
    let theirs = git_show_stage_if_present_bytes(&state.git, &repo, "3", &path, &stages, &settings, &encoding).await?;

    Ok(ConflictFile { base, ours, theirs })
}

async fn git_list_file_stages(
    executor: &crate::git::GitExecutor,
    repo: &Path,
    file: &str,
) -> Result<HashSet<String>, String> {
    let args = vec![
        "ls-files".to_string(),
        "-u".to_string(),
        "--".to_string(),
        file.to_string(),
    ];
    let resp = executor
        .run(repo, &args, TIMEOUT_QUICK)
        .await
        .map_err(|e| format!("git ls-files -u -- {} failed: {}", file, e))?;

    let mut stages = HashSet::new();
    for line in resp.stdout.lines() {
        // format: <mode> <oid> <stage>\t<path>
        let mut parts = line.split_whitespace();
        let _mode = parts.next();
        let _oid = parts.next();
        let Some(stage) = parts.next() else {
            continue;
        };
        stages.insert(stage.to_string());
    }

    Ok(stages)
}

async fn git_show_stage_if_present_bytes(
    executor: &crate::git::GitExecutor,
    repo: &Path,
    stage: &str,
    file: &str,
    stages: &HashSet<String>,
    settings: &crate::settings::AppSettings,
    encoding: &Option<String>,
) -> Result<String, String> {
    if !stages.contains(stage) {
        return Ok(String::new());
    }
    git_show_stage_bytes(executor, repo, stage, file, settings, encoding).await
}

async fn git_show_stage_bytes(
    executor: &crate::git::GitExecutor,
    repo: &Path,
    stage: &str,
    file: &str,
    settings: &crate::settings::AppSettings,
    encoding: &Option<String>,
) -> Result<String, String> {
    let arg = format!(":{}:{}", stage, file);
    let args = vec!["show".to_string(), arg];
    let resp = executor
        .run_with_output_bytes(repo, &args, TIMEOUT_QUICK)
        .await
        .map_err(|e| format!("git show :{}:{} failed: {}", stage, file, e))?;
    Ok(crate::git::encoding::decode_bytes(
        &resp.stdout,
        Path::new(file),
        settings,
        encoding.clone(),
    ))
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

    let (is_merging, is_rebasing, is_cherry_picking, is_reverting) =
        detect_operation_flags(&git_dir);

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

    Ok(!collect_conflict_paths(&resp.stdout).is_empty())
}

fn read_git_file(git_dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(git_dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_merge_branch_from_msg(git_dir: &Path) -> Option<String> {
    let msg = read_git_file(git_dir, "MERGE_MSG")?;
    // Typical: "Merge branch 'branchname'" or "Merge branch 'branchname' into ..."
    if let Some(rest) = msg.strip_prefix("Merge branch '") {
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    // "Merge remote-tracking branch 'origin/branchname'"
    if let Some(rest) = msg.strip_prefix("Merge remote-tracking branch '") {
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

async fn resolve_conflict_metadata(
    state: &State<'_, AppState>,
    repo_path: &str,
    git_dir: &Path,
    is_merging: bool,
    is_rebasing: bool,
    is_cherry_picking: bool,
    is_reverting: bool,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let has_operation = is_merging || is_rebasing || is_cherry_picking || is_reverting;
    if !has_operation {
        return (None, None, None, None);
    }

    // ours commit: git rev-parse --short HEAD
    let ours_commit = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--short", "HEAD"], TIMEOUT_QUICK)
        .await
        .ok()
        .map(|r| r.stdout.trim().to_string())
        .filter(|s| !s.is_empty());

    // ours branch: git rev-parse --abbrev-ref HEAD
    let ours_branch = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--abbrev-ref", "HEAD"], TIMEOUT_QUICK)
        .await
        .ok()
        .map(|r| r.stdout.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD");

    let mut theirs_commit: Option<String> = None;
    let mut theirs_branch: Option<String> = None;

    if is_merging {
        if let Some(hash) = read_git_file(git_dir, "MERGE_HEAD") {
            theirs_commit = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--short", &hash], TIMEOUT_QUICK)
                .await
                .ok()
                .map(|r| r.stdout.trim().to_string())
                .filter(|s| !s.is_empty());
        }
        theirs_branch = parse_merge_branch_from_msg(git_dir);
    } else if is_rebasing {
        if let Some(hash) = read_git_file(git_dir, "REBASE_HEAD") {
            theirs_commit = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--short", &hash], TIMEOUT_QUICK)
                .await
                .ok()
                .map(|r| r.stdout.trim().to_string())
                .filter(|s| !s.is_empty());
        }
        theirs_branch = read_git_file(git_dir, "rebase-merge/head-name")
            .and_then(|s| s.strip_prefix("refs/heads/").map(|b| b.to_string()));
    } else if is_cherry_picking {
        if let Some(hash) = read_git_file(git_dir, "CHERRY_PICK_HEAD") {
            theirs_commit = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--short", &hash], TIMEOUT_QUICK)
                .await
                .ok()
                .map(|r| r.stdout.trim().to_string())
                .filter(|s| !s.is_empty());
        }
    } else if is_reverting {
        if let Some(hash) = read_git_file(git_dir, "REVERT_HEAD") {
            theirs_commit = git_run(state, Some(repo_path.to_string()), &["rev-parse", "--short", &hash], TIMEOUT_QUICK)
                .await
                .ok()
                .map(|r| r.stdout.trim().to_string())
                .filter(|s| !s.is_empty());
        }
    }

    (ours_commit, ours_branch, theirs_commit, theirs_branch)
}

pub async fn cmd_get_operation_state_impl(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<GitOperationState, String> {
    let path = resolve_repo_path(&state, repo_path)?;
    let p = Path::new(&path);
    let git_dir = p.join(".git");

    let (is_merging, is_rebasing, is_cherry_picking, is_reverting) =
        detect_operation_flags(&git_dir);

    let resp = git_run(
        &state,
        Some(path.clone()),
        &["status", "--porcelain"],
        TIMEOUT_LOCAL,
    )
    .await?;
    let conflict_paths = collect_conflict_paths(&resp.stdout);
    let has_conflicts = !conflict_paths.is_empty();

    let (ours_commit, ours_branch, theirs_commit, theirs_branch) =
        resolve_conflict_metadata(&state, &path, &git_dir, is_merging, is_rebasing, is_cherry_picking, is_reverting).await;

    Ok(GitOperationState {
        is_merging,
        is_rebasing,
        is_cherry_picking,
        is_reverting,
        has_conflicts,
        conflict_paths,
        ours_commit,
        ours_branch,
        theirs_commit,
        theirs_branch,
    })
}
