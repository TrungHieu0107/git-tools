
# Setup and Environment
## Version: 1.0.0
## Last updated: 2026-03-13 – Initial creation from README and PROJECT.md
## Project: GitHelper

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
*   Note: Build artifacts are compiled natively by Tauri.

## Changelog
- [2026-03-13] Extracted setup instructions into setup.md
