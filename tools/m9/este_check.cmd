@echo off
rem Makes Spanish test dictations with "este" said as a hesitation and as "this", then shows how each model writes and cleans them.
set ROOT=%~dp0..\..
set AUDIO=%ROOT%\test-audio-m9
set OUT=%ROOT%\proof\M9
if not exist "%OUT%" mkdir "%OUT%"
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\tools\m4\make_speech.ps1" -Cases "%~dp0cases.tsv" -Out "%AUDIO%"
call "%ROOT%\tools\benchmark\build_env.cmd" >nul 2>nul
cd /d "%ROOT%\mimi\src-tauri"
cargo build --release --features test-tools --bin pipeline_check
if errorlevel 1 exit /b 1
set CHECK=C:\mimi-build\app\release\pipeline_check.exe
"%CHECK%" "%AUDIO%" es 1.0 0 "%OUT%\este-parakeet.json" parakeet clip hint gpu
"%CHECK%" "%AUDIO%" es 1.0 0 "%OUT%\este-turbo.json" whisper-turbo clip hint gpu
