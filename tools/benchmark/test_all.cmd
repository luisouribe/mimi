@echo off
rem Runs Mimi's code tests.
call "%~dp0build_env.cmd"
cd /d "%~dp0..\..\mimi\src-tauri"
cargo test --lib
