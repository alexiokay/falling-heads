@echo off
REM FallingHeads Release Script - Simple batch version
setlocal

set VERSION=0.2.5
set DIST_FOLDER=FallingHeads-v%VERSION%-Windows

echo.
echo ========================================
echo   Building FallingHeads v%VERSION%
echo ========================================
echo.

REM Step 1: Build release
echo [1/5] Building release executable...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Build failed!
    exit /b 1
)
echo OK: Build complete!
echo.

REM Step 2: Create distribution folder
echo [2/5] Creating distribution folder...
if exist "%DIST_FOLDER%" rmdir /s /q "%DIST_FOLDER%"
mkdir "%DIST_FOLDER%"
echo OK: Created %DIST_FOLDER%
echo.

REM Step 3: Copy executable
echo [3/5] Copying files...
copy /y "target\release\falling-heads.exe" "%DIST_FOLDER%\" >nul
echo   - Copied falling-heads.exe

REM Step 4: Copy assets
xcopy /e /i /y "assets" "%DIST_FOLDER%\assets" >nul
echo   - Copied assets folder
echo.

REM Step 5: Create README.txt
echo [4/5] Creating README.txt...
(
echo ===========================================
echo        FallingHeads v%VERSION% - Windows
echo ===========================================
echo.
echo SYSTEM REQUIREMENTS:
echo - Windows 10/11 ^(64-bit^)
echo - Graphics card with OpenGL 3.3+ or DirectX 11
echo - 4 GB RAM
echo - 200 MB disk space
echo.
echo HOW TO RUN:
echo 1. Extract this folder anywhere
echo 2. Double-click falling-heads.exe
echo 3. Enjoy!
echo.
echo CONTROLS:
echo - A/D or Arrow Keys: Move hero left/right
echo - Hero has automatic AI movement
echo.
echo PERFORMANCE:
echo - First launch may take 10-20s to load assets
echo - If you get low FPS, try closing other apps
echo.
echo TROUBLESHOOTING:
echo If game doesn't start:
echo 1. Install Visual C++ Redistributable:
echo    https://aka.ms/vs/17/release/vc_redist.x64.exe
echo 2. Update graphics drivers
echo.
echo WHAT'S NEW IN v%VERSION%:
echo - Major performance improvements ^(CPU usage down 80%% when idle^)
echo - Fixed entity despawn crashes
echo - Death animations now exactly 1.0 second
echo - Enhanced performance diagnostics
echo.
echo LICENSE:
echo This game is provided as-is for free.
echo.
echo Built with Bevy Engine ^(https://bevyengine.org^)
echo Copyright 2025
echo.
echo ===========================================
) > "%DIST_FOLDER%\README.txt"
echo OK: Created README.txt
echo.

REM Step 6: Create ZIP
echo [5/5] Creating ZIP archive...
powershell -command "Compress-Archive -Path '%DIST_FOLDER%' -DestinationPath '%DIST_FOLDER%.zip' -Force"
if %ERRORLEVEL% EQU 0 (
    echo OK: Created %DIST_FOLDER%.zip
) else (
    echo WARNING: Could not create ZIP. Use Windows Explorer to zip manually.
)
echo.

echo ========================================
echo   SUCCESS! Release package created
echo ========================================
echo.
echo Distribution folder: %DIST_FOLDER%\
echo ZIP archive: %DIST_FOLDER%.zip
echo.
echo Next steps:
echo 1. Test: %DIST_FOLDER%\falling-heads.exe
echo 2. Upload %DIST_FOLDER%.zip to itch.io or GitHub!
echo.
pause
