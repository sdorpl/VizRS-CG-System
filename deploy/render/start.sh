#!/bin/sh
# Start w chmurze: nasłuch na wszystkich interfejsach, port z $PORT (Render: 10000),
# dane na trwałym dysku. Bez tokenu odmawiamy startu – inaczej każdy w internecie steruje anteną.
set -e
if [ -z "$VIZRS_TOKEN" ]; then
    echo "Brak VIZRS_TOKEN – ustaw go w panelu (Environment). Dozwolone znaki: A-Z a-z 0-9 - _ ." >&2
    exit 1
fi
DATA="${VIZRS_DATA:-/var/data}"
mkdir -p "$DATA/assets"
exec vizrs --bind 0.0.0.0 --port "${PORT:-10000}" --tcp 0 --data "$DATA"
