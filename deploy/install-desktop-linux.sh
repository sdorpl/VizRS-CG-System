#!/bin/sh
# Skróty VizRS w menu aplikacji (Linux, GNOME/KDE/XFCE…): „VizRS Artist” i „VizRS Trio”.
# Gdy obok vizrs leży vizrs-shell – okno natywne (WebKitGTK, menu, „Zapisz jako”), inaczej okno aplikacji
# Chromium/Chrome/Edge (vizrs artist|trio). Serwer startuje na wskazanym katalogu danych; drugie okno dołącza do niego.
#
# Użycie: deploy/install-desktop-linux.sh [--chromium] [ścieżka-do-vizrs] [katalog-danych]
#   domyślnie: bin/linux-x86_64/vizrs i vizrs-data/ obok repozytorium; --chromium = bez okna natywnego
# Usunięcie: rm ~/.local/share/applications/vizrs-*.desktop ~/.local/share/icons/hicolor/scalable/apps/vizrs.svg
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHROMIUM=0
if [ "$1" = "--chromium" ]; then CHROMIUM=1; shift; fi
BIN="$(realpath "${1:-$ROOT/bin/linux-x86_64/vizrs}")"
DATA="${2:-$ROOT/vizrs-data}"
mkdir -p "$DATA"
DATA="$(realpath "$DATA")"
[ -x "$BIN" ] || { echo "Brak pliku wykonywalnego: $BIN" >&2; exit 1; }
SHELLBIN="$(dirname "$BIN")/vizrs-shell"
{ [ "$CHROMIUM" = 0 ] && [ -x "$SHELLBIN" ]; } || SHELLBIN=""
case "$BIN$DATA$SHELLBIN" in *'"'*|*'`'*|*'$'*|*'\'*|*%*) echo "Ścieżka zawiera znak niedozwolony w pliku .desktop (\" \` \$ \\ %)" >&2; exit 1 ;; esac

SHARE="${XDG_DATA_HOME:-$HOME/.local/share}"
mkdir -p "$SHARE/applications" "$SHARE/icons/hicolor/scalable/apps"
cp "$ROOT/deploy/vizrs.svg" "$SHARE/icons/hicolor/scalable/apps/vizrs.svg"

for w in Artist Trio; do
  lower=$(echo "$w" | tr 'A-Z' 'a-z')
  if [ "$w" = Artist ]; then desc="Projektowanie szablonów grafiki CG"; else desc="Playout grafiki CG na żywo"; fi
  if [ -n "$SHELLBIN" ]; then
    exec_line="\"$SHELLBIN\" --vizrs \"$BIN\" --data \"$DATA\" $lower"; wmclass=vizrs-shell
  else
    exec_line="\"$BIN\" --data \"$DATA\" $lower"; wmclass="VizRS-$w"
  fi
  cat > "$SHARE/applications/vizrs-$lower.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=VizRS $w
Comment=$desc
Exec=$exec_line
Icon=vizrs
Terminal=false
Categories=AudioVideo;Video;
StartupWMClass=$wmclass
EOF
  echo "Utworzono $SHARE/applications/vizrs-$lower.desktop"
done
if [ -n "$SHELLBIN" ]; then
  # Wayland rozpoznaje okno po identyfikatorze aplikacji (nazwa programu) – ten wpis daje oknom ikonę VizRS
  cat > "$SHARE/applications/vizrs-shell.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=VizRS
Comment=Okno natywne VizRS
Exec="$SHELLBIN" --vizrs "$BIN" --data "$DATA" artist
Icon=vizrs
Terminal=false
NoDisplay=true
StartupWMClass=vizrs-shell
EOF
  echo "Okno natywne: $SHELLBIN"
fi
command -v update-desktop-database >/dev/null && update-desktop-database "$SHARE/applications" 2>/dev/null || true
if [ -n "$SHELLBIN" ]; then echo "Dane: $DATA"; else echo "Dane: $DATA  –  okna szukają Chromium/Chrome/Edge (inna przeglądarka: zmienna VIZRS_BROWSER)."; fi
