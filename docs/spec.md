
# Technical Specification
## Version: 1.0.0
## Last updated: 2026-03-13 – Initial split from PROJECT.md
## Project: GitHelper

## Requirements
**GitHelper** (tên nội bộ: `git-kraken-mini` hoặc `git-tools`) là một ứng dụng desktop GUI cho Git, tương tự GitKraken nhưng nhẹ hơn. Ứng dụng tập trung vào **tốc độ**, **tiêu thụ ít bộ nhớ**, và **hành vi có thể dự đoán được**.

### Nguyên tắc cốt lõi
- **Git chỉ được truy cập qua CLI hệ thống** (`git` binary) — KHÔNG dùng libgit2, KHÔNG dùng API
- **Không background daemon** — mọi thao tác Git được thực thi on-demand
- **Frontend không trực tiếp truy cập filesystem hoặc chạy git** — mọi thứ thông qua Tauri IPC
- **Mọi lỗi phải visible và traceable** — không swallow error, không silent failure

## Technology Stack

| Layer | Công nghệ | Phiên bản |
|-------|-----------|-----------|
| **Frontend Framework** | Svelte 5 (Next) + TypeScript | ^5.0.0-next.1 |
| **Styling** | TailwindCSS | ^3.4.1 |
| **Bundler** | Vite | ^5.1.0 |
| **Backend/Desktop** | Rust + Tauri v2 | tauri 2.x |
| **IPC/Shell** | tauri-plugin-shell, tauri-plugin-dialog | ^2.0.0 |
| **Async Runtime** | Tokio (process, rt-multi-thread, time) | 1.x |
| **Git Execution** | `std::process::Command` (sync) + `tokio::process::Command` (async) | - |
| **Serialization** | Serde + serde_json | 1.x |
| **HTTP Client** | reqwest (for Gemini AI API) | 0.12 |
| **Encoding** | encoding_rs | 0.8 |
| **Error Handling** | thiserror | 1.x |
| **UUID** | uuid (v4) | 1.x |

## Limitations & Constraints
*   **Inferred Behavior**: The application assumes that any command writing to `stderr` is an error, though Git sometimes uses `stderr` for progress indicators.
*   **Security**: The application executes shell commands. While restricted to the `git` binary, it runs with the permissions of the user.
*   **Missing Features**: Currently no support for SSH key management or credential helpers within the UI; relies on the system's global git configuration.

## Changelog
- [2026-03-13] Initialized spec.md from PROJECT.md
