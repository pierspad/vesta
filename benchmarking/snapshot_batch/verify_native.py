#!/usr/bin/env python3
"""Offline benchmark of the native Rust engine; no media optimization in Python."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sqlite3
import statistics
import subprocess
import sys
import time
import zipfile
from datetime import datetime

ROOT = Path(__file__).resolve().parents[2]
VIDEO_EXTENSIONS = {'.mp4', '.mkv', '.mov', '.avi', '.webm', '.m4v'}
PROFILES = {'default': (256, 144), 'large': (640, 360)}
try:
    import psutil
except ImportError:
    psutil = None


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def atomic_json(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
    temporary.replace(path)


def milliseconds(value):
    h, m, s, ms = map(int, re.split('[:,.]', value))
    return ((h * 60 + m) * 60 + s) * 1000 + ms


def subtitles(path):
    entries = []
    text = path.read_text(encoding='utf-8-sig').replace('\r\n', '\n')
    for block in re.split(r'\n\s*\n', text.strip()):
        lines = block.splitlines()
        for index, line in enumerate(lines):
            match = re.match(r'(\d+:\d{2}:\d{2}[,.]\d{3})\s*-->\s*(\d+:\d{2}:\d{2}[,.]\d{3})', line)
            if match:
                start, end = map(milliseconds, match.groups())
                content = '\n'.join(lines[index + 1:]).strip()
                if start >= 0 and end > start and content:
                    entries.append((start, end, content))
                break
    if not entries:
        raise ValueError(f'No usable subtitles: {path}')
    return entries


def discover(directory, language):
    films, skipped = [], []
    for video in sorted(directory.rglob('*')):
        if video.suffix.lower() not in VIDEO_EXTENSIONS or not video.is_file():
            continue
        stems = [video.stem, re.sub(r'\s*\(\d{4}\)$', '', video.stem)]
        candidates = [video.with_name(f'{stem}-{language}.srt') for stem in dict.fromkeys(stems)]
        candidates += [video.with_suffix('.srt')]
        subtitle = next((path for path in candidates if path.is_file()), None)
        if subtitle is None:
            skipped.append({'video': str(video), 'reason': 'No matching subtitles', 'expected': [str(p) for p in candidates]})
            continue
        films.append({'video': str(video), 'subtitle': str(subtitle), 'entries': subtitles(subtitle)})
    return films, skipped


def make_cases(films, sizes, profiles, sparse):
    cases = []
    for film in films:
        entries = film['entries']
        selections = []
        seen = set()
        for size in sizes:
            count = len(entries) if size == 'all' else min(int(size), len(entries))
            if count in seen:
                continue
            seen.add(count)
            selections.append(('nearby' if count < len(entries) else 'full', entries[:count]))
        if sparse and len(entries) > 1:
            count = min(sparse, len(entries))
            indices = [round(i * (len(entries) - 1) / (count - 1)) for i in range(count)] if count > 1 else [0]
            selections.append(('sparse', [entries[i] for i in indices]))
        for distribution, chosen in selections:
            for profile in profiles:
                identity = f"{film['video']}|{len(chosen)}|{distribution}|{profile}"
                cases.append({'id': hashlib.sha256(identity.encode()).hexdigest()[:16], 'video': film['video'],
                              'subtitle': film['subtitle'], 'cards': len(chosen), 'distribution': distribution,
                              'profile': profile, 'entries': chosen})
    return cases


def write_srt(path, entries):
    def timestamp(value):
        return f'{value // 3600000:02}:{value // 60000 % 60:02}:{value // 1000 % 60:02},{value % 1000:03}'
    path.write_text(''.join(f'{i + 1}\n{timestamp(a)} --> {timestamp(b)}\n{text}\n\n'
                            for i, (a, b, text) in enumerate(entries)), encoding='utf-8')


def validate(package, count, folder):
    with zipfile.ZipFile(package) as archive:
        if archive.testzip() is not None:
            raise ValueError('ZIP integrity failure')
        manifest = json.loads(archive.read('media'))
        if len(manifest) != 2 * count or len(set(manifest.values())) != 2 * count:
            raise ValueError('Unexpected media count or duplicate names')
        fingerprints = {}
        for key, name in manifest.items():
            with archive.open(key) as stream:
                digest, size = hashlib.sha256(), 0
                for block in iter(lambda: stream.read(1024 * 1024), b''):
                    digest.update(block)
                    size += len(block)
            if not size:
                raise ValueError(f'Empty media: {name}')
            fingerprints[name] = digest.hexdigest()
        db = folder / 'check.sqlite'
        db.write_bytes(archive.read('collection.anki2'))
    with sqlite3.connect(db) as connection:
        if connection.execute('pragma integrity_check').fetchone()[0] != 'ok':
            raise ValueError('SQLite integrity failure')
        notes = connection.execute('select flds from notes order by flds').fetchall()
        if len(notes) != count:
            raise ValueError(f'Expected {count} notes, got {len(notes)}')
        model = next(iter(json.loads(connection.execute('select models from col').fetchone()[0]).values()))
        schema = [field['name'] for field in model['flds']]
        refs = set()
        for (fields,) in notes:
            if len(fields.split('\x1f')) != len(schema):
                raise ValueError('Unexpected note fields')
            refs.update(a or b for a, b in re.findall(r'\[sound:([^\]]+)\]|src="([^"]+)"', fields))
        if refs != set(fingerprints):
            raise ValueError('Unresolved or unused media references')
        note_hash = hashlib.sha256(json.dumps([schema, notes], ensure_ascii=False).encode()).hexdigest()
    db.unlink()
    return {'media': fingerprints, 'notes_sha256': note_hash}


def terminate(process):
    if process.poll() is not None:
        return
    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()


def execute(command, folder, timeout):
    start, peak, seen, status = time.monotonic(), 0, {}, 'completed'
    # Files avoid blocking on full stdout/stderr pipes during long film runs.
    with (folder / 'stdout.log').open('wb') as stdout, (folder / 'stderr.log').open('wb') as stderr:
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            while process.poll() is None:
                if psutil:
                    try:
                        parent = psutil.Process(process.pid)
                        rss = 0
                        for child in [parent] + parent.children(recursive=True):
                            try:
                                rss += child.memory_info().rss
                                seen[(child.pid, child.create_time())] = child.name()
                            except psutil.Error:
                                pass
                        peak = max(peak, rss)
                    except psutil.Error:
                        pass
                if timeout and time.monotonic() - start >= timeout:
                    status = 'timeout'
                    terminate(process)
                    break
                time.sleep(.02)
        finally:
            terminate(process)
    return {'seconds': time.monotonic() - start, 'status': status, 'returncode': process.returncode,
            'peak_tree_rss_bytes': peak if psutil else None,
            'sampled_ffmpeg_processes': sum(name == 'ffmpeg' for name in seen.values()) if psutil else None,
            'sampled_ffprobe_processes': sum(name == 'ffprobe' for name in seen.values()) if psutil else None}


def summary(output, data):
    lines = ['# Vesta — native Rust benchmark', '', f"Status: **{data['status']}**. Only validated pairs are compared.", '',
             '| Film | Cards | Distribution | Profile | Pairs | Baseline s | Optimized s | Time saved |',
             '|---|---:|---|---|---:|---:|---:|---:|']
    for case in data['cases']:
        pairs = []
        for repetition in range(data['settings']['repetitions']):
            rows = [r for r in data['runs'] if r['case'] == case['id'] and r['rep'] == repetition]
            if len(rows) == 2 and all(r.get('validated') for r in rows):
                pairs.append({r['mode']: r['seconds'] for r in rows})
        if pairs:
            a, b = [statistics.median(pair[mode] for pair in pairs) for mode in ('baseline', 'optimized')]
            film = Path(case['video']).name.replace('|', '\\|')
            lines.append(f"| {film} | {case['cards']} | {case['distribution']} | {case['profile']} | {len(pairs)} | {a:.3f} | {b:.3f} | {(1 - b/a)*100:.1f}% |")
    lines += ['', 'Positive percentages mean less wall time; negative percentages mean slower.',
              'Each valid pair has byte-identical MP3/WebP and identical note fields. Full pipeline time includes probing, extraction and packaging.',
              'OS cache is uncontrolled. A/B order alternates. Process/RSS measurements are sampled and optional.',
              'This measures audio + snapshots, not video clips or unrelated app operations. Incomplete pairs cannot establish a speedup.', '', '## Skipped inputs', '']
    lines += [f"- {item['video']}: {item['reason']}" for item in data['skipped']] or ['None.']
    (output / 'summary.md').write_text('\n'.join(lines) + '\n')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=ROOT / 'Test_Subs/FILM')
    parser.add_argument('--language', default='en')
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/srt-flashcards')
    parser.add_argument('--output', type=Path, default=ROOT.parent / 'vesta-benchmark-results' / datetime.now().strftime('%Y%m%d-%H%M%S'))
    parser.add_argument('--sizes', default='25,100,300,all', help='Comma-separated card counts and/or all')
    parser.add_argument('--profiles', default='default,large', help='default=256x144, large=640x360')
    parser.add_argument('--sparse', type=int, default=100, help='Additional sparse cards per film; 0 disables')
    parser.add_argument('--repetitions', type=int, default=3)
    parser.add_argument('--jobs', type=int, default=15)
    parser.add_argument('--timeout', type=float, default=7200, help='Seconds per generation; 0 disables')
    parser.add_argument('--keep-decks', action='store_true')
    parser.add_argument('--resume', action='store_true', help='Resume interrupted results in --output, same binary/inputs/settings required')
    parser.add_argument('--dry-run', action='store_true', help='Print discovered matrix without building or generating')
    args = parser.parse_args()
    args.input, args.binary, args.output = args.input.resolve(), args.binary.resolve(), args.output.resolve()
    args.sizes, args.profiles = args.sizes.split(','), args.profiles.split(',')
    if args.repetitions < 1 or args.jobs < 1 or args.timeout < 0 or args.sparse < 0:
        parser.error('Invalid repetitions, jobs, timeout or sparse count')
    if not args.sizes or any(size != 'all' and (not size.isdigit() or int(size) < 1) for size in args.sizes):
        parser.error('Sizes must be positive integers or all')
    if not args.profiles or any(profile not in PROFILES for profile in args.profiles):
        parser.error('Profiles must be default and/or large')
    args.profiles = list(dict.fromkeys(args.profiles))
    return args


def main():
    args = arguments()
    films, skipped = discover(args.input, args.language)
    cases = make_cases(films, args.sizes, args.profiles, args.sparse)
    for item in skipped:
        print(f"SKIP {item['video']}: {item['reason']}", flush=True)
    if not cases:
        raise ValueError('No paired videos/subtitles found')
    print(f'{len(films)} films; {len(cases)} cases; {len(cases)*args.repetitions*2} native generations. Results: {args.output}', flush=True)
    for case in cases:
        print(f"  {Path(case['video']).name}: {case['cards']} cards, {case['distribution']}, {case['profile']}", flush=True)
    if args.dry_run:
        return 0
    if not args.binary.is_file():
        raise ValueError(f'Missing binary: {args.binary}; use run-benchmark.sh to build it')
    for tool in ('ffmpeg', 'ffprobe'):
        if not shutil.which(tool):
            raise ValueError(f'Missing executable: {tool}')
    settings = {'sizes': args.sizes, 'profiles': args.profiles, 'sparse': args.sparse, 'repetitions': args.repetitions,
                'jobs': args.jobs, 'timeout': args.timeout, 'keep_decks': args.keep_decks, 'normalize_audio': True,
                'audio': 'MP3 128k/44100/stereo/track0', 'snapshot': 'WebP quality80/crop0/padding0', 'video_clips': False}
    public_cases = [{key: value for key, value in case.items() if key != 'entries'} for case in cases]
    inputs = [{'video': film['video'], 'video_size': Path(film['video']).stat().st_size,
               'video_mtime_ns': Path(film['video']).stat().st_mtime_ns, 'subtitle': film['subtitle'],
               'subtitle_sha256': sha(Path(film['subtitle']))} for film in films]
    signature = {'runner_sha256': sha(Path(__file__)), 'binary_sha256': sha(args.binary), 'settings': settings, 'inputs': inputs, 'cases': public_cases,
                 'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0],
                 'ffprobe': subprocess.check_output(['ffprobe', '-version'], text=True).splitlines()[0]}
    results = args.output / 'results.json'
    if args.resume:
        data = json.loads(results.read_text())
        if any(data.get(key) != value for key, value in signature.items()):
            raise ValueError('Cannot resume: binary, inputs, settings or tool versions changed')
        if any(run.get('status') == 'failed' for run in data['runs']):
            raise ValueError('Failed validation exists; inspect logs and use a new output directory')
    else:
        args.output.mkdir(parents=True, exist_ok=False)
        data = {**signature, 'status': 'running', 'skipped': skipped, 'runs': [], 'created': datetime.now().isoformat(),
                'platform': sys.platform, 'logical_cpus': os.cpu_count(), 'memory_sampling': bool(psutil),
                'sample_interval_seconds': .02, 'cache': 'OS cache uncontrolled; no persistent optimization cache'}
    def save():
        atomic_json(results, data)
        summary(args.output, data)
    data['status'] = 'running'
    save()
    try:
        for case in cases:
            case_dir = args.output / case['id']
            case_dir.mkdir(exist_ok=True)
            srt = case_dir / 'subset.srt'
            write_srt(srt, case['entries'])
            for rep in range(args.repetitions):
                for mode in (('baseline', 'optimized') if rep % 2 == 0 else ('optimized', 'baseline')):
                    if any(r['case'] == case['id'] and r['rep'] == rep and r['mode'] == mode and r.get('validated') for r in data['runs']):
                        continue
                    folder = case_dir / f'{rep}-{mode}'
                    folder.mkdir(exist_ok=True)
                    deck = folder / 'deck'
                    if deck.exists():
                        shutil.rmtree(deck)
                    width, height = PROFILES[case['profile']]
                    command = [str(args.binary), 'generate', '--target', str(srt), '--video', case['video'], '--output', str(deck),
                               '--deck', 'ScaleProbe', '--format', 'apkg', '--jobs', str(args.jobs), '--no-video', '--normalize-audio',
                               '--audio-track', '0', '--audio-format', 'mp3', '--audio-bitrate', '128', '--snapshot-format', 'webp',
                               '--snapshot-width', str(width), '--snapshot-height', str(height), '--snapshot-quality', '80', '--crop-bottom', '0']
                    if mode == 'baseline':
                        command.append('--no-optimize')
                    print(f"RUN {Path(case['video']).name} {case['cards']} {case['distribution']} {case['profile']} {rep+1}/{args.repetitions} {mode}", flush=True)
                    run = {'case': case['id'], 'rep': rep, 'mode': mode, 'command': command, 'logs': str(folder), 'validated': False}
                    data['active'] = run
                    save()
                    run.update(execute(command, folder, args.timeout))
                    data['runs'].append(run)
                    data.pop('active', None)
                    try:
                        if run['returncode'] or run['status'] != 'completed':
                            raise ValueError(f"Generation {run['status']}, exit {run['returncode']}")
                        packages = list(deck.rglob('*.apkg'))
                        if len(packages) != 1:
                            raise ValueError('Expected one APKG')
                        run['fingerprints'] = validate(packages[0], case['cards'], folder)
                        reference = next((r for r in data['runs'] if r['case'] == case['id'] and r.get('validated')), None)
                        if reference and run['fingerprints'] != reference['fingerprints']:
                            expected, actual = reference['fingerprints']['media'], run['fingerprints']['media']
                            run['different_media'] = [name for name in sorted(set(expected) | set(actual)) if expected.get(name) != actual.get(name)]
                            raise ValueError('Media or note fields differ from reference')
                        run['validated'] = True
                    except Exception as error:
                        run['status'], run['error'] = 'failed', str(error)
                        raise
                    finally:
                        save()
                    if not args.keep_decks:
                        shutil.rmtree(deck)
                    print(f"  OK {run['seconds']:.3f}s; exact media and note validation", flush=True)
        data['status'] = 'complete'
        save()
        print(f"Complete: {args.output / 'summary.md'}", flush=True)
        return 0
    except KeyboardInterrupt:
        data['status'] = 'interrupted'
        save()
        print(f'Interrupted; resume with --resume --output {args.output}', file=sys.stderr)
        return 130
    except Exception as error:
        data['status'], data['error'] = 'failed', str(error)
        save()
        raise


def interrupted(*_):
    raise KeyboardInterrupt()


if __name__ == '__main__':
    # SIGTERM follows the same child cleanup and result persistence as Ctrl+C.
    signal.signal(signal.SIGTERM, interrupted)
    try:
        sys.exit(main())
    except Exception as error:
        print(f'ERROR: {error}', file=sys.stderr)
        sys.exit(1)
