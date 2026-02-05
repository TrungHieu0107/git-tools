#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;
mod commands;
mod settings;

use settings::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_state = AppState::new();
            // Load settings on startup
            let saved_settings = settings::load_settings(app.handle());
            *app_state.settings.lock().unwrap() = saved_settings;
            
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::run_git,
            commands::cmd_get_conflicts,
            commands::cmd_get_conflict_file,
            commands::cmd_resolve_ours,
            commands::cmd_resolve_theirs,
            commands::cmd_mark_resolved,
            commands::cmd_write_file,
            commands::cmd_get_settings,
            commands::cmd_add_repo,
            commands::cmd_remove_repo,
            commands::cmd_set_active_repo,
            commands::cmd_get_active_repo,
            commands::cmd_git_status,
            commands::cmd_git_pull,
            commands::cmd_git_push,
            commands::cmd_git_fetch,
            commands::cmd_git_commit,
            commands::cmd_git_add_all,
            commands::cmd_git_checkout,
            commands::cmd_git_branch_list,
            commands::cmd_git_log,
            commands::cmd_check_conflict_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
