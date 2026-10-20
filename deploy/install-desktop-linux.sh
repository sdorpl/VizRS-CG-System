#!/bin/sh
# Skróty VizRS w menu aplikacji (Linux, GNOME/KDE/XFCE…): „VizRS Artist” i „VizRS Trio”.
# Każdy otwiera okno aplikacji (Chromium/Chrome/Edge w trybie --app) z serwerem na wskazanym katalogu danych;
# drugie okno dołącza do już działającego serwera.
#
# Użycie: deploy/install-desktop-linux.sh [ścieżka-do-vizrs] [katalog-danych]
#   domyślnie: bin/linux-x86_64/vizrs i vizrs-data/ obok repozytorium
# Usunięcie: rm ~/.local/share/applications/vizrs-*.desktop ~/.local/share/icons/hicolor/scalable/apps/vizrs.svg
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$(realpath "${1:-$ROOT/bin/linux-x86_64/vizrs}")"
DATA="${2:-$ROOT/vizrs-data}"
mkdir -p "$DATA"
DATA="$(realpath "$DATA")"
[ -x "$BIN" ] || { echo "Brak pliku wykonywalnego: $BIN" >&2; exit 1; }
case "$BIN$DATA" in *'"'*|*'`'*|*'$'*|*'\'*|*%*) echo "Ścieżka zawiera znak niedozwolony w pliku .desktop (\" \` \$ \\ %)" >&2; exit 1 ;; esac

SHARE="${XDG_DATA_HOME:-$HOME/.local/share}"
mkdir -p "$SHARE/applications" "$SHARE/icons/hicolor/scalable/apps"
cp "$ROOT/deploy/vizrs.svg" "$SHARE/icons/hicolor/scalable/apps/vizrs.svg"

for w in Artist Trio; do
  lower=$(echo "$w" | tr 'A-Z' 'a-z')
  if [ "$w" = Artist ]; then desc="Projektowanie szablonów grafiki CG"; else desc="Playout grafiki CG na żywo"; fi
  cat > "$SHARE/applications/vizrs-$lower.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=VizRS $w
Comment=$desc
Exec="$BIN" --data "$DATA" $lower
Icon=vizrs
Terminal=false
Categories=AudioVideo;Video;
StartupWMClass=VizRS-$w
EOF
  echo "Utworzono $SHARE/applications/vizrs-$lower.desktop"
done
command -v update-desktop-database >/dev/null && update-desktop-database "$SHARE/applications" 2>/dev/null || true
echo "Dane: $DATA  –  okna szukają Chromium/Chrome/Edge (inna przeglądarka: zmienna VIZRS_BROWSER)."
