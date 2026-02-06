use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::git::types::{DiagnosticInfo, GitError, GitResponse, GitResult};

/// Timeout tiers for different command categories.
pub const TIMEOUT_LOCAL: u64 = 30;
pub const TIMEOUT_NETWORK: u64 = 120;
pub const TIMEOUT_QUICK: u64 = 15;

/// Unified async git executor.
///
/// Resolves the git binary once at startup and reuses the path for all
/// subsequent invocations. Every command runs fully async with timeout
/// protection and never blocks the Tauri IPC thread.
pub struct GitExecutor {
    git_binary: PathBuf,
}

impl GitExecutor {
    /// Create a new executor with a pre-resolved git binary path.
    pub fn new(git_binary: PathBuf) -> Self {
        Self { git_binary }
    }

    // ------------------------------------------------------------------
    // Git binary resolution (called once at startup)
    // ------------------------------------------------------------------

    /// Locate the git binary on the system.
    ///
    /// 1. Try the system PATH (`git --version`).
    /// 2. Probe well-known Windows install locations.
    /// 3. Return an error if nothing works.
    pub fn resolve_git_binary() -> Result<PathBuf, String> {
        // 1) Try PATH lookup (works if git is on the inherited env PATH)
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;

            if let Ok(output) = std::process::Command::new("git")
                .arg("--version")
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .output()
            {
                if output.status.success() {
                    println!("[GIT] Found git on PATH: {}", String::from_utf8_lossy(&output.stdout).trim());
                    return Ok(PathBuf::from("git"));
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = std::process::Command::new("git")
                .arg("--version")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .output()
            {
                if output.status.success() {
                    println!("[GIT] Found git on PATH: {}", String::from_utf8_lossy(&output.stdout).trim());
                    return Ok(PathBuf::from("git"));
                }
            }
        }

        // 2) Probe common Windows install paths
        #[cfg(target_os = "windows")]
        {
            let mut candidates: Vec<PathBuf> = vec![
                PathBuf::from(r"C:\Program Files\Git\cmd\git.exe"),
                PathBuf::from(r"C:\Program Files (x86)\Git\cmd\git.exe"),
            ];

            // %LOCALAPPDATA%\Programs\Git\cmd\git.exe  (user-scoped install)
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                candidates.push(PathBuf::from(format!(r"{}\Programs\Git\cmd\git.exe", local)));
            }

            // Scoop install: %USERPROFILE%\scoop\apps\git\current\cmd\git.exe
            if let Ok(profile) = std::env::var("USERPROFILE") {
                candidates.push(PathBuf::from(format!(
                    r"{}\scoop\apps\git\current\cmd\git.exe",
                    profile
                )));
            }

            for candidate in &candidates {
                if candidate.exists() {
                    println!("[GIT] Found git at: {}", candidate.display());
                    return Ok(candidate.clone());
                }
            }
        }

        Err(
            "Git binary not found. Please install Git and ensure it is available in your PATH \
             or installed in a standard location."
                .to_string(),
        )
    }

    // ------------------------------------------------------------------
    // Core execution
    // ------------------------------------------------------------------

    /// Run a git command asynchronously with timeout protection.
    ///
    /// * `repo_path` – working directory (must be a valid git repo).
    /// * `args`      – argument list, e.g. `["status", "--porcelain"]`.
    /// * `timeout_secs` – maximum wall-clock seconds before the process is killed.
    pub async fn run(
        &self,
        repo_path: &Path,
        args: &[String],
        timeout_secs: u64,
    ) -> GitResult<GitResponse> {
        // Validate repo path
        if !repo_path.exists() || !repo_path.is_dir() {
            return Err(GitError::InvalidRepoPath(
                repo_path.display().to_string(),
            ));
        }

        let start = Instant::now();
        let args_display = args.join(" ");
        println!(
            "[GIT START] git {} | cwd: {} | timeout: {}s",
            args_display,
            repo_path.display(),
            timeout_secs
        );

        let mut cmd = Command::new(&self.git_binary);
        cmd.current_dir(repo_path)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .env("LC_ALL", "C")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_PAGER", "")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Prevent console window flash on Windows
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        // Spawn the child process
        let child = cmd
            .spawn()
            .map_err(|e| GitError::IoError(format!("Failed to spawn git: {}", e)))?;

        // Await with timeout
        let output = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(GitError::IoError(format!(
                    "git process IO error: {}",
                    e
                )));
            }
            Err(_) => {
                // Timeout elapsed – the child is dropped which sends SIGKILL / TerminateProcess
                println!(
                    "[GIT TIMEOUT] git {} (after {}s)",
                    args_display, timeout_secs
                );
                return Err(GitError::Timeout(timeout_secs));
            }
        };

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        println!(
            "[GIT END] exit={} | {}ms | stdout={}b stderr={}b | git {}",
            exit_code,
            duration.as_millis(),
            stdout.len(),
            stderr.len(),
            args_display,
        );

        if output.status.success() {
            return Ok(GitResponse {
                stdout,
                stderr,
                exit_code,
                duration_ms: duration.as_millis() as u64,
            });
        }

        // Parse well-known error patterns
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepo(repo_path.display().to_string()));
        }
        if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
            return Err(GitError::MergeConflict);
        }

        Err(GitError::CommandError(format!(
            "git {} failed (exit {}): {}",
            args_display, exit_code, stderr
        )))
    }

    /// Run a git command that does not require a repository directory.
    /// Used for diagnostics (`git --version`).
    pub async fn run_bare(&self, args: &[String], timeout_secs: u64) -> GitResult<GitResponse> {
        let start = Instant::now();

        let mut cmd = Command::new(&self.git_binary);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd
            .spawn()
            .map_err(|e| GitError::IoError(format!("Failed to spawn git: {}", e)))?;

        let output = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(GitError::IoError(e.to_string())),
            Err(_) => return Err(GitError::Timeout(timeout_secs)),
        };

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(GitResponse {
            stdout,
            stderr,
            exit_code,
            duration_ms: duration.as_millis() as u64,
        })
    }

    // ------------------------------------------------------------------
    // Diagnostics
    // ------------------------------------------------------------------

    /// Collect diagnostic information about the git setup.
    pub async fn diagnostics(&self) -> DiagnosticInfo {
        let version = match self.run_bare(&["--version".to_string()], TIMEOUT_QUICK).await {
            Ok(r) => Some(r.stdout.trim().to_string()),
            Err(_) => None,
        };

        DiagnosticInfo {
            git_version: version,
            git_path: self.git_binary.display().to_string(),
            path_env: std::env::var("PATH").unwrap_or_default(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    /// Return a reference to the resolved binary path.
    #[allow(dead_code)]
    pub fn binary_path(&self) -> &Path {
        &self.git_binary
    }
}
