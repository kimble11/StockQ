@echo off
title Stock Launcher

:menu
cls
echo ==================================================
echo   Stock Launcher
echo ==================================================
echo   1. Taiwan Stock Tracker (Rust)
echo   2. US Stock Tracker (Rust)
echo   0. Exit
echo ==================================================
set /p choice=Select (1/2/0): 

if "%choice%"=="1" goto tw
if "%choice%"=="2" goto us
if "%choice%"=="0" goto end
goto menu

:tw
cd /d "%~dp0taiwan_stock"
.\stock_tracker.exe
cd /d "%~dp0"
goto menu

:us
cd /d "%~dp0us_stock"
.\us_stock_tracker.exe
cd /d "%~dp0"
goto menu

:end
exit
