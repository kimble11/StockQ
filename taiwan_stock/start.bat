@echo off
title Taiwan Stock Tracker

set PYTHON_PATH=C:\Users\TDC-G04432\AppData\Local\Programs\Python\Python311\python.exe
set SCRIPT_DIR=%~dp0

echo ========================================
echo        Taiwan Stock Tracker
echo ========================================
echo.

set /p interval="Enter refresh interval in seconds (default 30): "

if "%interval%"=="" set interval=30

echo.
echo Starting... Interval: %interval% seconds
echo Press Ctrl+C to stop
echo.

"%PYTHON_PATH%" "%SCRIPT_DIR%tracker.py" -i %interval%

pause
