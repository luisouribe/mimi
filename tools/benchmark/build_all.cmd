@echo off
rem Builds Mimi and its pipeline test tool.
call "%~dp0build_env.cmd"
cd /d "%~dp0..\..\mimi\src-tauri"
rem The speech library doesn't notice changed settings, so it's rebuilt only when asked.
if "%1"=="clean-whisper" cargo clean -p whisper-rs-sys --release
cd /d "%~dp0..\..\mimi"
call npm run tauri build -- --no-bundle
if errorlevel 1 exit /b 1
cd /d "%~dp0..\..\mimi\src-tauri"
cargo build --release --features test-tools --bin pipeline_check
