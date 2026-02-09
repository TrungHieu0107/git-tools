use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[derive(Error, Debug, Serialize)]
#[serde(tag = "type", content = "message")]
#[allow(dead_code)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepo(String),

    #[error("Git command failed: {0}")]
    CommandError(String),

    #[error("Merge conflict detected")]
    MergeConflict,

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Git binary not found: {0}")]
    GitNotFound(String),

    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    #[error("Invalid repository path: {0}")]
    InvalidRepoPath(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> Self {
        GitError::IoError(err.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConflictFile {
    pub base: String,
    pub ours: String,
    pub theirs: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub git_version: Option<String>,
    pub git_path: String,
    pub path_env: String,
    pub platform: String,
}

pub type GitResult<T> = Result<T, GitError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum GitCommandType {
    Checkout,
    Merge,
    Commit,
    Pull,
    Push,
    Fetch,
    Branch,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub command_type: GitCommandType,
}
