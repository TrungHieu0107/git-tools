#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;
mod commands;
mod settings;

use git::GitExecutor;
use settings::AppState;
use tauri::Manager;

fn main() {
    // Resolve the git binary before starting the app.
    // This runs synchronously once at startup so we get a clear error if git
    // is not installed rather than failing silently on every command.
    let git_binary = GitExecutor::resolve_git_binary()
        .expect("Git not found. Please install Git and ensure it is in your PATH.");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let app_state = AppState::new(git_binary);
            let saved_settings = settings::load_settings(app.handle());
            *app_state.settings.lock().unwrap() = saved_settings;

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::run_git,
            commands::cmd_diagnostics,
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
            commands::cmd_get_commit_graph,
            commands::cmd_check_conflict_state,
            commands::cmd_get_git_branches,
            commands::cmd_get_current_branch,
            commands::cmd_git_switch_branch,
            commands::cmd_git_checkout_new_branch,
            commands::cmd_git_merge,
            commands::cmd_get_pending_commits_count,
            commands::cmd_get_status_files,
            commands::cmd_get_diff_file,
            commands::cmd_git_add,
            commands::cmd_git_unstage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
