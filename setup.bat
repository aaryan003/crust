@echo off
setlocal enabledelayedexpansion

REM ============================================================
REM  CRUST one-liner setup script (Windows)
REM  Usage: setup.bat
REM ============================================================

echo.
echo ===================================
echo   CRUST Setup (Windows)
echo ===================================
echo.

REM ----------------------------------------------------------
REM  Check required tools
REM ----------------------------------------------------------

set "MISSING=0"

where docker >nul 2>nul
if %errorlevel% neq 0 (
    echo [X] 'docker' not found. Install Docker Desktop:
    echo     https://docs.docker.com/desktop/install/windows-install/
    set "MISSING=1"
)

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [X] 'cargo' not found. Install Rust via rustup:
    echo     https://rustup.rs
    set "MISSING=1"
)

where curl >nul 2>nul
if %errorlevel% neq 0 (
    echo [X] 'curl' not found. Windows 10+ ships with curl.
    echo     Ensure curl.exe is on your PATH.
    set "MISSING=1"
)

if "!MISSING!"=="1" (
    echo.
    echo Setup aborted — install the missing tools above and re-run.
    exit /b 1
)

echo [OK] All prerequisites found (docker, cargo, curl)

REM ----------------------------------------------------------
REM  If not inside the repo, clone it
REM ----------------------------------------------------------

if not exist "Cargo.toml" (
    echo.
    echo Cloning repository...
    git clone https://github.com/bhaumiksonii/crust.git
    cd crust
)

REM ----------------------------------------------------------
REM  Copy .env.example to .env if it doesn't exist
REM ----------------------------------------------------------

if not exist ".env" (
    if exist ".env.example" (
        echo.
        echo Copying .env.example to .env ...
        copy ".env.example" ".env" >nul
        echo [OK] Created .env from .env.example (edit it to customise secrets)
    )
)

REM ----------------------------------------------------------
REM  Start the server (PostgreSQL + crust-server)
REM ----------------------------------------------------------

echo.
echo Starting server via Docker Compose...
docker compose up -d --build
if %errorlevel% neq 0 (
    echo.
    echo [X] docker compose failed. Is Docker Desktop running?
    exit /b 1
)

REM ----------------------------------------------------------
REM  Wait for the server health endpoint
REM ----------------------------------------------------------

echo.
set /a "TRIES=0"
set /a "MAX_TRIES=30"

:healthloop
if !TRIES! geq !MAX_TRIES! (
    echo.
    echo [!] Server did not become healthy within %MAX_TRIES% seconds.
    echo     Check logs with: docker compose logs app
    goto :afterhealth
)

curl -sf http://localhost:8080/health >nul 2>nul
if %errorlevel% equ 0 (
    echo [OK] Server is healthy at http://localhost:8080
    goto :afterhealth
)

set /a "TRIES+=1"
<nul set /p "=."
timeout /t 1 /nobreak >nul
goto :healthloop

:afterhealth

REM ----------------------------------------------------------
REM  Install the CLI
REM ----------------------------------------------------------

echo.
echo Installing crust CLI (cargo install)...
cargo install --path crust-cli --force
if %errorlevel% neq 0 (
    echo.
    echo [X] cargo install failed. Check Rust toolchain and build errors above.
    exit /b 1
)

REM ----------------------------------------------------------
REM  Verify CLI installation
REM ----------------------------------------------------------

where crust >nul 2>nul
if %errorlevel% equ 0 (
    for /f "tokens=*" %%i in ('where crust') do set "CRUST_PATH=%%i"
) else (
    set "CRUST_PATH=crust (not found on PATH — check %%USERPROFILE%%\.cargo\bin)"
)

REM ----------------------------------------------------------
REM  Done!
REM ----------------------------------------------------------

echo.
echo ===================================
echo   Setup Complete!
echo ===================================
echo.
echo   Server : http://localhost:8080
echo   CLI    : !CRUST_PATH!
echo.
echo   Next steps:
echo     1. crust login http://localhost:8080
echo     2. crust init my-project
echo     3. cd my-project
echo     4. crust commit -m "first commit"
echo     5. crust push
echo.

<<<<<<< HEAD
endlocal
=======
endlocal
>>>>>>> upstream/main
