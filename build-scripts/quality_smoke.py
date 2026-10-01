#!/usr/bin/env python3
"""Bounded, synthetic CLI smoke/benchmark; all outputs live in a temporary directory."""
import argparse
import json
import os
import platform
import shutil
import sqlite3
import statistics
import subprocess
import tempfile
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def run(command, timeout=120):
    start = time.perf_counter()
    result = subprocess.run([str(x) for x in command], capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{command[0]} exited {result.returncode}: {result.stderr[-2000:]}')
    return time.perf_counter() - start

def validate_package(path):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None, 'Corrupt ZIP entry'
        media = json.loads(archive.read('media'))
        assert media, 'Empty media manifest'
        for entry in media:
            assert archive.getinfo(entry).file_size > 0, 'Empty media output'
        with tempfile.TemporaryDirectory() as dbdir:
            db = Path(dbdir) / 'collection.anki2'
            db.write_bytes(archive.read('collection.anki2'))
            with sqlite3.connect(db) as connection:
                assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
                count = connection.execute('SELECT count(*) FROM notes').fetchone()[0]
                assert count == 10, f'Expected 10 notes, got {count}'
    return {'notes': count, 'media_files': len(media)}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--profile', choices=['debug', 'release'], default='debug')
    args = parser.parse_args()
    if not 1 <= args.samples <= 10:
        parser.error('--samples must be between 1 and 10')
    binary = ROOT / 'target' / args.profile / ('srt-flashcards.exe' if os.name == 'nt' else 'srt-flashcards')
    if not binary.exists():
        parser.error(f'Build first: cargo build -p srt-flashcards-cli {"--release" if args.profile == "release" else ""}')
    for tool in ('ffmpeg', 'ffprobe'):
        if not shutil.which(tool):
            parser.error(f'{tool} is required; media tests must not silently skip')
    timings = {'preview': [], 'apkg_media': []}
    with tempfile.TemporaryDirectory(prefix='vesta-quality-') as directory:
        temp = Path(directory)
        subtitles = temp / 'input.srt'
        subtitles.write_text(''.join(
            f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nUnique dialogue number {i+1}.\n\n'
            for i in range(10)), encoding='utf-8')
        video = temp / 'fixture.mp4'
        run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i',
             'testsrc2=size=320x240:rate=10:duration=20', '-f', 'lavfi', '-i',
             'sine=frequency=440:sample_rate=16000:duration=20', '-c:v', 'libx264',
             '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest', video])
        for sample in range(args.samples):
            common = ['--target', subtitles, '--video', video, '--output', temp / f'run-{sample}',
                      '--deck', 'QualitySmoke', '--jobs', '2', '--no-optimize', '--no-video']
            timings['preview'].append(run([binary, 'preview', *common]))
            timings['apkg_media'].append(run([binary, 'generate', *common, '--format', 'apkg']))
            packages = list((temp / f'run-{sample}').rglob('*.apkg'))
            assert len(packages) == 1, f'Expected one package, got {packages}'
            contents = validate_package(packages[0])
    print(json.dumps({'platform': platform.platform(), 'cpu_count': os.cpu_count(),
        'profile': args.profile, 'samples': args.samples, 'correctness': contents,
        'seconds': {name: {'median': statistics.median(values), 'min': min(values),
                          'max': max(values), 'runs': values} for name, values in timings.items()}}, indent=2))

if __name__ == '__main__':
    main()
