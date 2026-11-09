@echo off
rem Builds only the hidden test tool, then shows how each model writes the M4 test dictations.
set ROOT=%~dp0..\..
set AUDIO=%ROOT%\test-audio-m4
set OUT=%ROOT%\proof\M4
if not exist "%OUT%" mkdir "%OUT%"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0make_speech.ps1" -Cases "%~dp0cases.tsv" -Out "%AUDIO%"
call "%ROOT%\tools\benchmark\build_env.cmd"
cd /d "%ROOT%\mimi\src-tauri"
cargo build --release --features test-tools --bin pipeline_check
if errorlevel 1 exit /b 1
set CHECK=C:\mimi-build\app\release\pipeline_check.exe
"%CHECK%" "%AUDIO%" en,es 1.0 0 "%OUT%\raw-parakeet.json" parakeet clip hint gpu
"%CHECK%" "%AUDIO%" en,es 1.0 0 "%OUT%\raw-turbo.json" whisper-turbo clip hint gpu
"%CHECK%" "%AUDIO%" en,es 1.0 0 "%OUT%\raw-turbo-auto.json" whisper-turbo auto hint gpu
"%CHECK%" "%AUDIO%" en,es 1.0 0 "%OUT%\raw-small.json" whisper-small clip hint gpu
