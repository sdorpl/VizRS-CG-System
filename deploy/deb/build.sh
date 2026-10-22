#!/bin/sh
# Buduje pakiet Debian/Ubuntu vizrs_<wersja>_amd64.deb: serwer vizrs, okno natywne vizrs-shell,
# skróty w menu (VizRS Artist, VizRS Trio, VizRS Trio – serwer zdalny), ikona, usługa systemd (wyłączona).
# Użycie: deploy/deb/build.sh ŚCIEŻKA/vizrs ŚCIEŻKA/vizrs-shell WERSJA [katalog-wyjściowy]
# Wymaga dpkg-deb. Pakiet działa na systemach z glibc nie starszą niż ta, na której zbudowano binarki
# (wydanie na GitHub: Ubuntu 22.04).
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
VIZRS="$(realpath "$1")"; SHELLBIN="$(realpath "$2")"; VER="$3"; OUT="$(realpath -m "${4:-.}")"
[ -x "$VIZRS" ] && [ -x "$SHELLBIN" ] && [ -n "$VER" ] || { echo "Użycie: $0 vizrs vizrs-shell wersja [wyjście]" >&2; exit 1; }
mkdir -p "$OUT"
ROOT="$OUT/vizrs_${VER}_amd64"
rm -rf "$ROOT"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/bin" "$ROOT/usr/lib/vizrs" "$ROOT/usr/share/applications" \
  "$ROOT/usr/share/icons/hicolor/scalable/apps" "$ROOT/usr/share/icons/hicolor/256x256/apps" \
  "$ROOT/lib/systemd/system" "$ROOT/usr/share/doc/vizrs"
install -m755 "$VIZRS" "$SHELLBIN" "$ROOT/usr/bin/"
install -m644 "$HERE/../vizrs.svg" "$ROOT/usr/share/icons/hicolor/scalable/apps/vizrs.svg"
install -m644 "$HERE/../vizrs-256.png" "$ROOT/usr/share/icons/hicolor/256x256/apps/vizrs.png"

# okno: dane użytkownika w ~/.local/share/vizrs (wspólne dla Artist i Trio – drugie okno dołącza do serwera)
cat > "$ROOT/usr/lib/vizrs/vizrs-window" <<'S'
#!/bin/sh
: "${VIZRS_DATA:=${XDG_DATA_HOME:-$HOME/.local/share}/vizrs}"
export VIZRS_DATA
mkdir -p "$VIZRS_DATA"
exec /usr/bin/vizrs-shell --vizrs /usr/bin/vizrs "$@"
S
chmod 755 "$ROOT/usr/lib/vizrs/vizrs-window"

desktop() { # plik nazwa opis argumenty [NoDisplay]
  cat > "$ROOT/usr/share/applications/$1.desktop" <<D
[Desktop Entry]
Type=Application
Name=$2
Comment=$3
Exec=/usr/lib/vizrs/vizrs-window $4
Icon=vizrs
Terminal=false
Categories=AudioVideo;Video;
StartupWMClass=vizrs-shell
D
  [ -z "$5" ] || echo "NoDisplay=true" >> "$ROOT/usr/share/applications/$1.desktop"
}
desktop vizrs-artist "VizRS Artist" "Projektowanie szablonów grafiki CG" artist
desktop vizrs-trio "VizRS Trio" "Playout grafiki CG na żywo" trio
desktop vizrs-trio-zdalnie "VizRS Trio – serwer zdalny" "Trio sterujące serwerem VizRS na VPS (adres i token)" connect
# Wayland rozpoznaje okno po identyfikatorze aplikacji (nazwa programu) – ten wpis daje oknom ikonę VizRS
desktop vizrs-shell "VizRS" "Okno natywne VizRS" artist hidden

# usługa serwera bez GUI (VPS, studio): wyłączona – włączenie opisuje /usr/share/doc/vizrs/README.Debian
{ echo "# Usługa VizRS z pakietu vizrs – włączenie: /usr/share/doc/vizrs/README.Debian"
  sed -e 's#/opt/vizrs/vizrs#/usr/bin/vizrs#' -e '1,/^\[Unit\]/{/^#/d}' "$HERE/../vizrs.service"; } > "$ROOT/lib/systemd/system/vizrs.service"
cat > "$ROOT/usr/share/doc/vizrs/README.Debian" <<'R'
VizRS – grafika CG na żywo dla OBS

Okna: menu aplikacji → VizRS Artist / VizRS Trio / VizRS Trio – serwer zdalny.
Dane okien: ~/.local/share/vizrs (zmienna VIZRS_DATA).

Serwer bez GUI (usługa systemd, dane w /var/lib/vizrs):
  sudo systemctl edit vizrs      # [Service] Environment=VIZRS_TOKEN=<długi token>
  sudo systemctl enable --now vizrs
Dostęp z sieci: --bind 0.0.0.0 w ExecStart i reverse proxy z HTTPS (Caddy, nginx).
R
cp "$HERE/../../CHANGELOG.md" "$ROOT/usr/share/doc/vizrs/changelog"

cat > "$ROOT/DEBIAN/control" <<C
Package: vizrs
Version: $VER
Section: video
Priority: optional
Architecture: amd64
Maintainer: VizRS <noreply@github.com>
Depends: libc6, libgcc-s1, libwebkit2gtk-4.1-0, libgtk-3-0
Suggests: hunspell-pl, obs-studio
Installed-Size: $(du -ks "$ROOT/usr" "$ROOT/lib" | awk '{s+=$1} END {print s}')
Homepage: https://github.com/sdorpl/VizRS-CG-System
Description: grafika telewizyjna CG na żywo (Artist + Trio) dla OBS
 VizRS projektuje szablony grafiki (Artist) i wydaje strony na antenę (Trio).
 Wyjście to przezroczysta strona 1920x1080 dla źródła Przeglądarka w OBS,
 vMix, Tricaster. Serwer działa bez zależności, okno natywne używa WebKitGTK.
C
cat > "$ROOT/DEBIAN/postinst" <<'P'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
  getent passwd vizrs >/dev/null || useradd --system --home-dir /var/lib/vizrs --create-home --shell /usr/sbin/nologin vizrs
  command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q /usr/share/icons/hicolor || true
  command -v systemctl >/dev/null && systemctl daemon-reload >/dev/null 2>&1 || true
fi
exit 0
P
cat > "$ROOT/DEBIAN/prerm" <<'P'
#!/bin/sh
set -e
if [ "$1" = remove ] && command -v systemctl >/dev/null; then
  systemctl stop vizrs >/dev/null 2>&1 || true
  systemctl disable vizrs >/dev/null 2>&1 || true
fi
exit 0
P
# uprawnienia niezależne od umaski budującego
find "$ROOT" -type d -exec chmod 755 {} +
find "$ROOT" -type f -exec chmod 644 {} +
chmod 755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/prerm" "$ROOT/usr/bin/vizrs" "$ROOT/usr/bin/vizrs-shell" "$ROOT/usr/lib/vizrs/vizrs-window"
dpkg-deb --root-owner-group -Zxz --build "$ROOT" "$OUT/vizrs_${VER}_amd64.deb"
rm -rf "$ROOT"
