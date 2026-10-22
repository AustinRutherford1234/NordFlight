@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy RemoteSigned -File "%~dp0update-windows.ps1"
if errorlevel 1 (
  echo.
  echo NordFlight update failed. See the message above.
  pause
  exit /b 1
)
echo.
echo NordFlight is updated. You can launch it from your usual shortcut.
pause
