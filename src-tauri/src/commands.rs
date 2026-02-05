use crate::git::{GitCommandService, GitResponse, GitResult};
use std::path::PathBuf;

#[tauri::command]
pub async fn run_git(
    repo_path: String,
    subcommand: Vec<String>,
) -> GitResult<GitResponse> {
    let service = GitCommandService::new(PathBuf::from(repo_path));
    service.run(subcommand).await
}
