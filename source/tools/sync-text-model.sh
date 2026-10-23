#!/bin/sh
# Odświeża wspólny model tekstu i wektory testowe z oryginału (F4.1). Użycie: tools/sync-text-model.sh [katalog oryginału] [--check]
# --check: tylko porównuje (kod wyjścia 1 przy rozjeździe), nic nie kopiuje.
set -e
cd "$(dirname "$0")/.."
B="${CG_SRC_DIR:-}"
CHECK=0
for a in "$@"; do case "$a" in --check) CHECK=1;; *) B="$a";; esac; done
[ -f "$B/text-model.js" ] || { echo "Brak $B/text-model.js"; exit 2; }
rc=0
cmp_or_copy() { # źródło, cel, [ile pierwszych linii celu pominąć]
  skip="${3:-0}"
  if [ "$skip" -gt 0 ]; then tail -n +$((skip+1)) "$2" > /tmp/vizrs-sync-cmp.$$; else cp "$2" /tmp/vizrs-sync-cmp.$$; fi
  if cmp -s "$1" /tmp/vizrs-sync-cmp.$$; then echo "zgodne:   $2"; else
    echo "ROZJAZD:  $2"; rc=1
    if [ "$CHECK" = 0 ]; then
      if [ "$skip" -gt 0 ]; then { head -n "$skip" "$2"; cat "$1"; } > /tmp/vizrs-sync-new.$$ && mv /tmp/vizrs-sync-new.$$ "$2"; else cp "$1" "$2"; fi
      echo "          odświeżono"
    fi
  fi
  rm -f /tmp/vizrs-sync-cmp.$$
}
cmp_or_copy "$B/text-model.js" web/text-model.js 2
for f in model field-rules word-replace spell-tokens; do cmp_or_copy "$B/test-fixtures/text/$f.json" "tests/vectors/$f.json"; done
[ "$CHECK" = 1 ] && exit $rc
exit 0
