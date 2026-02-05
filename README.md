# GitHelper (git-kraken-mini)

A lightweight, high-performance Git GUI application built with **Tauri v2** and **Svelte 5**. Designed for speed and developer efficiency, it acts as a direct graphical wrapper around your system's `git` binary, offering a modern interface for common Git operations and conflict resolution.

## Overview

GitHelper (internally `git-kraken-mini` or `git-tools`) provides a responsive, dark-mode interface to manage multiple git repositories. It bridges the gap between the command line and heavy GUI clients by offering visualization tools (Graph, Conflicts) alongside a direct passthrough console for power users.

## Features

*   **Repository Management**:
    *   Add and manage multiple local repositories.
    *   Quickly switch between active repositories.
    *   Persists repository list and active state.
*   **Conflict Resolution**:
    *   Dedicated view for merge conflicts.
    *   Visual "Ours" vs "Theirs" comparison (derived from `git show`).
    *   One-click resolution (Checkout Ours/Theirs).
    *   Mark files as resolved (`git add`).
*   **Commit Graph Visualization**:
    *   Interactive commit history graph.
    *   Visualizes branching and merging topology.
    *   Customizable depth (number of commits).
*   **Integrated Command Center**:
    *   Execute raw `git` commands directly from the UI.
    *   View real-time `stdout` and `stderr` output.
    *   Exit code status indicators.
*   **Dark Mode UI**:
    *   Modern, "GitHub Dark" inspired aesthetics using TailwindCSS.

## Architecture

This project uses a hybrid architecture leveraging the strength of **Rust** for system interactions and **Web Technologies** for the User Interface.

### Technology Stack
*   **Frontend**: Svelte 5 (Next), TypeScript, TailwindCSS, Vite.
*   **Backend/Core**: Rust, Tauri v2.
*   **System Integration**: `tauri-plugin-shell` for safe subprocess execution.
*   **Data Persistence**: Local JSON file (managed by Rust `AppState`).

### Key Modules
*   **`src-tauri/src/main.rs`**: Application entry point. Initializes Tauri, loads settings, and sets up state management.
*   **`src-tauri/src/commands.rs`**: The core API layer. It validates requests and spawns `git` subprocesses using `std::process::Command`.
*   **`src-tauri/src/settings.rs`**: Handles persistence of the repository list to the local file system.
*   **`src/App.svelte`**: The main frontend orchestration layer, handling routing between Repos, Conflicts, and Console views.
*   **`src/lib/graph-layout.ts`**: Client-side logic for calculating commit graph topology positions and rendering.

### Data Flow
1.  **User Action**: User triggers an event (e.g., clicking "Load Graph").
2.  **IPC Call**: Svelte invokes a Tauri command (e.g., `run_git` or `cmd_get_conflicts`).
3.  **Rust Handler**: Backend receives the invoke message, validates input, and spawns a `git` subprocess in the active repository context.
4.  **Response**: Standard Output/Error from `git` is captured, parsed (if necessary), and returned to the frontend for rendering.

## Folder Structure

```
d:\learn\git-tools
├── src/                    # Frontend Source (SvelteKit/Vite)
│   ├── components/         # UI Components (RepoManager, Conflicts, Graph, etc.)
│   ├── lib/                # Shared logic (GitService, Graph algorithms)
│   ├── App.svelte          # Main entry component
│   └── main.ts             # Frontend entry point
├── src-tauri/              # Backend Source (Rust)
│   ├── src/
│   │   ├── commands.rs     # Exposes Rust functions to JS
│   │   ├── main.rs         # Tauri setup & entry point
│   │   └── settings.rs     # Configuration state management
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration & permissions
├── package.json            # Frontend dependencies & scripts
└── tsconfig.json           # TypeScript configuration
```

## Prerequisites & Setup

### Environment Requirements
*   **Git**: Must be installed and globally available in your system's `PATH`.
*   **Node.js**: Required for building and running the frontend (v18+ recommended).
*   **Rust**: Required for building the Tauri backend (latest stable).
*   **Visual Studio Build Tools** (Windows): C++ build tools required for Tauri's native bindings.

### Installation
1.  **Clone the repository:**
    ```bash
    git clone <repository_url>
    cd git-tools
    ```

2.  **Install Frontend Dependencies:**
    ```bash
    npm install
    ```

## Build & Run Instructions

### Development
To run the application in development mode (with hot-reloading for both Rust and Svelte):
```bash
npm run tauri dev
```
*   This command starts the Vite dev server.
*   It compiles the Rust backend.
*   Opens the application window.

### Production Build
To build a standalone executable (e.g., `.exe` on Windows):
```bash
npm run tauri build
```
*   The final artifact will be located in `src-tauri/target/release/bundle/`.

## Usage

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

## Limitations / Assumptions

*   **Inferred Behavior**: The application assumes that any command writing to `stderr` is an error, though Git sometimes uses `stderr` for progress indicators.
*   **Security**: The application executes shell commands. While restricted to the `git` binary, it runs with the permissions of the user.
*   **Missing Features**: Currently no support for SSH key management or credential helpers within the UI; relies on the system's global git configuration.
