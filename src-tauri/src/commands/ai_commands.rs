use super::*;

const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";
const GEMINI_MAX_DIFF_CHARS: usize = 40_000;
const GEMINI_MAX_FILE_SUMMARY_CHARS: usize = 4_000;
const GEMINI_LIST_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_MODELS_PAGE_SIZE: &str = "1000";

const DEFAULT_OPEN_ROUTER_MODEL: &str = "google/gemini-2.5-flash";
const OPEN_ROUTER_CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const OPEN_ROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

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

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    #[serde(default)]
    data: Vec<OpenRouterModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelEntry {
    id: String,
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

fn build_gemini_prompt(
    file_summary: &str,
    diff_patch: &str,
    diff_was_truncated: bool,
    custom_prompt: Option<String>,
) -> String {
    let mut prompt = if let Some(p) = custom_prompt {
        if p.trim().is_empty() {
            get_default_expert_prompt()
        } else {
            p
        }
    } else {
        get_default_expert_prompt()
    };

    // Ensure double newline before context
    if !prompt.ends_with("\n\n") {
        if prompt.ends_with('\n') {
            prompt.push('\n');
        } else {
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str("Staged files (summary):\n");
    prompt.push_str(file_summary.trim());
    prompt.push_str("\n\nStaged diff details:\n");
    prompt.push_str(diff_patch.trim());

    if diff_was_truncated {
        prompt.push_str("\n\n[NOTE] The diff content was truncated due to its large size. Please rely on the file summary and the provided diff snippet to infer the overall logic.");
    }

    prompt
}

pub fn cmd_get_default_ai_prompt_impl() -> String {
    get_default_expert_prompt()
}

fn get_default_expert_prompt() -> String {
    String::from(
        r#"You are a Principal Software Engineer and a stickler for clean git history.
Task: Analyze the provided staged changes (diff) and generate a semantic git commit message that adheres strictly to the Conventional Commits specification.

Guidelines:

1. Structure:
   <type>(<scope>): <subject>

   [optional body]

   [optional footer]

2. Header (<type>(<scope>): <subject>):
   - <type>: Must be one of: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert.
   - <scope>: A noun describing the section of the codebase (e.g., auth, api, button, db). If the change is global, omit the scope.
   - <subject>:
     - Use the imperative, present tense ("add", "change", "fix", NOT "added", "changing").
     - No capitalization of the first letter (unless it's a proper noun).
     - No period (.) at the end.
     - Maximum 50 characters ideally, absolute limit 72.
     - Describe the "what" concisely.

3. Body:
   - Mandatory if the change is non-trivial.
   - Wrap lines at 72 characters.
   - Paragraph 1: Motivation. Explain *why* this change is necessary. What problem does it solve? (e.g., "The previous sorting algorithm caused latency in large datasets.")
   - Paragraph 2: Implementation. Describe *how* the problem was solved technically. (e.g., "Implemented QuickSort and memoization for the user list.")
   - Use bullet points (-) for listing multiple logical changes or specific file impacts.

4. Footer:
   - BREAKING CHANGE: If the change breaks backward compatibility, start the footer with "BREAKING CHANGE:" followed by a description of what broke and how to migrate.
   - References: Link to issue tracker IDs if applicable (e.g., "Closes #123", "Refs JIRA-456").

5. Analysis Strategy:
   - Identify the primary intent. Is it a feature? A fix? A refactor?
   - If the diff contains multiple unrelated changes, focus on the most significant one for the subject, and detail the others in the body.
   - Ignore formatting changes (whitespace) unless the commit is specifically about formatting.

Constraints:
- Output PLAIN TEXT only. No markdown formatting, no code fences (```).
- Do not include "Subject:", "Body:", or any meta-labels.
- Ensure exactly one empty line between Header and Body, and between Body and Footer."#,
    )
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
            "temperature": 0.3,
            "topP": 0.9,
            "maxOutputTokens": 1024
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

    Ok("Gemini did not return any commit message text.".to_string())
}

async fn call_open_router_api(token: &str, model: &str, prompt: &str) -> Result<String, String> {
    let payload = json!({
        "model": model,
        "messages": [
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.3,
        "top_p": 0.9,
    });

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_NETWORK))
        .build()
        .map_err(|e| format!("Failed to initialize OpenRouter client: {}", e))?;

    let response = client
        .post(OPEN_ROUTER_CHAT_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("HTTP-Referer", "https://github.com/TrungHieu0107/git-tools")
        .header("X-Title", "Git Tools")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call OpenRouter API: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read OpenRouter response: {}", e))?;

    if !status.is_success() {
        if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(error) = err_json.get("error") {
                if let Some(raw) = error.get("metadata").and_then(|m| m.get("raw")).and_then(|v| v.as_str()) {
                    return Err(format!("OpenRouter rate-limit or upstream error: {}", raw));
                }
                if let Some(msg) = error.get("message").and_then(|v| v.as_str()) {
                    return Err(format!("OpenRouter API error: {}", msg));
                }
            }
        }
        return Err(format!("OpenRouter API error ({}): {}", status, body));
    }

    let response_json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid OpenRouter response: {}", e))?;

    // Extract text from OpenAI-compatible response format
    if let Some(choices) = response_json.get("choices").and_then(|v| v.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|v| v.as_str()) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return Ok(trimmed.to_string());
                }
            }
        }
    }

    if let Some(message) = response_json
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
    {
        return Err(format!("OpenRouter API error: {}", message));
    }

    Ok("OpenRouter did not return any commit message text.".to_string())
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

pub async fn cmd_get_open_router_models_impl(
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
            .open_router_api_token
            .clone()
            .ok_or("OpenRouter API token is missing. Set it in Settings first.")?
    };

    if api_token.trim().is_empty() {
        return Err("OpenRouter API token is missing. Set it in Settings first.".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_NETWORK))
        .build()
        .map_err(|e| format!("Failed to initialize OpenRouter client: {}", e))?;

    let response = client
        .get(OPEN_ROUTER_MODELS_URL)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("HTTP-Referer", "https://github.com/TrungHieu0107/git-tools")
        .header("X-Title", "Git Tools")
        .send()
        .await
        .map_err(|e| format!("Failed to call OpenRouter API: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read OpenRouter response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "OpenRouter API error while listing models ({}): {}",
            status, body
        ));
    }

    let parsed: OpenRouterModelsResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid OpenRouter model list response: {}", e))?;

    let mut models: Vec<String> = parsed
        .data
        .into_iter()
        .map(|m| m.id.clone())
        .collect();

    if models.is_empty() {
        return Err("No OpenRouter models found.".to_string());
    }

    models.sort_unstable();
    Ok(models)
}

pub async fn cmd_generate_commit_message_impl(
    state: State<'_, AppState>,
    repo_path: Option<String>,
) -> Result<String, String> {
    let path = resolve_repo_path(&state, repo_path)?;

    let (
        gemini_token,
        gemini_model,
        or_token,
        or_model,
        provider,
        global_prompt,
        repo_prompt,
    ) = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        
        let gemini_token = settings.gemini_api_token.clone();
        let gemini_model = settings
            .gemini_model
            .clone()
            .unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string());
            
        let or_token = settings.open_router_api_token.clone();
        let or_model = settings
            .open_router_model
            .clone()
            .unwrap_or_else(|| DEFAULT_OPEN_ROUTER_MODEL.to_string());
            
        // default to "gemini" for backwards compatibility
        let provider = settings
            .active_ai_provider
            .clone()
            .unwrap_or_else(|| "gemini".to_string());

        let global_prompt = settings.global_commit_prompt.clone();
        let repo_prompt = settings.repo_commit_prompts.get(&path).cloned();

        (
            gemini_token,
            gemini_model,
            or_token,
            or_model,
            provider,
            global_prompt,
            repo_prompt,
        )
    };

    let target_prompt = repo_prompt.or(global_prompt);
    let commit_context = fetch_commit_context(&state, &path).await?;
    
    let prompt = build_gemini_prompt(
        &commit_context.file_summary_for_prompt,
        &commit_context.diff_patch_for_prompt,
        commit_context.diff_was_truncated,
        target_prompt,
    );

    let raw_response = if provider == "openrouter" {
        let token = or_token.ok_or("OpenRouter API token is missing. Set it in Settings first.")?;
        let active_model = if or_model.trim().is_empty() {
            DEFAULT_OPEN_ROUTER_MODEL.to_string()
        } else {
            or_model.trim().to_string()
        };
        call_open_router_api(&token, &active_model, &prompt).await?
    } else {
        let token = gemini_token.ok_or("Gemini API token is missing. Set it in Settings first.")?;
        let active_model = if gemini_model.trim().is_empty() {
            DEFAULT_GEMINI_MODEL.to_string()
        } else {
            gemini_model.trim().to_string()
        };
        call_gemini_api(&token, &active_model, &prompt).await?
    };

    let sanitized = sanitize_commit_message(&raw_response);
    let message = ensure_commit_message_has_body(&sanitized, &commit_context.staged_files);

    if message.trim().is_empty() {
        return Err("AI provider returned an empty commit message.".to_string());
    }

    Ok(message)
}

