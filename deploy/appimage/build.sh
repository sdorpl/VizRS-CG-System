#!/bin/sh
# Buduje VizRS-Artist-<wersja>-x86_64.AppImage i VizRS-Trio-<wersja>-x86_64.AppImage.
# Użycie: deploy/appimage/build.sh ŚCIEŻKA/vizrs ŚCIEŻKA/vizrs-shell WERSJA [katalog-wyjściowy]
# appimagetool: $APPIMAGETOOL albo pobierany do katalogu wyjściowego.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
VIZRS="$(realpath "$1")"; SHELLBIN="$(realpath "$2")"; VER="$3"; OUT="$(realpath -m "${4:-.}")"
[ -x "$VIZRS" ] && [ -x "$SHELLBIN" ] && [ -n "$VER" ] || { echo "Użycie: $0 vizrs vizrs-shell wersja [wyjście]" >&2; exit 1; }
mkdir -p "$OUT"
TOOL="${APPIMAGETOOL:-$OUT/appimagetool}"
if [ ! -x "$TOOL" ]; then
  curl -fsSL -o "$TOOL" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$TOOL"
fi
for w in Artist Trio; do
  kind=$(echo "$w" | tr 'A-Z' 'a-z')
  if [ "$w" = Artist ]; then desc="Projektowanie szablonów grafiki CG"; else desc="Playout grafiki CG na żywo"; fi
  dir="$OUT/VizRS-$w.AppDir"
  rm -rf "$dir"; mkdir -p "$dir/usr/bin"
  install -m755 "$VIZRS" "$SHELLBIN" "$dir/usr/bin/"
  sed "s/@KIND@/$kind/" "$HERE/AppRun" > "$dir/AppRun"; chmod 755 "$dir/AppRun"
  cp "$HERE/../vizrs-256.png" "$dir/vizrs.png"
  cat > "$dir/vizrs-$kind.desktop" <<D
[Desktop Entry]
Type=Application
Name=VizRS $w
Comment=$desc
Exec=AppRun
Icon=vizrs
Categories=AudioVideo;Video;
Terminal=false
D
  ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$dir" "$OUT/VizRS-$w-$VER-x86_64.AppImage"
  rm -rf "$dir"
done
