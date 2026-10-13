@echo off
rem Double-cliquez ici : construit le launcher et prepare a-envoyer\ et a-envoyer.zip pour le VPS.
rem Le travail est fait par outils\preparer.ps1.
setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0outils\preparer.ps1"
set CODE=%ERRORLEVEL%
echo.
if not "%CODE%"=="0" echo La preparation a echoue (code %CODE%) : lisez le message ci-dessus.
pause
exit /b %CODE%
