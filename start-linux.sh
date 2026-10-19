#!/bin/sh
# Uruchamia VizRS z katalogiem danych obok skryptu.
cd "$(dirname "$0")"
exec ./bin/linux-x86_64/vizrs --data ./vizrs-data "$@"
