use tauri::State;

use crate::{
    git_engine::GitCommandService,
    models::{GitCommandErrorPayload, GitCommandOutput, GitCommandRequest},
};

#[tauri::command]
pub async fn run_git(
    state: State<'_, GitCommandService>,
    request: GitCommandRequest,
) -> Result<GitCommandOutput, GitCommandErrorPayload> {
    state.run_request(request).await.map_err(Into::into)
}
