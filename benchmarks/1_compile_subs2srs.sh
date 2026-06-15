#!/usr/bin/env bash
#
# 1_compile_subs2srs.sh — build the headless subs2srs benchmark harness.
#
# subs2srs ships only as a .NET 3.5 WinForms GUI with no command line. This
# compiles the *real* subs2srs generation classes (Worker*/Utils*) into a
# console executable, swapping three GUI types for headless no-op stubs
# (see benchmarks/subs2srs-headless/Stubs.cs). No generation logic is altered.

source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
cd "$REPO_ROOT"

need mcs
need ffmpeg

S2S_SRC="subs2srs_source_code/subs2srs"
HL="benchmarks/subs2srs-headless"
RSP="$HL/sources.rsp"

[ -d "$S2S_SRC" ] || die "subs2srs source not found at $S2S_SRC"

log "Collecting subs2srs compute sources (excluding the WinForms GUI)…"
# Exclude: main GUI form, original entry point, the WinForms message helper, the
# BackgroundWorker orchestrator (we replicate it), and every Dialog*/progress
# dialog. Their .Designer/.designer companions are excluded by the same pattern.
ls "$S2S_SRC"/*.cs "$S2S_SRC"/SubtitleCreator/*.cs 2>/dev/null \
  | grep -vE '/(FormMain|Program|UtilsMsg|SubsProcessor|DialogAbout|DialogAdvancedSubtitleOptions|DialogDuelingSubtitles|DialogExtractAudioFromMedia|DialogMkvExtract|DialogPref|DialogPreview|DialogPreviewSnapshot|DialogProgress|DialogSelectMkvTrack|DialogSubtitleStyle|DialogVideoDimensionsChooser|GroupBoxCheck)(\.Designer|\.designer)?\.cs$' \
  > "$RSP"
echo "$HL/Stubs.cs"   >> "$RSP"
echo "$HL/Program.cs" >> "$RSP"
ok "$(wc -l < "$RSP") source files."

log "Compiling subs2srs-headless.exe with mcs…"
mcs -target:exe -unsafe -out:"$HL/subs2srs-headless.exe" \
  -r:System -r:System.Drawing -r:System.Windows.Forms -r:System.Xml -r:System.Core -r:System.Data \
  "@$RSP"
[ -f "$HL/subs2srs-headless.exe" ] || die "Compilation produced no executable."
ok "Built $HL/subs2srs-headless.exe"

# subs2srs invokes ffmpeg as "<appdir>/Utils/ffmpeg/ffmpeg.exe". On Linux we point
# that at the system ffmpeg via a symlink — subs2srs code stays untouched.
log "Wiring system ffmpeg into the location subs2srs expects…"
mkdir -p "$HL/Utils/ffmpeg/presets"
ln -sf "$(command -v ffmpeg)" "$HL/Utils/ffmpeg/ffmpeg.exe"
ok "ffmpeg.exe -> $(readlink "$HL/Utils/ffmpeg/ffmpeg.exe")"

echo
ok "subs2srs harness ready. Smoke test:"
echo "   mono $HL/subs2srs-headless.exe --target <t.srt> --native <n.srt> --video <v> --output <dir>"
