@echo off
title Taiwan Stock Tracker

set PYTHON_PATH=C:\Users\TDC-G04432\AppData\Local\Programs\Python\Python311\python.exe
set SCRIPT_DIR=%~dp0

"%PYTHON_PATH%" "%SCRIPT_DIR%tracker_realtime.py" -i 30
