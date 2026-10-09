@echo off
:: Tryb UCZELNIA: normalne ladowanie do 100%.
net session >nul 2>&1 || (powershell -NoProfile -Command "Start-Process '%~f0' -Verb RunAs" & exit /b)
"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe" --PrimaryBattChargeCfg=Standard
echo.
echo Aktualne ustawienie:
"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe" --PrimaryBattChargeCfg
pause
