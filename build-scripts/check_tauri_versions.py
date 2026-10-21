#!/usr/bin/env python3
"""Validate locked Tauri JS/Rust pairs without network or installed packages."""
import json
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def check_pairs(cargo, npm):
    errors = []
    packages = npm['packages']
    pairs = {'@tauri-apps/api': 'tauri'}
    for path in packages:
        if path.startswith('node_modules/@tauri-apps/plugin-'):
            name = path.removeprefix('node_modules/')
            pairs[name] = name.replace('@tauri-apps/plugin-', 'tauri-plugin-')
    for js, rust in pairs.items():
        js_versions = {p['version'] for path, p in packages.items()
                       if path.endswith('node_modules/' + js)}
        rust_versions = {p['version'] for p in cargo['package'] if p['name'] == rust}
        if len(js_versions) != 1 or len(rust_versions) != 1:
            errors.append(f'{js} / {rust}: expected one version each; JS={sorted(js_versions)}, Rust={sorted(rust_versions)}')
            continue
        j, r = next(iter(js_versions)), next(iter(rust_versions))
        # Core shares a minor; plugin APIs can change in patch releases.
        compatible = j.split('.')[:2] == r.split('.')[:2] if rust == 'tauri' else j == r
        if not compatible:
            errors.append(f'{js} {j} / {rust} {r}: incompatible locked versions')
    return errors


def main():
    with (ROOT / 'Cargo.lock').open('rb') as handle:
        cargo = tomllib.load(handle)
    npm = json.loads((ROOT / 'apps/srt-gui/package-lock.json').read_text())
    errors = check_pairs(cargo, npm)
    if errors:
        print('\n'.join(errors), file=sys.stderr)
        return 1
    print('Locked Tauri JS/Rust pairs are compatible.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
