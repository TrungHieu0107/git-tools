@echo off
setlocal

:: Switch to the script's directory to ensure relative paths work
cd /d "%~dp0"

echo ========================================
echo  GitHelper - One Click Build (.exe)
echo ========================================
echo.

:: 1. Check for Node.js
where node >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Node.js is not found in PATH.
    echo Please install Node.js v18+ from https://nodejs.org/
    pause
    exit /b 1
)

:: 2. Check for npm
where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] npm is not found in PATH.
    pause
    exit /b 1
)

:: 3. Check for Rust (cargo)
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust 'cargo' is not found in PATH.
    echo Please install Rust from https://rustup.rs/
    pause
    exit /b 1
)

echo [INFO] Environment verified (Node, npm, Cargo found).
echo.

:: 4. Install npm dependencies if node_modules is missing
if not exist "node_modules\" (
    echo [INFO] 'node_modules' not found. Installing dependencies...
    call npm install
    if errorlevel 1 (
        echo [ERROR] 'npm install' failed.
        pause
        exit /b 1
    )
) else (
    echo [INFO] Dependencies found. Skipping install.
)

echo.
echo [INFO] Starting Tauri build (npm run tauri build)...
echo.

:: 5. Run the build command
:: IMPORTANT: Use 'call' because npm is a cmd script on Windows
call npm run tauri build
if errorlevel 1 (
    echo.
    echo [ERROR] Build process failed.
    echo Check the error messages above.
    pause
    exit /b 1
)

echo.
echo ========================================
echo  BUILD SUCCESS!
echo ========================================
echo.
echo Output location:
echo %~dp0src-tauri\target\release\bundle\
echo.
pause
