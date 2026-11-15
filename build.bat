@echo off
rem Builds the Pack Creator into one exe: target\release\pack-creator.exe
rem Needs Rust (MSVC toolchain) and Node. Close any running pack-creator.exe first, or the build cannot replace it.
setlocal
cd /d "%~dp0"

if not exist ui\node_modules (
    echo Installing the page packages...
    pushd ui
    call npm install
    if errorlevel 1 popd & goto :failed
    popd
)

echo Building...
call ui\node_modules\.bin\tauri build --no-bundle -- --features tauri/custom-protocol
if errorlevel 1 goto :failed

echo.
echo Done: %~dp0target\release\pack-creator.exe
exit /b 0

:failed
echo.
echo Build failed.
exit /b 1
