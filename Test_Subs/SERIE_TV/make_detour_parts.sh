#!/usr/bin/env bash
#
# make_detour_parts.sh — divide "Detour" (1945) in 3 parti uguali, simulando
# tre episodi di una serie TV. Le parti combaciano con i sottotitoli già
# presenti in questa cartella (Detour-{en,it}_parte{1,2,3}.srt), che furono
# creati proprio su questi tagli: durata_totale / 3 ≈ 1381.746 s per parte.
#
# Prerequisito: Test_Subs/FILM/Detour(1945).mp4
#   (scaricalo con ../FILM/download_detour.sh — Detour è di pubblico dominio)
#
# Il taglio è ri-encodato (x264 veryfast, CRF 20) perché con -c copy ffmpeg
# aggancerebbe i keyframe più vicini e i timestamp dei sottotitoli non
# combacerebbero più. L'accurate seek (-ss prima di -i, con re-encode) dà
# tagli esatti al frame.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$DIR/../FILM/Detour(1945).mp4"

[ -f "$SRC" ] || { echo "✗ Film non trovato: $SRC — esegui prima ../FILM/download_detour.sh" >&2; exit 1; }
command -v ffmpeg  >/dev/null || { echo "✗ ffmpeg non trovato" >&2; exit 1; }
command -v ffprobe >/dev/null || { echo "✗ ffprobe non trovato" >&2; exit 1; }

TOTAL="$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$SRC")"
# Durata di ogni parte = totale / 3 (stessa regola usata per creare gli .srt).
PART="$(python3 -c "print(f'{${TOTAL}/3:.3f}')" 2>/dev/null || awk "BEGIN{printf \"%.3f\", ${TOTAL}/3}")"

echo "▶ Film: ${TOTAL}s → 3 parti da ${PART}s"

for i in 1 2 3; do
    OUT="$DIR/Detour_parte${i}.mp4"
    if [ -f "$OUT" ]; then
        echo "✔ $OUT esiste già, skip."
        continue
    fi
    START="$(awk "BEGIN{printf \"%.3f\", (${i}-1)*${PART}}")"
    echo "▶ Parte $i: start=${START}s, durata=${PART}s"
    ffmpeg -hide_banner -loglevel warning -y \
        -ss "$START" -t "$PART" -i "$SRC" \
        -c:v libx264 -preset veryfast -crf 20 \
        -c:a aac -b:a 160k \
        -movflags +faststart \
        "$OUT"
    echo "✔ $OUT"
done

echo "✔ Fatto. Le parti combaciano con Detour-{en,it}_parte{1,2,3}.srt"
