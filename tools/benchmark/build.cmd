@echo off
rem Builds the speed test with Microsoft's C++ compiler settings loaded.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
for /d %%v in (C:\VulkanSDK\*) do set "VULKAN_SDK=%%v"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files\LLVM\bin;%VULKAN_SDK%\Bin;%PATH%"
rem A short output folder avoids the Windows 260-character path limit in Whisper's Vulkan build.
set "CARGO_TARGET_DIR=C:\mimi-build\bench"
cargo build --release --manifest-path "%~dp0Cargo.toml"
