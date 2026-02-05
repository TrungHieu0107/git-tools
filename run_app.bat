@echo off
setlocal
title GitKraken Mini Launcher

echo [INFO] Checking environment...
where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] npm is not installed or not in PATH. Please install Node.js.
    pause
    exit /b 1
)

echo [INFO] Environment OK.

echo [INFO] Environment OK.

if not exist node_modules (
    echo [INFO] Installing dependencies... (This runs once)
    call npm install
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to install dependencies.
        pause
        exit /b 1
    )
)

echo [INFO] Starting Tauri Development Server...
echo [INFO] Please wait for the window to open.
call npm run tauri dev

if %errorlevel% neq 0 (
    echo [ERROR] Application crashed or failed to start.
    pause
)
endlocal
