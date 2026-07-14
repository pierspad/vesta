#!/usr/bin/env bash
#
# download_detour.sh — scarica "Detour" (1945, Edgar G. Ulmer) da archive.org.
#
# Detour è di PUBBLICO DOMINIO (US: copyright non rinnovato), quindi è l'unico
# film usato come test media in questo repo. Il video NON è committato
# (i media sono gitignorati): questo script lo rigenera in locale.
#
# Item:  https://archive.org/details/detour1945HD
# File:  Detour.1945.1080p.BluRay.x264-[YTS.AM].mp4  (~1.1 GiB, 1080p, 69 min)

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$DIR/Detour(1945).mp4"
URL='https://archive.org/download/detour1945HD/Detour.1945.1080p.BluRay.x264-%5BYTS.AM%5D.mp4'

if [ -f "$OUT" ]; then
    echo "✔ $OUT esiste già ($(du -h "$OUT" | cut -f1)), niente da fare."
    exit 0
fi

echo "▶ Scarico Detour (1945) da archive.org (~1.1 GiB)…"
if command -v curl >/dev/null; then
    curl -L --fail --retry 3 -C - -o "$OUT.part" "$URL"
else
    wget -c -O "$OUT.part" "$URL"
fi
mv "$OUT.part" "$OUT"

echo "✔ Salvato in $OUT"
echo "  Per generare le 3 parti 'serie TV': ../SERIE_TV/make_detour_parts.sh"
