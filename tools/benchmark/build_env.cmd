@echo off
rem Shared build settings for Mimi.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
for /d %%v in (C:\VulkanSDK\*) do set "VULKAN_SDK=%%v"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files\LLVM\bin;%VULKAN_SDK%\Bin;%PATH%"
rem Whisper's C++ code is built without optimization or fast processor instructions by default,
rem which makes it extremely slow without a graphics card. These settings turn both on.
rem AVX2 has been in nearly every processor since 2013.
set "GGML_NATIVE=OFF"
set "GGML_AVX=ON"
set "GGML_AVX2=ON"
set "GGML_FMA=ON"
set "GGML_F16C=ON"
set "CMAKE_C_FLAGS_RELEASE=-nologo -MD -Brepro -W0 /O2 /Ob2 /DNDEBUG"
set "CMAKE_CXX_FLAGS_RELEASE=-nologo -MD -Brepro -W0 /O2 /Ob2 /DNDEBUG"
