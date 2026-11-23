#!/usr/bin/env python3
"""Bounded, synthetic CLI/MCP smoke & benchmark; all outputs live in a temporary directory."""
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

EXPECTED_MCP_TOOLS = {
    'get_configuration',
    'inspect_subtitles',
    'preview_flashcards',
    'generate_flashcards',
    'translate_subtitles',
    'transcribe_media',
    'list_embedded_subtitles',
    'ocr_subtitles',
    'extract_embedded_subtitle',
    'sync_subtitles',
    'autosync_subtitles',
    'refine_flashcards',
    'export_dueling_subtitles',
    'merge_apkg',
    'get_job',
    'cancel_job',
    'list_plugins',
    'call_plugin',
}


def run(command, timeout=120):
    start = time.perf_counter()
    result = subprocess.run([str(x) for x in command], capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{command[0]} exited {result.returncode}: {result.stderr[-2000:]}')
    return time.perf_counter() - start


def run_capture(command, timeout=120):
    result = subprocess.run([str(x) for x in command], capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{command[0]} exited {result.returncode}: {result.stderr[-2000:]}')
    return result.stdout


def validate_package(path, expected_notes=10, require_media=True):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None, 'Corrupt ZIP entry'
        media = json.loads(archive.read('media'))
        if require_media:
            assert media, 'Empty media manifest'
        for entry in media:
            assert archive.getinfo(entry).file_size > 0, 'Empty media output'
        with tempfile.TemporaryDirectory() as dbdir:
            db = Path(dbdir) / 'collection.anki2'
            db.write_bytes(archive.read('collection.anki2'))
            with sqlite3.connect(db) as connection:
                assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
                count = connection.execute('SELECT count(*) FROM notes').fetchone()[0]
                assert count == expected_notes, f'Expected {expected_notes} notes, got {count}'
    return {'notes': count, 'media_files': len(media)}


def run_mcp_smoke(cli_bin, read_root, output_root, inputs):
    """Verify stdio JSON-RPC lifecycle, tool discovery (18 tools), and real engine calls."""
    proc = subprocess.Popen(
        [str(cli_bin), 'mcp', '--read-root', str(read_root), '--output-root', str(output_root)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    next_id = 0

    def send(payload):
        assert proc.stdin is not None
        proc.stdin.write(json.dumps(payload) + '\n')
        proc.stdin.flush()

    def rpc(method, params):
        nonlocal next_id
        next_id += 1
        req_id = next_id
        send({'jsonrpc': '2.0', 'id': req_id, 'method': method, 'params': params})
        assert proc.stdout is not None
        while True:
            line = proc.stdout.readline()
            if not line:
                stderr = proc.stderr.read() if proc.stderr else ''
                raise RuntimeError(f'vesta-cli mcp EOF waiting for {method}: {stderr[-1000:]}')
            msg = json.loads(line)
            if msg.get('id') == req_id:
                if 'error' in msg:
                    raise RuntimeError(f'MCP error on {method}: {msg["error"]}')
                return msg['result']

    def call_tool(name, arguments):
        res = rpc('tools/call', {'name': name, 'arguments': arguments})
        assert not res.get('isError'), f'{name} returned tool error: {res}'
        text = ''.join(part.get('text', '') for part in res.get('content', []) if part.get('type') == 'text')
        return json.loads(text) if text else res

    try:
        init = rpc('initialize', {
            'protocolVersion': '2025-03-26',
            'capabilities': {},
            'clientInfo': {'name': 'vesta-quality-smoke', 'version': '1.0'},
        })
        assert init.get('serverInfo', {}).get('name'), f'Missing serverInfo: {init}'
        send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})

        tools_res = rpc('tools/list', {})
        tool_names = {t['name'] for t in tools_res.get('tools', [])}
        assert tool_names == EXPECTED_MCP_TOOLS, f'Tool mismatch: missing={EXPECTED_MCP_TOOLS - tool_names}, extra={tool_names - EXPECTED_MCP_TOOLS}'

        cfg = call_tool('get_configuration', {})
        assert cfg.get('read_roots') and cfg.get('output_root'), cfg

        inspected = call_tool('inspect_subtitles', {'path': str(inputs['subtitles'])})
        assert inspected.get('count') == 10, inspected

        preview = call_tool('preview_flashcards', {
            'target': str(inputs['subtitles']),
            'native': str(inputs['native']),
        })
        assert preview.get('active') == 10 and preview.get('total') == 10 and len(preview.get('rows', [])) == 10, preview

        tracks = call_tool('list_embedded_subtitles', {'input': str(inputs['mkv'])})
        assert tracks.get('tracks') and len(tracks['tracks']) >= 1, tracks
        track_idx = tracks['tracks'][0]['index']
        extracted = call_tool('extract_embedded_subtitle', {
            'input': str(inputs['mkv']),
            'track_index': track_idx,
        })
        assert Path(extracted['output_path']).read_text(encoding='utf-8').find('Unique dialogue number 1.') >= 0, extracted

        dueling = call_tool('export_dueling_subtitles', {
            'target': str(inputs['subtitles']),
            'native': str(inputs['native']),
            'format': 'ass',
        })
        assert Path(dueling['output_path']).exists() and dueling.get('line_count') == 10, dueling

        merged = call_tool('merge_apkg', {'inputs': [str(inputs['apkg_a']), str(inputs['apkg_b'])]})
        merged_stats = validate_package(Path(merged['output_path']), expected_notes=20, require_media=False)
        assert merged_stats['notes'] == 20, merged_stats

        synced = call_tool('sync_subtitles', {
            'input': str(inputs['shifted']),
            'anchors': [{'subtitle_id': 1, 'corrected_time_ms': 0}, {'subtitle_id': 10, 'corrected_time_ms': 18000}],
        })
        assert '00:00:00,000 -->' in Path(synced['output_path']).read_text(encoding='utf-8'), synced
    finally:
        if proc.stdin:
            proc.stdin.close()
        proc.wait(timeout=15)

    return {'tools_verified': len(EXPECTED_MCP_TOOLS), 'mcp_calls_ok': 8}


def run_suite_checks(cli_bin, fc_bin, temp, inputs, package_a, package_b):
    """Run offline synthetic smoke tests across unified CLI, flashcards, extract, sync, refine, and MCP."""
    checks = {}

    # EX-01 / EX-02: vesta-cli extract (--list-tracks & --track) on muxed MKV
    tracks_json = json.loads(run_capture([cli_bin, 'extract', '-i', inputs['mkv'], '--list-tracks']))
    assert tracks_json and tracks_json[0]['codec'].lower() in {'subrip', 'srt'}, tracks_json
    extracted_srt = temp / 'cli_extracted.srt'
    run([cli_bin, 'extract', '-i', inputs['mkv'], '--track', str(tracks_json[0]['index']), '-o', extracted_srt])
    assert 'Unique dialogue number 10.' in extracted_srt.read_text(encoding='utf-8')
    checks['extract_embedded'] = 'ok'

    # SY-01: vesta-cli sync manual anchors
    synced_srt = temp / 'cli_synced.srt'
    run([cli_bin, 'sync', '-i', inputs['shifted'], '-o', synced_srt, '--anchor', '1:0', '--anchor', '10:18000'])
    synced_text = synced_srt.read_text(encoding='utf-8')
    assert '00:00:00,000 -->' in synced_text and '00:00:18,000 -->' in synced_text
    checks['sync_anchors'] = 'ok'

    # FC-01 TSV + FC-06 dueling (.ass / .srt) + FC-03 merge
    tsv_dir = temp / 'tsv-deck'
    run([fc_bin, 'generate', '--target', inputs['subtitles'], '--output', tsv_dir, '--deck', 'TsvSmoke', '--no-audio', '--no-snapshots', '--no-video'])
    tsv_files = list(tsv_dir.rglob('*.tsv'))
    assert len(tsv_files) == 1 and 'Unique dialogue number 1.' in tsv_files[0].read_text(encoding='utf-8')
    ass_out = temp / 'bilingual.ass'
    srt_out = temp / 'bilingual.srt'
    run([fc_bin, 'dueling', '--target', inputs['subtitles'], '--native', inputs['native'], '-o', ass_out])
    run([fc_bin, 'dueling', '--target', inputs['subtitles'], '--native', inputs['native'], '--dueling-format', 'srt', '-o', srt_out])
    assert '[Events]' in ass_out.read_text(encoding='utf-8') and 'Native line 1.' in srt_out.read_text(encoding='utf-8')
    merged_apkg = temp / 'merged.apkg'
    run([fc_bin, 'merge', package_a, package_b, '--output', merged_apkg])
    validate_package(merged_apkg, expected_notes=20, require_media=False)
    checks['flashcards_cli_suite'] = 'ok'

    # RF offline inspection via vesta-cli refine inspect
    cards = json.loads(run_capture([cli_bin, 'refine', 'inspect', '--input', merged_apkg]))
    assert isinstance(cards, list) and len(cards) == 20, f'Expected 20 refined cards, got {len(cards)}'
    checks['refine_inspect'] = len(cards)

    # CM-01 / CM-02: vesta-cli mcp live stdio server verification
    mcp_out = temp / 'mcp-out'
    mcp_out.mkdir(exist_ok=True)
    checks['mcp'] = run_mcp_smoke(cli_bin, temp, mcp_out, {**inputs, 'apkg_a': package_a, 'apkg_b': package_b})
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--profile', choices=['debug', 'release'], default='debug')
    args = parser.parse_args()
    if not 1 <= args.samples <= 10:
        parser.error('--samples must be between 1 and 10')
    fc_bin = ROOT / 'target' / args.profile / ('vesta-flashcards.exe' if os.name == 'nt' else 'vesta-flashcards')
    cli_bin = ROOT / 'target' / args.profile / ('vesta-cli.exe' if os.name == 'nt' else 'vesta-cli')
    for required, pkg in ((fc_bin, 'vesta-flashcards-cli'), (cli_bin, 'vesta-cli')):
        if not required.exists():
            parser.error(f'Build first: cargo build -p {pkg} {"--release" if args.profile == "release" else ""}')
    for tool in ('ffmpeg', 'ffprobe'):
        if not shutil.which(tool):
            parser.error(f'{tool} is required; media tests must not silently skip')

    timings = {'preview': [], 'apkg_media': []}
    with tempfile.TemporaryDirectory(prefix='vesta-quality-') as directory:
        temp = Path(directory)
        subtitles = temp / 'input.srt'
        native = temp / 'native.srt'
        shifted = temp / 'shifted.srt'
        subtitles.write_text(''.join(
            f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nUnique dialogue number {i+1}.\n\n'
            for i in range(10)), encoding='utf-8')
        native.write_text(''.join(
            f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nNative line {i+1}.\n\n'
            for i in range(10)), encoding='utf-8')
        shifted.write_text(''.join(
            f'{i+1}\n00:00:{i*2+5:02},000 --> 00:00:{i*2+6:02},000\nUnique dialogue number {i+1}.\n\n'
            for i in range(10)), encoding='utf-8')

        video = temp / 'fixture.mp4'
        run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i',
             'testsrc2=size=320x240:rate=10:duration=20', '-f', 'lavfi', '-i',
             'sine=frequency=440:sample_rate=16000:duration=20', '-c:v', 'libx264',
             '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest', video])
        mkv = temp / 'with_subs.mkv'
        run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-i', video, '-i', subtitles,
             '-c', 'copy', '-metadata:s:s:0', 'language=eng', mkv])

        apkg_outputs = []
        for sample in range(args.samples):
            common = ['--target', subtitles, '--video', video, '--output', temp / f'run-{sample}',
                      '--deck', f'QualitySmoke-{sample}', '--jobs', '2', '--no-optimize', '--no-video']
            timings['preview'].append(run([fc_bin, 'preview', *common]))
            timings['apkg_media'].append(run([fc_bin, 'generate', *common, '--format', 'apkg']))
            packages = list((temp / f'run-{sample}').rglob('*.apkg'))
            assert len(packages) == 1, f'Expected one package, got {packages}'
            contents = validate_package(packages[0])
            apkg_outputs.append(packages[0])

        # Generate a second distinct non-overlapping APKG for clean merge testing if needed
        alt_dir = temp / 'alt-merge-src'
        alt_subs = temp / 'alt.srt'
        alt_subs.write_text(''.join(
            f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nSecond deck cue {i+1}.\n\n'
            for i in range(10)), encoding='utf-8')
        run([fc_bin, 'generate', '--target', alt_subs, '--video', video, '--output', alt_dir,
             '--deck', 'QualitySmokeAlt', '--jobs', '2', '--no-optimize', '--no-video', '--format', 'apkg'])
        alt_package = next(alt_dir.rglob('*.apkg'))

        smoke_checks = run_suite_checks(
            cli_bin,
            fc_bin,
            temp,
            {'subtitles': subtitles, 'native': native, 'shifted': shifted, 'video': video, 'mkv': mkv},
            apkg_outputs[0],
            alt_package,
        )

    print(json.dumps({'platform': platform.platform(), 'cpu_count': os.cpu_count(),
        'profile': args.profile, 'samples': args.samples, 'correctness': contents,
        'smoke_checks': smoke_checks,
        'seconds': {name: {'median': statistics.median(values), 'min': min(values),
                          'max': max(values), 'runs': values} for name, values in timings.items()}}, indent=2))


if __name__ == '__main__':
    main()
