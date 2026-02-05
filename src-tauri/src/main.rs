#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod git_engine;
mod models;

use crate::git_engine::GitCommandService;

fn main() {
    tauri::Builder::default()
        .manage(GitCommandService::new())
        .invoke_handler(tauri::generate_handler![commands::run_git])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
