@echo off
setlocal enabledelayedexpansion

echo Converting all MP3 files to OGG format...
echo.

for %%f in (*.mp3) do (
    set "filename=%%~nf"
    echo Converting: %%f
    ffmpeg -i "%%f" -vn -c:a libvorbis -q:a 4 "!filename!.ogg"
    echo.
)

echo.
echo Conversion complete!
pause
