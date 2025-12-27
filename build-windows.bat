@echo off
REM Build script for Windows
REM Run this from Command Prompt or PowerShell

echo === TrackPersonalInsights Build Script for Windows ===
echo Building release binary...

REM Build
cargo build --release --target x86_64-pc-windows-msvc

REM Copy to releases folder
if not exist "releases" mkdir releases
copy /Y target\x86_64-pc-windows-msvc\release\TrackPersonalInsights.exe releases\TrackPersonalInsights-windows-x86_64.exe

echo.
echo Build complete!
echo Binary: releases\TrackPersonalInsights-windows-x86_64.exe
dir releases\TrackPersonalInsights-windows-x86_64.exe

echo.
echo To run: releases\TrackPersonalInsights-windows-x86_64.exe
pause
