@echo off
REM Cleanup Old Grok Installation Script
REM This script removes the old grok.exe from the Cargo bin directory
REM to prevent version conflicts

echo.
echo Grok CLI - Cleanup Old Installation
echo ======================================
echo.

REM Define paths
set CARGO_GROK=%USERPROFILE%\.cargo\bin\grok.exe
set NEW_GROK=%LOCALAPPDATA%\grok-cli\bin\grok.exe

echo Checking for installed versions...
echo.

REM Check for old Cargo installation
if exist "%CARGO_GROK%" (
    echo [FOUND] Old Cargo installation:
    echo   Location: %CARGO_GROK%

    REM Try to get version
    "%CARGO_GROK%" --version 2>nul
    if errorlevel 1 (
        echo   Version:  Unable to determine
    )

    set OLD_FOUND=1
) else (
    echo [OK] No old Cargo installation found.
    set OLD_FOUND=0
)

echo.

REM Check for new installation
if exist "%NEW_GROK%" (
    echo [FOUND] New installation:
    echo   Location: %NEW_GROK%

    REM Try to get version
    "%NEW_GROK%" --version 2>nul
    if errorlevel 1 (
        echo   Version:  Unable to determine
    )
) else (
    echo [WARNING] New installation not found at:
    echo           %NEW_GROK%
    echo           Please run the installer first!
)

echo.
echo ========================================
echo.

REM Exit if no old version found
if %OLD_FOUND%==0 (
    echo [OK] No cleanup needed!
    echo.
    goto END
)

REM Prompt for removal
echo Do you want to remove the old Cargo installation?
echo This will delete: %CARGO_GROK%
echo.
set /p CONFIRM="Type 'yes' to continue or 'no' to cancel: "

if /i not "%CONFIRM%"=="yes" (
    echo.
    echo [CANCELLED] Old installation was not removed.
    echo You can manually delete it later if needed.
    echo.
    goto END
)

echo.
echo Removing old installation...

REM Try to delete the old version
del /F "%CARGO_GROK%" 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to remove old installation.
    echo.
    echo The file may be in use. Please:
    echo   1. Close all running grok instances
    echo   2. Close all PowerShell/terminal windows using grok
    echo   3. Run this script again
    echo.
    goto END
)

echo [SUCCESS] Old Cargo installation removed!
echo.
echo You may need to restart your PowerShell session or run:
echo   refreshenv
echo or close and reopen your terminal to use the new version.
echo.

:END
echo Cleanup complete!
echo.

REM Verify which grok will be used
echo Verifying grok command...
where grok >nul 2>&1
if errorlevel 1 (
    echo   [ERROR] 'grok' command not found in PATH
    echo   Please make sure the installer completed successfully.
) else (
    for /f "tokens=*" %%i in ('where grok') do (
        echo   Current grok path: %%i
        goto VERSION_CHECK
    )
)

:VERSION_CHECK
grok --version 2>nul | findstr /C:"grok-cli" >nul
if errorlevel 1 (
    echo   [ERROR] Unable to get version
) else (
    grok --version 2>nul | findstr /C:"0.1.4" >nul
    if not errorlevel 1 (
        echo.
        echo [SUCCESS] You are using the correct version!
    ) else (
        echo.
        echo [WARNING] Still using old version!
        echo           Please restart your terminal.
    )
)

echo.
pause
