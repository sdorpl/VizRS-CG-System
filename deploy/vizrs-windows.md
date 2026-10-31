# VizRS jako usługa Windows

VizRS jest zwykłym programem konsolowym; jako usługę najprościej uruchomić go przez [NSSM](https://nssm.cc/) (Non-Sucking Service Manager):

```bat
nssm install VizRS "C:\VizRS\bin\windows-x86_64\vizrs.exe" --bind 127.0.0.1 --data C:\VizRS\vizrs-data
nssm set VizRS AppEnvironmentExtra VIZRS_TOKEN=ZMIEN_MNIE
nssm set VizRS AppExit Default Restart
nssm set VizRS AppStdout C:\VizRS\vizrs.log
nssm start VizRS
```

Bez NSSM: skrót do `start-windows.bat serve` w folderze „Autostart” (`shell:startup`); sam `start-windows.bat` otwiera okno Trio.
Dostęp z innego komputera: `--bind 0.0.0.0` + `VIZRS_TOKEN`, a w Zaporze Windows otwórz port TCP 8088, a jeśli ich używasz, także TCP 6100 (komendy) i port UDP z `--osc`.
