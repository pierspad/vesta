#!/usr/bin/env python3
"""Repeatable correctness gate. Network and desktop builds are opt-in."""
import argparse
import json
import os
import re
import shutil
import subprocess
import time
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
TARGET_WARN_GIB = 40.0
HASH_DIR_RE = re.compile(r'^(.+)-[0-9a-f]{16}$')


def dir_size_bytes(path: Path) -> int:
    """Return total byte size of a directory tree without failing on vanished files."""
    if not path.exists():
        return 0
    if os.name == 'posix':
        proc = subprocess.run(['du', '-sb', str(path)], capture_output=True, text=True)
        if proc.returncode == 0 and proc.stdout.strip():
            try:
                return int(proc.stdout.split()[0])
            except (IndexError, ValueError):
                pass
    total = 0
    stack = [path]
    while stack:
        current = stack.pop()
        try:
            with os.scandir(current) as entries:
                for entry in entries:
                    if entry.is_symlink():
                        continue
                    if entry.is_dir(follow_symlinks=False):
                        stack.append(Path(entry.path))
                    else:
                        total += entry.stat(follow_symlinks=False).st_size
        except OSError:
            continue
    return total


def prune_stale_target(target_dir: Path, max_age_days: int) -> int:
    """Remove stale incremental caches and superseded build-script hashes older than max_age_days."""
    cutoff = time.time() - max_age_days * 86400
    freed = 0
    for inc_root in (
        target_dir / 'debug' / 'incremental',
        target_dir / 'integration' / 'debug' / 'incremental',
    ):
        if not inc_root.is_dir():
            continue
        for entry in list(inc_root.iterdir()):
            if not entry.is_dir() or entry.is_symlink():
                continue
            try:
                if entry.stat().st_mtime < cutoff:
                    size = dir_size_bytes(entry)
                    shutil.rmtree(entry, ignore_errors=True)
                    freed += size
            except OSError:
                continue

    for build_root in (
        target_dir / 'debug' / 'build',
        target_dir / 'integration' / 'debug' / 'build',
    ):
        if not build_root.is_dir():
            continue
        grouped: dict[str, list[tuple[float, Path]]] = {}
        for entry in list(build_root.iterdir()):
            if not entry.is_dir() or entry.is_symlink():
                continue
            match = HASH_DIR_RE.match(entry.name)
            if not match:
                continue
            try:
                grouped.setdefault(match.group(1), []).append((entry.stat().st_mtime, entry))
            except OSError:
                continue
        for entries in grouped.values():
            entries.sort(key=lambda item: item[0], reverse=True)
            for mtime, entry in entries[1:]:
                if mtime < cutoff:
                    size = dir_size_bytes(entry)
                    shutil.rmtree(entry, ignore_errors=True)
                    freed += size
    return freed


def check_target_footprint(prune: bool, max_age_days: int) -> None:
    """Warn when target/ exceeds TARGET_WARN_GIB, and optionally prune stale artifacts."""
    target_dir = ROOT / 'target'
    if not target_dir.exists():
        return
    if prune:
        freed = prune_stale_target(target_dir, max_age_days)
        print(f'Pruned {freed / (1024 ** 3):.2f} GiB of stale artifacts older than {max_age_days} days from {target_dir}.')
    size_gib = dir_size_bytes(target_dir) / (1024 ** 3)
    if size_gib > TARGET_WARN_GIB:
        print(
            f'WARNING: {target_dir} is {size_gib:.1f} GiB (> {TARGET_WARN_GIB:.0f} GiB). '
            'Run `python3 build-scripts/quality_check.py --prune-only` or remove stale `target/debug/incremental` / `target/integration`.'
        )

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--desktop', action='store_true', help='Include native Vesta tests; requires Tauri/Vulkan build dependencies')
    parser.add_argument('--smoke', action='store_true', help='Build the CLI and validate a synthetic media/APKG export')
    parser.add_argument('--browser', action='store_true', help='Run the Chromium suite; install Playwright Chromium first')
    parser.add_argument('--prune-target', action='store_true', help='Prune stale incremental/build artifacts from target/ after checks')
    parser.add_argument('--prune-only', action='store_true', help='Only prune stale target/ artifacts and exit')
    parser.add_argument('--prune-days', type=int, default=7, help='Age threshold in days for --prune-target / --prune-only (default: 7)')
    args = parser.parse_args()
    if args.prune_only:
        check_target_footprint(prune=True, max_age_days=args.prune_days)
        return
    rust = ['cargo', 'test', '--locked', '--workspace', '--exclude', 'whisper-bench']
    if not args.desktop: rust += ['--exclude', 'vesta']
    clippy = ['cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--exclude', 'whisper-bench']
    if not args.desktop: clippy += ['--exclude', 'vesta']
    clippy += ['--', '-D', 'warnings']
    commands = [(ROOT, ['python3', 'build-scripts/test_release_workflows.py']),
                (ROOT, ['python3', 'build-scripts/test_ci_apt.py']),
                (ROOT, ['python3', 'build-scripts/check_tauri_versions.py']),
                (ROOT, ['python3', 'build-scripts/test_tauri_versions.py']),
                (ROOT, ['python3', 'build-scripts/check_doc_contracts.py']),
                (ROOT, ['python3', 'build-scripts/test_doc_contracts.py']),
                (ROOT, ['python3', 'build-scripts/browser_ipc_contract.py', '--check']),
                (ROOT, ['python3', 'build-scripts/test_browser_ipc_contract.py']),
                (ROOT, ['python3', 'build-scripts/check_library_integration.py']),
                (ROOT, ['python3', 'build-scripts/generate_scorecard.py', '--check']),
                (ROOT, ['python3', 'build-scripts/scorecard_db.py', 'check']),
                (ROOT, ['python3', 'build-scripts/test_scorecard.py']),
                (ROOT / 'apps/vesta-gui', ['npm', 'run', 'check']),
                (ROOT / 'apps/vesta-gui', ['npm', 'test']),
                (ROOT / 'apps/vesta-gui', ['npm', 'run', 'test:dev-css']),
                (ROOT / 'apps/vesta-gui', ['npm', 'run', 'check:i18n']),
                (ROOT / 'apps/vesta-gui', ['python3', 'scripts/check_missing_translations.py', '--fail-on-issues', '--output', str(Path(tempfile.gettempdir()) / 'vesta-quality-i18n.json')]),
                (ROOT / 'apps/vesta-gui', ['npm', 'run', 'build']),
                (ROOT, ['cargo', 'fmt', '--all', '--check']),
                (ROOT, clippy),
                (ROOT, rust), (ROOT, ['bash', 'build-scripts/check_internal_crate_versions.sh'])]
    if args.browser:
        commands.append((ROOT / 'apps/vesta-gui', ['npm', 'run', 'test:browser']))
    if args.smoke:
        commands += [(ROOT, ['cargo', 'build', '--locked', '-p', 'vesta-flashcards-cli']),
                     (ROOT, ['python3', 'build-scripts/quality_smoke.py'])]
    timings = []
    for cwd, command in commands:
        start = time.monotonic()
        subprocess.run(command, cwd=cwd, check=True)
        timings.append({'command': command, 'seconds': round(time.monotonic() - start, 3)})
    print(json.dumps({'checks': timings}, indent=2))
    subprocess.run(['python3', 'build-scripts/scorecard_db.py', 'impact', '--since', 'origin/dev'], cwd=ROOT, check=False)
    check_target_footprint(prune=args.prune_target, max_age_days=args.prune_days)

if __name__ == '__main__': main()
