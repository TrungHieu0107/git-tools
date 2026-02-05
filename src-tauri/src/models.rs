use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandRequest {
    pub repo_path: String,
    pub subcommand: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub parsed: Option<GitParsedOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GitParsedOutput {
    Status { is_clean: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GitCommandErrorPayload {
    InvalidRepoPath { path: String },
    NotRepository { path: String },
    MergeConflict,
    CommandFailed { code: Option<i32>, stderr: String },
    Io { message: String },
}

impl From<crate::git_engine::GitCommandError> for GitCommandErrorPayload {
    fn from(error: crate::git_engine::GitCommandError) -> Self {
        match error {
            crate::git_engine::GitCommandError::InvalidRepoPath(path) => {
                GitCommandErrorPayload::InvalidRepoPath {
                    path: path.to_string_lossy().to_string(),
                }
            }
            crate::git_engine::GitCommandError::NotRepository(path) => {
                GitCommandErrorPayload::NotRepository {
                    path: path.to_string_lossy().to_string(),
                }
            }
            crate::git_engine::GitCommandError::MergeConflict => GitCommandErrorPayload::MergeConflict,
            crate::git_engine::GitCommandError::CommandFailed { code, stderr } => {
                GitCommandErrorPayload::CommandFailed { code, stderr }
            }
            crate::git_engine::GitCommandError::Io(error) => GitCommandErrorPayload::Io {
                message: error.to_string(),
            },
        }
    }
}
