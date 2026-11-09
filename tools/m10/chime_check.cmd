@echo off
rem Saves Mimi's two chimes, mixes the start chime into the M9 test clips, and checks the text is unchanged.
set ROOT=%~dp0..\..
set OUT=%ROOT%\proof\M10
set AUDIO=%ROOT%\test-audio-m10
if not exist "%OUT%" mkdir "%OUT%"
call "%ROOT%\tools\benchmark\build_env.cmd" >nul 2>nul
cd /d "%ROOT%\mimi\src-tauri"
set MIMI_CHIME_OUT=%OUT%
cargo test --release --lib -- --ignored save_chimes
if errorlevel 1 exit /b 1
python "%~dp0mix_chime.py" "%ROOT%\test-audio-m9" "%OUT%\chime-start.wav" "%AUDIO%"
cargo build --release --features test-tools --bin pipeline_check
if errorlevel 1 exit /b 1
set CHECK=C:\mimi-build\app\release\pipeline_check.exe
"%CHECK%" "%AUDIO%" es 1.0 0 "%OUT%\chime-parakeet.json" parakeet clip hint gpu
"%CHECK%" "%AUDIO%" es 1.0 0 "%OUT%\chime-turbo.json" whisper-turbo clip hint gpu
