@echo off
:: Tryb DOM: laduje tylko do 80%, ladowanie wznawia ponizej 75%.
net session >nul 2>&1 || (powershell -NoProfile -Command "Start-Process '%~f0' -Verb RunAs" & exit /b)
"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe" --PrimaryBattChargeCfg=Custom:75-80
echo.
echo Aktualne ustawienie:
"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe" --PrimaryBattChargeCfg
pause
