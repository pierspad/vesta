#!/usr/bin/env python3
"""Repeatable correctness gate. Network and desktop builds are opt-in."""
import argparse
import json
import subprocess
import time
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--desktop', action='store_true', help='Include native Vesta tests; requires Tauri/Vulkan build dependencies')
    parser.add_argument('--smoke', action='store_true', help='Build the CLI and validate a synthetic media/APKG export')
    args = parser.parse_args()
    rust = ['cargo', 'test', '--workspace', '--exclude', 'whisper-bench']
    if not args.desktop: rust += ['--exclude', 'vesta']
    commands = [(ROOT / 'apps/srt-gui', ['npm', 'run', 'check']),
                (ROOT / 'apps/srt-gui', ['npm', 'test']),
                (ROOT / 'apps/srt-gui', ['npm', 'run', 'test:dev-css']),
                (ROOT / 'apps/srt-gui', ['npm', 'run', 'check:i18n']),
                (ROOT / 'apps/srt-gui', ['python3', 'scripts/check_missing_translations.py', '--fail-on-issues', '--block-reasons', 'missing_locale,missing_key,empty_value,placeholder_mismatch,same_as_english', '--output', str(Path(tempfile.gettempdir()) / 'vesta-quality-i18n.json')]),
                (ROOT / 'apps/srt-gui', ['npm', 'run', 'build']),
                (ROOT, rust), (ROOT, ['bash', 'build-scripts/check_internal_crate_versions.sh'])]
    if args.smoke:
        commands += [(ROOT, ['cargo', 'build', '-p', 'srt-flashcards-cli']),
                     (ROOT, ['python3', 'build-scripts/quality_smoke.py'])]
    timings = []
    for cwd, command in commands:
        start = time.monotonic()
        subprocess.run(command, cwd=cwd, check=True)
        timings.append({'command': command, 'seconds': round(time.monotonic() - start, 3)})
    print(json.dumps({'checks': timings}, indent=2))

if __name__ == '__main__': main()
