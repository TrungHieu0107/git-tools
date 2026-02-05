use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Error, Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepo(String),
    
    #[error("Git command failed: {0}")]
    CommandError(String),
    
    #[error("Merge conflict detected")]
    MergeConflict,
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> Self {
        GitError::IoError(err.to_string())
    }
}

pub type GitResult<T> = Result<T, GitError>;
