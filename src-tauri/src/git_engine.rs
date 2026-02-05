use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio::process::Command;

use crate::models::{GitCommandOutput, GitCommandRequest, GitParsedOutput};

#[derive(Debug, Error)]
pub enum GitCommandError {
    #[error("invalid repository path: {0}")]
    InvalidRepoPath(PathBuf),
    #[error("not a git repository: {0}")]
    NotRepository(PathBuf),
    #[error("merge conflict detected")]
    MergeConflict,
    #[error("git command failed with exit code {code:?}: {stderr}")]
    CommandFailed { code: Option<i32>, stderr: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Default)]
pub struct GitCommandService {
    git_binary: PathBuf,
}

impl GitCommandService {
    pub fn new() -> Self {
        Self {
            git_binary: PathBuf::from("git"),
        }
    }

    pub async fn run(
        &self,
        repo_path: &Path,
        subcommand: &[String],
    ) -> Result<GitCommandOutput, GitCommandError> {
        if !repo_path.exists() || !repo_path.is_dir() {
            return Err(GitCommandError::InvalidRepoPath(
                repo_path.to_path_buf(),
            ));
        }

        let output = Command::new(&self.git_binary)
            .current_dir(repo_path)
            .args(subcommand)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            if stderr.to_lowercase().contains("not a git repository") {
                return Err(GitCommandError::NotRepository(repo_path.to_path_buf()));
            }

            if stderr.contains("CONFLICT") {
                return Err(GitCommandError::MergeConflict);
            }

            return Err(GitCommandError::CommandFailed {
                code: output.status.code(),
                stderr,
            });
        }

        let parsed = Self::parse_stdout(subcommand, &stdout);

        Ok(GitCommandOutput {
            stdout,
            stderr,
            exit_code: output.status.code(),
            parsed,
        })
    }

    fn parse_stdout(subcommand: &[String], stdout: &str) -> Option<GitParsedOutput> {
        if subcommand.first().is_some_and(|cmd| cmd == "status") {
            let is_clean = stdout.contains("nothing to commit") || stdout.contains("working tree clean");
            return Some(GitParsedOutput::Status { is_clean });
        }

        None
    }

    pub async fn run_request(
        &self,
        request: GitCommandRequest,
    ) -> Result<GitCommandOutput, GitCommandError> {
        let repo_path = Path::new(&request.repo_path);
        self.run(repo_path, &request.subcommand).await
    }
}
