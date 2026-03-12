
# Full Documentation
## Version: 1.0.0
## Last updated: 2026-03-13 – Initial split from PROJECT.md
## Project: GitHelper

## Usage Instructions
1.  **Launch** the application.
2.  **Add Repo**: Navigate to the "Repos" view and enter the absolute path to a local git repository.
3.  **Select Repo**: Click "Set Active" on the repository you wish to manage.
4.  **Commands**: Use the "Console" tab to run `status`, `fetch`, or arbitrary git commands.
5.  **Graph**: Switch to the "Graph" tab and click "Load Graph" to visualize history.
6.  **Conflicts**: If conflicts exist, the "Conflicts" button will highlight. Click it to resolve files using the 3-pane logic (Base, Ours, Theirs).

## Configuration
The application stores its configuration (list of known repositories, active repo ID) in a local JSON file managed by the OS-specific application data directory.
*   **Windows**: `%APPDATA%\git-tools\` (inferred)
*   **Logic**: `src-tauri/src/settings.rs` handles `load_settings` and `save_settings`.

## Operations Rules
**KHÔNG ĐƯỢC:**
- Tự tạo tính năng mới ngoài ghi chú.
- Refactor kiến trúc, đổi framework
- Xóa logic, đổi tên command/event
- Dùng `unwrap()` trong production path
- Frontend trực tiếp truy cập filesystem hoặc chạy git
- Dùng stderr = error hoặc exit_code != 0 = fatal
- Swallow errors hoặc silent failures
- Auto-load data khi không có active repo

**PHẢI:**
- Mọi git operation qua `std::process::Command` (Rust)
- Async execution, không block UI
- Return stdout + stderr + exit_code
- Validate repo path trước khi chạy lệnh
- Handle loading/empty/error states trong UI
- Mọi thay đổi phải minimal, localized, reversible
- Khi fix bug: xác định root cause → giải thích → minimum fix → verify

## Sync Workflow 
This application adheres to Living Documentation Sync rule. Before closing tasks, agent MUST sync changes to all affected documents in the registry.

## Changelog
- [2026-03-13] Initialized docs.md from README.md and PROJECT.md content.
