use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageLineSelection {
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedPatchLineKind {
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

fn parse_diff_header(lines: &[&str]) -> Result<(Vec<String>, usize), String> {
    let mut header_lines: Vec<String> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("@@") {
            break;
        }
        if line.starts_with("diff --git ") && !header_lines.is_empty() {
            break;
        }
        header_lines.push(line.to_string());
        index += 1;
    }

    if header_lines.is_empty() {
        return Err("Unable to parse diff header".to_string());
    }

    Ok((header_lines, index))
}

fn parse_hunk_lines(lines: &[&str], start: usize) -> Result<(ParsedPatchHunk, usize), String> {
    let header = lines
        .get(start)
        .ok_or("Unexpected end of diff while parsing hunk header".to_string())?;
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!("Invalid hunk header '{}'", header));
    }

    let (old_start, _) = parse_hunk_range(parts[1], '-')?;
    let (new_start, _) = parse_hunk_range(parts[2], '+')?;
    let mut old_cursor = old_start;
    let mut new_cursor = new_start;

    let mut parsed_lines = Vec::new();
    let mut index = start + 1;

    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("@@") || line.starts_with("diff --git ") {
            break;
        }
        if line.starts_with("\\ No newline at end of file") {
            index += 1;
            continue;
        }

        if let Some(content) = line.strip_prefix('+') {
            parsed_lines.push(ParsedPatchLine {
                kind: ParsedPatchLineKind::Add,
                content: content.to_string(),
                old_line: None,
                new_line: Some(new_cursor),
                old_anchor: old_cursor,
                new_anchor: new_cursor,
            });
            new_cursor += 1;
            index += 1;
            continue;
        }

        if let Some(content) = line.strip_prefix('-') {
            parsed_lines.push(ParsedPatchLine {
                kind: ParsedPatchLineKind::Remove,
                content: content.to_string(),
                old_line: Some(old_cursor),
                new_line: None,
                old_anchor: old_cursor,
                new_anchor: new_cursor,
            });
            old_cursor += 1;
            index += 1;
            continue;
        }

        if line.starts_with(' ') {
            old_cursor += 1;
            new_cursor += 1;
        }

        index += 1;
    }

    Ok((
        ParsedPatchHunk {
            lines: parsed_lines,
        },
        index,
    ))
}

fn build_parsed_patch(
    header_lines: Vec<String>,
    hunks: Vec<ParsedPatchHunk>,
) -> Result<ParsedUnstagedPatch, String> {
    if hunks.is_empty() {
        return Err("No unstaged diff hunks available for selected file".to_string());
    }

    Ok(ParsedUnstagedPatch {
        header_lines,
        hunks,
    })
}

fn parse_unstaged_zero_context_diff(diff_output: &str) -> Result<ParsedUnstagedPatch, String> {
    let lines: Vec<&str> = diff_output.lines().collect();
    let (header_lines, mut index) = parse_diff_header(&lines)?;

    let mut hunks = Vec::new();
    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("diff --git ") && !hunks.is_empty() {
            break;
        }
        if !line.starts_with("@@") {
            index += 1;
            continue;
        }

        let (hunk, next_index) = parse_hunk_lines(&lines, index)?;
        hunks.push(hunk);
        index = next_index;
    }

    build_parsed_patch(header_lines, hunks)
}

fn lookup_line_in_patch<'a>(
    patch: &'a ParsedUnstagedPatch,
    line_number: u32,
    line_type: ParsedPatchLineKind,
) -> Result<(usize, &'a ParsedPatchLine), String> {
    let (line_kind, line_label) = match line_type {
        ParsedPatchLineKind::Add => ("added", "new"),
        ParsedPatchLineKind::Remove => ("removed", "old"),
    };

    for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
        for line in &hunk.lines {
            if line.kind != line_type {
                continue;
            }
            let matches_line = match line_type {
                ParsedPatchLineKind::Add => line.new_line == Some(line_number),
                ParsedPatchLineKind::Remove => line.old_line == Some(line_number),
            };
            if matches_line {
                return Ok((hunk_index, line));
            }
        }
    }

    Err(format!(
        "Unable to find {} line {} in unstaged diff ({})",
        line_kind, line_number, line_label
    ))
}

fn build_stage_line_patch(
    patch: &ParsedUnstagedPatch,
    selection: &StageLineSelection,
) -> Result<String, String> {
    let mut patch_lines = patch.header_lines.clone();

    match (selection.old_line_number, selection.new_line_number) {
        (Some(old_line_number), Some(new_line_number)) => {
            let (remove_hunk_index, remove_line) =
                lookup_line_in_patch(patch, old_line_number, ParsedPatchLineKind::Remove)?;
            let (add_hunk_index, add_line) =
                lookup_line_in_patch(patch, new_line_number, ParsedPatchLineKind::Add)?;

            if remove_hunk_index != add_hunk_index {
                return Err("Selected modified line pair is in different hunks".to_string());
            }

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
            let (_, remove_line) =
                lookup_line_in_patch(patch, old_line_number, ParsedPatchLineKind::Remove)?;
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
            let (_, add_line) =
                lookup_line_in_patch(patch, new_line_number, ParsedPatchLineKind::Add)?;
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

pub async fn cmd_get_diff_file_impl(
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
        .run_with_output_bytes(Path::new(&path), &args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let content = crate::git::encoding::decode_bytes(
        &resp.stdout,
        Path::new(&file_path),
        &settings,
        encoding,
    );

    Ok(content)
}

pub async fn cmd_get_file_base_content_impl(
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

pub async fn cmd_get_file_modified_content_impl(
    state: State<'_, AppState>,
    file_path: String,
    staged: bool,
    encoding: Option<String>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    if staged {
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

pub async fn cmd_git_stage_line_impl(
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

    emit_git_change_event(&app)?;
    Ok(())
}

pub async fn cmd_git_unstage_line_impl(
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

    emit_git_change_event(&app)?;
    Ok(())
}
