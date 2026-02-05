#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;
mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![commands::run_git])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
