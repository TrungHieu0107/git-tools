use std::path::PathBuf;
use tokio::process::Command;
use crate::git::types::{GitResponse, GitError, GitResult};

pub struct GitCommandService {
    repo_path: PathBuf,
}

impl GitCommandService {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub async fn run(&self, subcommands: Vec<String>) -> GitResult<GitResponse> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&subcommands)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if output.status.success() {
            Ok(GitResponse {
                stdout,
                stderr,
                exit_code,
            })
        } else {
            // Parse common git errors from stderr
            if stderr.contains("not a git repository") {
                return Err(GitError::NotARepo(self.repo_path.display().to_string()));
            }
            if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
                return Err(GitError::MergeConflict);
            }
            
            Err(GitError::CommandError(format!(
                "Command 'git {}' failed with code {}: {}",
                subcommands.join(" "),
                exit_code,
                stderr
            )))
        }
    }
}
