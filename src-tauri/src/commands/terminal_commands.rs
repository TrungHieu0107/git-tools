use super::*;

pub async fn cmd_terminal_start_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    state.terminal.start_session(app, repo_path)
}

pub async fn cmd_terminal_write_impl(
    state: State<'_, AppState>,
    repo_path: String,
    input: String,
) -> Result<(), String> {
    state.terminal.write_input(&repo_path, &input)
}

pub async fn cmd_terminal_stop_impl(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<(), String> {
    state.terminal.stop_session(&repo_path)
}
