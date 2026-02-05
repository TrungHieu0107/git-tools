Act as a Principal Software Architect and Senior Rust Developer specializing in Tauri (v2) applications.

**Project Goal:**
I want to build a high-performance, lightweight, and extensible desktop Git GUI (similar to GitKraken but minimal) that compiles to a single native .exe.
The application will function as a wrapper around the local `git` CLI, parsing stdout/stderr to visualize the repository state.

**Technical Stack Constraints:**
1.  **Core Framework:** Tauri v2 (latest stable).
2.  **Backend (Rust):** Must use `tokio` for asynchronous command execution to prevent blocking the UI thread. Use `anyhow` or `thiserror` for robust error handling.
3.  **Frontend:** Svelte 5 + TypeScript + TailwindCSS. (Chosen for maximum performance and minimal bundle size).
4.  **Inter-Process Communication (IPC):** Use Tauri Commands with strict serialization (Serde) for type-safe data exchange.

**Your Task:**
Please provide a production-ready scaffolding for this project. Break down your response into the following steps:

**Step 1: Application Architecture Strategy**
Briefly explain how you will structure the Rust backend to handle Git commands securely and scalably. Specifically, how to decouple the "Command Runner" logic from the Tauri command handlers.

**Step 2: Project Structure**
Outline the folder structure, specifically distinguishing between the Rust core logic (domain layer) and the UI layer.

**Step 3: Core Implementation (The "Git Engine")**
Write the Rust code for a `GitCommandService` struct. This service must:
* Take a repository path and a subcommand (e.g., `["status"]`, `["pull"]`) as input.
* Execute the command asynchronously using `tokio::process::Command`.
* Return a structured Result (Ok/Err) that separates `stdout` parsing from distinct logic errors (e.g., "Not a git repo", "Merge conflict").

**Step 4: Tauri Command Exposure**
Show how to expose this service to the Frontend via a Tauri command (e.g., `#[tauri::command] fn run_git(...)`).

**Step 5: Frontend Integration Example**
Provide a TypeScript interface that mirrors the Rust return type and a generic Svelte function to invoke the command.

**Focus on clean code principles, SOLID patterns, and performance.**