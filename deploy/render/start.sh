#!/bin/sh
# Start w chmurze: nasłuch na wszystkich interfejsach, port z $PORT (Render: 10000).
# Bez tokenu odmawiamy startu – inaczej każdy w internecie steruje anteną.
set -e
if [ -z "$VIZRS_TOKEN" ]; then
    echo "Brak VIZRS_TOKEN – ustaw go w panelu (Environment). Dozwolone znaki: A-Z a-z 0-9 - _ ." >&2
    exit 1
fi
DATA="${VIZRS_DATA:-/var/data}"
SEED="${VIZRS_SEED:-/usr/local/share/vizrs-seed}"
mkdir -p "$DATA/assets"
# Bez trwałego dysku (darmowy plan) katalog jest pusty po każdym starcie – wgrywamy show z repo.
if [ ! -f "$DATA/show.json" ] && [ -f "$SEED/show.json" ]; then
    cp "$SEED/show.json" "$DATA/show.json"
fi
if [ -d "$SEED/assets" ]; then
    cp -rn "$SEED/assets/." "$DATA/assets/" 2>/dev/null || true
fi
exec vizrs --bind 0.0.0.0 --port "${PORT:-10000}" --tcp 0 --data "$DATA"
