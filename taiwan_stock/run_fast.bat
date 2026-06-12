@echo off
title Taiwan Stock Tracker (10s)

set PYTHON_PATH=C:\Users\TDC-G04432\AppData\Local\Programs\Python\Python311\python.exe
set SCRIPT_DIR=%~dp0

"%PYTHON_PATH%" "%SCRIPT_DIR%tracker.py" -i 10
