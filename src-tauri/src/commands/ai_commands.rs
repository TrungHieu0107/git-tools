use super::*;

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

struct CommitContext {
    staged_files: String,
    file_summary_for_prompt: String,
    diff_patch_for_prompt: String,
    diff_was_truncated: bool,
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

async fn fetch_commit_context(
    state: &State<'_, AppState>,
    repo_path: &str,
) -> Result<CommitContext, String> {
    let staged_files_args: Vec<String> =
        vec!["diff".into(), "--cached".into(), "--name-status".into()];
    let staged_files_resp = state
        .git
        .run(Path::new(repo_path), &staged_files_args, TIMEOUT_QUICK)
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
        .run(Path::new(repo_path), &staged_diff_args, TIMEOUT_LOCAL)
        .await
        .map_err(|e| e.to_string())?;

    let (file_summary_for_prompt, _) =
        truncate_for_prompt(&staged_files, GEMINI_MAX_FILE_SUMMARY_CHARS);
    let (diff_patch_for_prompt, diff_was_truncated) =
        truncate_for_prompt(&staged_diff_resp.stdout, GEMINI_MAX_DIFF_CHARS);

    Ok(CommitContext {
        staged_files,
        file_summary_for_prompt,
        diff_patch_for_prompt,
        diff_was_truncated,
    })
}

fn build_gemini_prompt(file_summary: &str, diff_patch: &str, diff_was_truncated: bool) -> String {
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
    prompt.push_str(file_summary.trim());
    prompt.push_str("\n\nStaged diff:\n");
    prompt.push_str(diff_patch.trim());

    if diff_was_truncated {
        prompt.push_str("\n\n[NOTE] Diff content was truncated due to size.");
    }

    prompt
}

async fn call_gemini_api(token: &str, model: &str, prompt: &str) -> Result<String, String> {
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

    if let Some(text) = extract_gemini_text(&response_json) {
        return Ok(text);
    }

    if let Some(message) = response_json
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
    {
        return Err(format!("Gemini API error: {}", message));
    }

    Err("Gemini did not return any commit message text.".to_string())
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

pub async fn cmd_get_gemini_models_impl(
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

pub async fn cmd_generate_commit_message_impl(
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

    let commit_context = fetch_commit_context(&state, &path).await?;
    let prompt = build_gemini_prompt(
        &commit_context.file_summary_for_prompt,
        &commit_context.diff_patch_for_prompt,
        commit_context.diff_was_truncated,
    );
    let raw_response = call_gemini_api(&token, &model, &prompt).await?;
    let sanitized = sanitize_commit_message(&raw_response);
    let message = ensure_commit_message_has_body(&sanitized, &commit_context.staged_files);

    if message.trim().is_empty() {
        return Err("Gemini returned an empty commit message.".to_string());
    }

    Ok(message)
}
