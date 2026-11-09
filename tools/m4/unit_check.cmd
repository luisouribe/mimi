@echo off
rem Runs the cleanup rules' tests, then checks them against every saved M3 recognition.
set ROOT=%~dp0..\..
call "%ROOT%\tools\benchmark\build_env.cmd"
cd /d "%ROOT%\mimi\src-tauri"
cargo test --release --lib tidy
if errorlevel 1 exit /b 1
cargo build --release --features test-tools --bin tidy_check
if errorlevel 1 exit /b 1
set P=%ROOT%\proof\M3
C:\mimi-build\app\release\tidy_check.exe clips "%P%\parakeet.json" "%P%\turbo-hint.json" "%P%\small-hint.json" "%P%\turbo-no-hint.json" "%P%\turbo-auto-language.json"
