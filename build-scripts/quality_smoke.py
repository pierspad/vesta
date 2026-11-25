#!/usr/bin/env python3
"""Bounded, synthetic CLI/MCP smoke & benchmark; all outputs live in a temporary directory."""
import argparse
import http.server
import json
import os
import platform
import re
import shutil
import sqlite3
import statistics
import struct
import subprocess
import tempfile
import threading
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

EXPECTED_MCP_TOOLS = {
    'get_configuration',
    'inspect_subtitles',
    'preview_flashcards',
    'generate_flashcards',
    'batch_generate_flashcards',
    'translate_subtitles',
    'transcribe_media',
    'list_embedded_subtitles',
    'preview_video_frame',
    'ocr_subtitles',
    'extract_embedded_subtitle',
    'sync_subtitles',
    'autosync_subtitles',
    'inspect_deck',
    'edit_deck_cards',
    'refine_flashcards',
    'ankiconnect',
    'export_dueling_subtitles',
    'merge_apkg',
    'manage_local_models',
    'get_job',
    'cancel_job',
    'list_plugins',
    'call_plugin',
}


class MockLoopbackHandler(http.server.BaseHTTPRequestHandler):
    """Offline loopback HTTP fixture for LLM, Vision OCR, Cloud STT, and AnkiConnect."""

    def log_message(self, format, *args):  # noqa: A002
        return

    def _send_json(self, status, payload):
        body = json.dumps(payload).encode('utf-8')
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):  # noqa: N802
        length = int(self.headers.get('Content-Length') or 0)
        raw = self.rfile.read(length) if length > 0 else b''

        if self.path.startswith('/v1/audio/transcriptions'):
            segments = [
                {
                    'id': i,
                    'start': round(i * 2.0, 2),
                    'end': round(i * 2.0 + 1.0, 2),
                    'text': f'Unique dialogue number {i + 1}.',
                }
                for i in range(10)
            ]
            words = [{'word': 'Unique', 'start': 0.0, 'end': 0.3}]
            self._send_json(
                200,
                {
                    'text': ' '.join(s['text'] for s in segments),
                    'segments': segments,
                    'words': words,
                },
            )
            return

        if self.path.startswith('/v1/chat/completions'):
            req = json.loads(raw.decode('utf-8'))
            model = str(req.get('model', ''))
            messages = req.get('messages', [])
            if model == 'tier0-fail':
                self._send_json(402, {'error': {'message': 'tier0 unavailable', 'type': 'quota'}})
                return

            # Vision OCR payload passes structured content list with image_url parts
            is_vision = any(isinstance(m.get('content'), list) for m in messages)
            if is_vision:
                self._send_json(
                    200,
                    {'choices': [{'message': {'role': 'assistant', 'content': 'Mock OCR subtitle'}}]},
                )
                return

            user_text = '\n'.join(
                m.get('content', '') for m in messages if isinstance(m.get('content'), str)
            )
            if model == 'refine-partial' and 'FAIL_CARD' in user_text:
                self._send_json(500, {'error': {'message': 'synthetic card annotation error'}})
                return
            if model.startswith('refine'):
                self._send_json(
                    200,
                    {'choices': [{'message': {'role': 'assistant', 'content': 'Concise note'}}]},
                )
                return

            if 'REPAIR task' in user_text or 'Original subtitle text to translate:' in user_text:
                tail = user_text.split('Original subtitle text to translate:', 1)[-1]
                nums = re.findall(r'\b(\d+)\b', tail)
                cue_label = nums[-1] if nums else '10'
                self._send_json(
                    200,
                    {'choices': [{'message': {'role': 'assistant', 'content': f'Translated cue {cue_label}.'}}]},
                )
                return

            input_segment = user_text.split('Input subtitles:', 1)[1] if 'Input subtitles:' in user_text else user_text
            cue_ids = []
            seen = set()
            for cid_str in re.findall(r'"id"\s*:\s*(\d+)', input_segment):
                cid = int(cid_str)
                if cid not in seen:
                    seen.add(cid)
                    cue_ids.append(cid)
            if not cue_ids:
                cue_ids = [1]
            # Drop the last cue from the second batch in translate-repair mode so REPAIR triggers deterministically
            if model == 'translate-repair' and 10 in cue_ids and len(cue_ids) > 1:
                cue_ids = [cid for cid in cue_ids if cid != 10]
            items = [{'id': cid, 'text': f'Translated cue {cid}.'} for cid in cue_ids]
            self._send_json(
                200,
                {'choices': [{'message': {'role': 'assistant', 'content': json.dumps(items)}}]},
            )
            return

        if self.path.startswith('/ankiconnect'):
            req = json.loads(raw.decode('utf-8'))
            action = req.get('action')
            params = req.get('params', {})
            if action == 'version':
                self._send_json(200, {'result': 6, 'error': None})
            elif action == 'deckNames':
                self._send_json(200, {'result': ['Default', 'QualitySmoke'], 'error': None})
            elif action == 'importPackage':
                pkg = Path(params.get('path', ''))
                self._send_json(200, {'result': pkg.is_file(), 'error': None})
            else:
                self._send_json(200, {'result': None, 'error': f'unsupported action {action}'})
            return

        self._send_json(404, {'error': f'unknown path {self.path}'})


def start_mock_server():
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), MockLoopbackHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def run(command, timeout=120, allow_nonzero=False):
    start = time.perf_counter()
    result = subprocess.run([str(x) for x in command], capture_output=True, text=True, timeout=timeout)
    if result.returncode and not allow_nonzero:
        raise RuntimeError(f'{command[0]} exited {result.returncode}: {result.stderr[-2000:]}')
    if allow_nonzero:
        return result
    return time.perf_counter() - start


def run_capture(command, timeout=120):
    result = subprocess.run([str(x) for x in command], capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{command[0]} exited {result.returncode}: {result.stderr[-2000:]}')
    return result.stdout


def pgs_segment(pts_ms, seg_type, payload):
    pts_90k = (pts_ms * 90_000) // 1000
    return struct.pack('>2sIIBH', b'PG', pts_90k, pts_90k, seg_type, len(payload)) + payload


def create_single_cue_pgs_mkv(temp_dir):
    pds = bytes([0, 0, 0, 16, 128, 128, 0, 1, 235, 128, 128, 255])
    rle = b''.join([bytes([0x00, 0x80 | 20, 1, 0x00, 0x00])] * 8)
    obj_len = 4 + len(rle)
    ods = bytes([
        0, 0, 0, 0xC0,
        (obj_len >> 16) & 0xFF,
        (obj_len >> 8) & 0xFF,
        obj_len & 0xFF,
    ]) + struct.pack('>HH', 20, 8) + rle
    pcs = (
        struct.pack('>HHBHB', 1280, 720, 0x10, 1, 0x80)
        + b'\x00\x00\x01\x00\x00\x00\x00'
        + struct.pack('>HH', 100, 600)
    )
    sup = (
        pgs_segment(1000, 0x16, pcs)
        + pgs_segment(1000, 0x14, pds)
        + pgs_segment(1000, 0x15, ods)
        + pgs_segment(1000, 0x80, b'')
    )
    clear_pcs = struct.pack('>HHBHB', 1280, 720, 0x10, 10, 0x00) + b'\x00\x00\x00'
    sup += pgs_segment(2500, 0x16, clear_pcs) + pgs_segment(2500, 0x80, b'')
    sup_path = temp_dir / 'cue.sup'
    sup_path.write_bytes(sup)
    pg_mkv = temp_dir / 'pg_sub.mkv'
    run([
        'ffmpeg', '-hide_banner', '-loglevel', 'error', '-nostdin', '-copyts',
        '-f', 'lavfi', '-i', 'color=size=1280x720:duration=4:rate=1',
        '-i', sup_path, '-map', '0:v', '-map', '1:s',
        '-c:v', 'libx264', '-preset', 'ultrafast', '-c:s', 'copy', pg_mkv,
    ])
    return pg_mkv


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


def run_mcp_smoke(cli_bin, read_root, output_root, inputs, anki_url, v1_url):
    """Verify stdio JSON-RPC lifecycle, tool discovery (24 tools), and end-to-end tool calls."""
    provider_cfg_path = read_root / 'mcp-provider-config.json'
    smoke_secret = 'vesta-smoke-secret-do-not-expose'
    provider_cfg_path.write_text(json.dumps({
        'translation': {
            'profiles': {
                'smoke-translation': {
                    'tiers': [[{
                        'provider': 'custom',
                        'model': 'translate-repair',
                        'api_key_env': 'VESTA_SMOKE_API_KEY',
                        'api_url': v1_url,
                        'rpm': 0,
                    }]],
                    'batch_size': 5,
                },
            },
        },
        'transcription': {
            'profiles': {
                'smoke-stt': {
                    'provider': 'custom',
                    'model': 'whisper-1',
                    'api_key_env': 'VESTA_SMOKE_API_KEY',
                    'api_url': v1_url,
                },
            },
        },
    }), encoding='utf-8')

    proc = subprocess.Popen(
        [
            str(cli_bin),
            'mcp',
            '--read-root',
            str(read_root),
            '--output-root',
            str(output_root),
            '--provider-config',
            str(provider_cfg_path),
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env={**os.environ, 'VESTA_SMOKE_API_KEY': smoke_secret},
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

    def wait_job(job_id):
        deadline = time.monotonic() + 30.0
        while time.monotonic() < deadline:
            state = call_tool('get_job', {'job_id': job_id})
            status = state.get('status')
            if status in {'succeeded', 'failed', 'cancelled'}:
                return state
            time.sleep(0.05)
        raise TimeoutError(f'Timed out waiting for MCP job {job_id}')

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
        assert tool_names == EXPECTED_MCP_TOOLS, (
            f'Tool mismatch: missing={EXPECTED_MCP_TOOLS - tool_names}, '
            f'extra={tool_names - EXPECTED_MCP_TOOLS}'
        )

        cfg = call_tool('get_configuration', {})
        assert cfg.get('read_roots') and cfg.get('output_root'), cfg
        assert cfg.get('translation_available') is True and cfg.get('transcription_available') is True, cfg
        assert 'smoke-translation' in cfg.get('enabled_translation_profiles', []), cfg
        assert 'smoke-stt' in cfg.get('enabled_transcription_profiles', []), cfg
        cfg_serialized = json.dumps(cfg)
        assert smoke_secret not in cfg_serialized and 'api_url' not in cfg_serialized, cfg
        assert 'VESTA_SMOKE_API_KEY' not in cfg_serialized, cfg

        inspected = call_tool('inspect_subtitles', {'path': str(inputs['subtitles'])})
        assert inspected.get('count') == 10, inspected

        preview = call_tool('preview_flashcards', {
            'target': str(inputs['subtitles']),
            'native': str(inputs['native']),
            'min_chars': 5,
            'only_cjk': False,
        })
        assert preview.get('active') == 10 and preview.get('total') == 10 and len(preview.get('rows', [])) == 10, preview

        frame = call_tool('preview_video_frame', {
            'input': str(inputs['video']),
            'time_ms': 1000,
            'roi': {'x': 0.08, 'y': 0.74, 'width': 0.84, 'height': 0.2},
        })
        assert frame.get('data_url', '').startswith('data:image/png;base64,') and frame.get('roi_px'), frame

        batch_job = call_tool('batch_generate_flashcards', {
            'episodes': [
                {'target': str(inputs['subtitles']), 'deck': 'BatchEp1'},
                {'target': str(inputs['alt_subs']), 'deck': 'BatchEp2'},
            ],
            'format': 'apkg',
            'merge_apkg': True,
        })
        batch_done = wait_job(batch_job['job_id'])
        assert batch_done.get('status') == 'succeeded', batch_done
        merged_batch = Path(batch_done['result']['merged_apkg_path'])
        validate_package(merged_batch, expected_notes=20, require_media=False)

        tracks = call_tool('list_embedded_subtitles', {'input': str(inputs['mkv'])})
        assert tracks.get('tracks') and len(tracks['tracks']) >= 1, tracks
        extracted = call_tool('extract_embedded_subtitle', {
            'input': str(inputs['mkv']),
            'track_index': tracks['tracks'][0]['index'],
        })
        assert 'Unique dialogue number 1.' in Path(extracted['output_path']).read_text(encoding='utf-8'), extracted

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

        autosync_job = call_tool('autosync_subtitles', {
            'input': str(inputs['shifted']),
            'media': str(inputs['video']),
            'profile_id': 'smoke-stt',
            'language': 'en',
            'quick': True,
            'max_offset_secs': 60,
        })
        autosync_done = wait_job(autosync_job['job_id'])
        assert autosync_done.get('status') == 'succeeded', autosync_done
        autosync_res = autosync_done.get('result', {})
        assert autosync_res.get('anchors_created', 0) >= 1, autosync_done
        assert '00:00:00,000 -->' in Path(autosync_res['output_path']).read_text(encoding='utf-8'), autosync_done
        assert smoke_secret not in json.dumps(autosync_done), autosync_done

        deck_inspected = call_tool('inspect_deck', {
            'input': str(inputs['apkg_a']),
            'query': 'dialogue number 3',
        })
        assert deck_inspected.get('matched') == 1 and len(deck_inspected.get('cards', [])) == 1, deck_inspected
        card_id = deck_inspected['cards'][0]['id']

        edited = call_tool('edit_deck_cards', {
            'input': str(inputs['apkg_a']),
            'updates': [{'id': card_id, 'notes': 'MCP edited note'}],
        })
        assert edited.get('updated_count') == 1 and Path(edited['output_path']).is_file(), edited

        ping_res = call_tool('ankiconnect', {'action': 'ping', 'url': anki_url})
        assert ping_res.get('api_version') == 6, ping_res
        decks_res = call_tool('ankiconnect', {'action': 'deck_names', 'url': anki_url})
        assert 'QualitySmoke' in decks_res.get('decks', []), decks_res
        import_res = call_tool('ankiconnect', {
            'action': 'import_package',
            'url': anki_url,
            'apkg_path': str(inputs['apkg_a']),
        })
        assert import_res.get('imported') is True, import_res

        models_res = call_tool('manage_local_models', {'action': 'list'})
        assert 'whisper' in models_res and 'vad' in models_res and 'ocr' in models_res, models_res
    finally:
        if proc.stdin:
            proc.stdin.close()
        proc.wait(timeout=15)

    return {'tools_verified': len(EXPECTED_MCP_TOOLS), 'mcp_calls_ok': 17}


def run_suite_checks(cli_bin, fc_bin, temp, inputs, package_a, package_b, base_url):
    """Run offline synthetic smoke tests across unified CLI, engines, mock HTTP providers, and MCP."""
    checks = {}
    v1_url = f'{base_url}/v1'
    anki_url = f'{base_url}/ankiconnect'

    # EX-01 / EX-02: vesta-cli extract (--format stats, --list-tracks & --track) on muxed MKV
    stats_out = run_capture([cli_bin, 'extract', '-i', inputs['subtitles'], '--format', 'stats'])
    assert '10' in stats_out, stats_out
    tracks_json = json.loads(run_capture([cli_bin, 'extract', '-i', inputs['mkv'], '--list-tracks']))
    assert tracks_json and tracks_json[0]['codec'].lower() in {'subrip', 'srt'}, tracks_json
    extracted_srt = temp / 'cli_extracted.srt'
    run([cli_bin, 'extract', '-i', inputs['mkv'], '--track', str(tracks_json[0]['index']), '-o', extracted_srt])
    assert 'Unique dialogue number 10.' in extracted_srt.read_text(encoding='utf-8')
    checks['extract_embedded'] = 'ok'

    # EX-03 / EX-04 / EX-05: Bitmap PGS OCR & hardcoded video ROI OCR via vesta-cli ocr
    run([cli_bin, 'ocr', 'models'])
    pg_tracks = json.loads(run_capture([cli_bin, 'extract', '-i', inputs['pg_mkv'], '--list-tracks']))
    assert pg_tracks and 'pgs' in pg_tracks[0]['codec'].lower(), pg_tracks
    pg_ocr_srt = temp / 'pg_ocr.srt'
    run([
        cli_bin, 'ocr', 'run', '-i', inputs['pg_mkv'], '--track', str(pg_tracks[0]['index']),
        '-o', pg_ocr_srt, '--provider', 'custom', '--model', 'vision-ok', '--api-url', v1_url, '--quiet',
    ])
    assert 'Mock OCR subtitle' in pg_ocr_srt.read_text(encoding='utf-8')
    roi_ocr_srt = temp / 'roi_ocr.srt'
    run([
        cli_bin, 'ocr', 'run', '-i', inputs['video'], '--roi', '0.08,0.74,0.84,0.20', '--fps', '1',
        '-o', roi_ocr_srt, '--provider', 'custom', '--model', 'vision-ok', '--api-url', v1_url, '--quiet',
    ])
    assert 'Mock OCR subtitle' in roi_ocr_srt.read_text(encoding='utf-8')
    checks['ocr_bitmap_and_roi'] = 'ok'

    # SY-01: vesta-cli sync manual anchors
    synced_srt = temp / 'cli_synced.srt'
    run([cli_bin, 'sync', '-i', inputs['shifted'], '-o', synced_srt, '--anchor', '1:0', '--anchor', '10:18000'])
    synced_text = synced_srt.read_text(encoding='utf-8')
    assert '00:00:00,000 -->' in synced_text and '00:00:18,000 -->' in synced_text
    checks['sync_anchors'] = 'ok'

    # FC-01..FC-08: info, preview --json, TSV, real video+opus+webp+lang, dueling, merge, batch
    run([fc_bin, 'info', inputs['subtitles']])
    preview_json = json.loads(run_capture([
        fc_bin, 'preview', '--target', inputs['subtitles'], '--native', inputs['native'],
        '--min-chars', '5', '--json', '--offset', '1', '--limit', '2',
    ]))
    assert preview_json['active'] == 10 and len(preview_json['rows']) == 2, preview_json

    media_out = temp / 'full-media-deck'
    run([
        fc_bin, 'generate', '--target', inputs['short_subs'], '--video', inputs['video'], '--output', media_out,
        '--deck', 'FullMediaSmoke', '--jobs', '1', '--format', 'apkg', '--audio-format', 'opus',
        '--snapshot-format', 'webp', '--snapshot-width', '160', '--snapshot-height', '120',
        '--pad-audio-start', '50', '--pad-video-start', '50', '--video-hw-accel', 'off',
        '--target-language', 'ja', '--no-auto-card-font',
    ])
    full_apkg = next(media_out.rglob('*.apkg'))
    validate_package(full_apkg, expected_notes=2, require_media=True)
    with zipfile.ZipFile(full_apkg) as zf:
        media_names = set(json.loads(zf.read('media')).values())
        assert any(name.endswith('.opus') for name in media_names), media_names
        assert any(name.endswith('.webp') for name in media_names), media_names
        assert any(name.endswith('.mp4') or name.endswith('.avi') for name in media_names), media_names

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

    batch_manifest = temp / 'batch.json'
    batch_merged = temp / 'batch_series.apkg'
    batch_manifest.write_text(json.dumps({
        'output_dir': str(temp / 'batch-out'),
        'merge_output': str(batch_merged),
        'defaults': {'format': 'apkg', 'no_audio': True, 'no_snapshots': True, 'no_video': True},
        'episodes': [
            {'target': str(inputs['subtitles']), 'deck': 'SeriesEp1', 'episode': 1},
            {'target': str(inputs['alt_subs']), 'deck': 'SeriesEp2', 'episode': 2},
        ],
    }), encoding='utf-8')
    run([fc_bin, 'batch', '--manifest', batch_manifest, '--quiet'])
    validate_package(batch_merged, expected_notes=20, require_media=False)
    checks['flashcards_cli_suite'] = 'ok'

    # TR-01 / TR-04: Transcribe models list + custom STT loopback run
    run([cli_bin, 'transcribe', 'models'])
    transcribed_srt = temp / 'transcribed.srt'
    run([
        cli_bin, 'transcribe', 'run', inputs['video'], '-o', transcribed_srt,
        '--provider', 'custom', '--model', 'whisper-1', '--api-url', v1_url, '--language', 'en', '--quiet',
    ])
    assert 'Unique dialogue number 1.' in transcribed_srt.read_text(encoding='utf-8')
    checks['transcribe_cloud_and_models'] = 'ok'

    # AS-02 / AS-03: Auto-Sync via custom STT loopback
    autosynced_srt = temp / 'autosynced.srt'
    run([
        cli_bin, 'autosync', inputs['shifted'], inputs['video'], '-o', autosynced_srt,
        '--provider', 'custom', '--model', 'whisper-1', '--api-url', v1_url, '--language', 'en',
        '--quick', '--max-offset-secs', '60', '--quiet',
    ])
    assert '00:00:00,000 -->' in autosynced_srt.read_text(encoding='utf-8')
    checks['autosync_cloud'] = 'ok'

    # TL-01..TL-04: Translate tiered failover + missing-cue repair + check-missing + language-list
    run([cli_bin, 'translate', '--language-list'])
    tr_config = temp / 'translate.toml'
    tr_config.write_text(
        '[translation]\n'
        'batch_size = 5\n'
        '[[tiers]]\n'
        f'entries = [{{ provider = "custom", model = "tier0-fail", api_url = "{v1_url}", rpm = 0, max_requests = 1 }}]\n'
        '[[tiers]]\n'
        f'entries = [{{ provider = "custom", model = "translate-repair", api_url = "{v1_url}", rpm = 0 }}]\n',
        encoding='utf-8',
    )
    translated_srt = temp / 'translated.it.srt'
    run([cli_bin, 'translate', '-i', inputs['subtitles'], '-l', 'it', '-c', tr_config, '-o', translated_srt])
    translated_text = translated_srt.read_text(encoding='utf-8')
    assert 'Translated cue 1.' in translated_text and 'Translated cue 10.' in translated_text
    run([cli_bin, 'translate', '-i', inputs['subtitles'], '--check-missing', translated_srt])
    incomplete_srt = temp / 'incomplete.srt'
    incomplete_srt.write_text('1\n00:00:00,000 --> 00:00:01,000\nOnly one.\n\n', encoding='utf-8')
    missing_check = run([cli_bin, 'translate', '-i', inputs['subtitles'], '--check-missing', incomplete_srt], allow_nonzero=True)
    assert missing_check.returncode != 0
    checks['translate_failover_repair'] = 'ok'

    # RF-01..RF-04: Refine inspect --query, edit, enrich (including --allow-partial failure mode), and ankiconnect
    filtered_cards = json.loads(run_capture([cli_bin, 'refine', 'inspect', '--input', merged_apkg, '--query', 'Second deck cue 2']))
    assert len(filtered_cards) == 1, filtered_cards
    edited_apkg = temp / 'edited.apkg'
    run([cli_bin, 'refine', 'edit', '-i', merged_apkg, '-o', edited_apkg, '--id', filtered_cards[0]['id'], '--notes', 'Edited CLI note'])
    re_inspected = json.loads(run_capture([cli_bin, 'refine', 'inspect', '--input', edited_apkg, '--query', 'Edited CLI note']))
    assert len(re_inspected) == 1, re_inspected

    rf_cfg = temp / 'refine.json'
    rf_cfg.write_text(json.dumps({
        'prompt': 'Explain {{expression}}',
        'tiers': [[{'provider': 'custom', 'model': 'refine-ok', 'api_url': v1_url, 'rpm': 0}]],
    }), encoding='utf-8')
    enriched_tsv = temp / 'enriched.tsv'
    run([cli_bin, 'refine', 'enrich', '-i', package_a, '-o', enriched_tsv, '-c', rf_cfg])
    assert 'Concise note' in enriched_tsv.read_text(encoding='utf-8')

    fail_tsv = temp / 'partial_src.tsv'
    fail_tsv.write_text('OK_CARD\tHello\t\nFAIL_CARD\tFail\t\n', encoding='utf-8')
    partial_cfg = temp / 'refine_partial.json'
    partial_cfg.write_text(json.dumps({
        'prompt': 'Explain {{expression}}',
        'tiers': [[{'provider': 'custom', 'model': 'refine-partial', 'api_url': v1_url, 'rpm': 0}]],
    }), encoding='utf-8')
    partial_out = temp / 'partial_out.tsv'
    partial_res = run([cli_bin, 'refine', 'enrich', '-i', fail_tsv, '-o', partial_out, '-c', partial_cfg, '--allow-partial'], allow_nonzero=True)
    assert partial_res.returncode != 0 and 'Concise note' in partial_out.read_text(encoding='utf-8')

    run([cli_bin, 'refine', 'ankiconnect', '--url', anki_url, 'ping'])
    decks_out = run_capture([cli_bin, 'refine', 'ankiconnect', '--url', anki_url, 'decks'])
    assert 'QualitySmoke' in decks_out
    run([cli_bin, 'refine', 'ankiconnect', '--url', anki_url, 'import', '--apkg', package_a])
    checks['refine_and_ankiconnect'] = 'ok'

    # CM-01 / CM-02: vesta-cli mcp live stdio server verification (24 tools)
    mcp_out = temp / 'mcp-out'
    mcp_out.mkdir(exist_ok=True)
    checks['mcp'] = run_mcp_smoke(cli_bin, temp, mcp_out, {**inputs, 'apkg_a': package_a, 'apkg_b': package_b}, anki_url, v1_url)
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
    mock_server = start_mock_server()
    base_url = f'http://127.0.0.1:{mock_server.server_port}'
    try:
        with tempfile.TemporaryDirectory(prefix='vesta-quality-') as directory:
            temp = Path(directory)
            subtitles = temp / 'input.srt'
            native = temp / 'native.srt'
            shifted = temp / 'shifted.srt'
            short_subs = temp / 'short.srt'
            alt_subs = temp / 'alt.srt'
            subtitles.write_text(''.join(
                f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nUnique dialogue number {i+1}.\n\n'
                for i in range(10)), encoding='utf-8')
            native.write_text(''.join(
                f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nNative line {i+1}.\n\n'
                for i in range(10)), encoding='utf-8')
            shifted.write_text(''.join(
                f'{i+1}\n00:00:{i*2+5:02},000 --> 00:00:{i*2+6:02},000\nUnique dialogue number {i+1}.\n\n'
                for i in range(10)), encoding='utf-8')
            short_subs.write_text(''.join(
                f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\n日本語テスト {i+1}\n\n'
                for i in range(2)), encoding='utf-8')
            alt_subs.write_text(''.join(
                f'{i+1}\n00:00:{i*2:02},000 --> 00:00:{i*2+1:02},000\nSecond deck cue {i+1}.\n\n'
                for i in range(10)), encoding='utf-8')

            video = temp / 'fixture.mp4'
            run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i',
                 'testsrc2=size=320x240:rate=10:duration=20', '-f', 'lavfi', '-i',
                 'sine=frequency=440:sample_rate=16000:duration=20', '-c:v', 'libx264',
                 '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest', video])
            mkv = temp / 'with_subs.mkv'
            run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-i', video, '-i', subtitles,
                 '-c', 'copy', '-metadata:s:s:0', 'language=eng', mkv])
            pg_mkv = create_single_cue_pgs_mkv(temp)

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

            alt_dir = temp / 'alt-merge-src'
            run([fc_bin, 'generate', '--target', alt_subs, '--video', video, '--output', alt_dir,
                 '--deck', 'QualitySmokeAlt', '--jobs', '2', '--no-optimize', '--no-video', '--format', 'apkg'])
            alt_package = next(alt_dir.rglob('*.apkg'))

            smoke_checks = run_suite_checks(
                cli_bin,
                fc_bin,
                temp,
                {
                    'subtitles': subtitles,
                    'native': native,
                    'shifted': shifted,
                    'short_subs': short_subs,
                    'alt_subs': alt_subs,
                    'video': video,
                    'mkv': mkv,
                    'pg_mkv': pg_mkv,
                },
                apkg_outputs[0],
                alt_package,
                base_url,
            )
    finally:
        mock_server.shutdown()
        mock_server.server_close()

    print(json.dumps({'platform': platform.platform(), 'cpu_count': os.cpu_count(),
        'profile': args.profile, 'samples': args.samples, 'correctness': contents,
        'smoke_checks': smoke_checks,
        'seconds': {name: {'median': statistics.median(values), 'min': min(values),
                          'max': max(values), 'runs': values} for name, values in timings.items()}}, indent=2))


if __name__ == '__main__':
    main()
