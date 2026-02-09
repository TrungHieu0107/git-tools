use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct TerminalSession {
    process: Child,
    stdin: std::process::ChildStdin,
}

#[derive(Clone)]
pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_session(&self, app: AppHandle, repo_path: String) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        
        if sessions.contains_key(&repo_path) {
            return Ok(());
        }

        let mut child = Command::new("powershell")
            .arg("-NoLogo")
            .arg("-NoExit")
            .arg("-Command")
            .arg("-") // Read from stdin
            .current_dir(&repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn powershell: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

        // Spawn threads to read stdout/stderr
        let app_clone = app.clone();
        let repo_path_clone = repo_path.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let _ = app_clone.emit("terminal-output", serde_json::json!({
                            "repoPath": repo_path_clone,
                            "type": "stdout",
                            "data": l
                        }));
                    }
                    Err(_) => break,
                }
            }
        });

        let app_clone = app.clone();
        let repo_path_clone = repo_path.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let _ = app_clone.emit("terminal-output", serde_json::json!({
                            "repoPath": repo_path_clone,
                            "type": "stderr",
                            "data": l
                        }));
                    }
                    Err(_) => break,
                }
            }
        });

        sessions.insert(repo_path, TerminalSession {
            process: child,
            stdin,
        });

        Ok(())
    }

    pub fn write_input(&self, repo_path: &str, input: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(session) = sessions.get_mut(repo_path) {
            writeln!(session.stdin, "{}", input).map_err(|e| e.to_string())?;
            return Ok(());
        }
        Err("Session not found".to_string())
    }

    pub fn stop_session(&self, repo_path: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(mut session) = sessions.remove(repo_path) {
            let _ = session.process.kill();
        }
        Ok(())
    }
}
