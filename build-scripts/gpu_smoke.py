#!/usr/bin/env python3
"""Opt-in local Vulkan inference smoke; timings are diagnostics, not benchmarks."""
import argparse
import asyncio
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def validate_srt(path, duration):
    text = path.read_text(encoding='utf-8')
    timestamps = re.findall(r'(\d+):(\d+):(\d+),(\d+) --> (\d+):(\d+):(\d+),(\d+)', text)
    assert timestamps, 'No subtitle timestamps'
    previous_start = -1
    for fields in timestamps:
        fields = list(map(int, fields))
        start = fields[0] * 3600 + fields[1] * 60 + fields[2] + fields[3] / 1000
        end = fields[4] * 3600 + fields[5] * 60 + fields[6] + fields[7] / 1000
        assert previous_start <= start <= end <= duration + 1, f'Invalid timing: {start}–{end}'
        previous_start = start
    assert any(re.search(r'[A-Za-z]', block) for block in text.split('\n\n')), 'No transcript text'
    return len(timestamps)


async def run_case(args, directory, name, duration, flags=(), hidden=False, cancel=False):
    output = directory / f'{name}.srt'
    command = [str(args.binary), 'run', str(args.media), '--output', str(output),
               '--model', args.model, '--language', 'en', '--json', *flags]
    environment = os.environ.copy()
    if hidden:
        environment['GGML_VK_VISIBLE_DEVICES'] = ''
    started = time.monotonic()
    signal_sent = None
    with (directory / f'{name}.stderr').open('wb') as stderr:
        process = await asyncio.create_subprocess_exec(*command, stdout=asyncio.subprocess.PIPE,
                                                       stderr=stderr, env=environment,
                                                       start_new_session=os.name == 'posix')
        messages = []
        try:
            async with asyncio.timeout(args.timeout) as deadline:
                while line := await process.stdout.readline():
                    message = json.loads(line)
                    messages.append(message)
                    if (cancel and signal_sent is None and message.get('stage') == 'transcribe'
                            and message.get('percentage', 0) > 15):
                        process.send_signal(signal.SIGINT)
                        signal_sent = time.monotonic()
                        deadline.reschedule(asyncio.get_running_loop().time() + args.cancel_timeout)
                code = await process.wait()
        finally:
            if process.returncode is None:
                if os.name == 'posix':
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    process.kill()
                await process.wait()
    (directory / f'{name}.jsonl').write_text(''.join(json.dumps(m) + '\n' for m in messages))
    elapsed = time.monotonic() - started
    terminal = [m for m in messages if 'ok' in m]
    assert len(terminal) == 1, f'{name}: expected one terminal JSON result'
    log = (directory / f'{name}.stderr').read_text(errors='replace')
    gpu_used = bool(re.search(r'using Vulkan\d+ backend', log))
    if cancel:
        assert gpu_used, 'Cancellation case did not use the Vulkan backend'
        assert signal_sent is not None, 'Inference stage was never reached'
        assert code != 0 and not terminal[0]['ok'], 'Cancelled process reported success'
        assert 'cancel' in terminal[0].get('error', '').lower(), terminal[0]
        assert not output.exists(), 'Cancelled inference published output'
        return {'case': name, 'cancel_seconds': round(time.monotonic() - signal_sent, 3)}
    assert code == 0 and terminal[0]['ok'], f'{name}: {terminal[0]}; see {directory / (name + ".stderr")}'
    expected_gpu = '--no-gpu' not in flags and not hidden
    assert gpu_used == expected_gpu, f'{name}: wrong backend; see stderr'
    if expected_gpu and args.device:
        assert args.device in log, f'Expected device {args.device!r} not in native log'
    count = validate_srt(output, duration)
    assert count == terminal[0]['outcome']['subtitle_count'], 'Result count does not match SRT'
    return {'case': name, 'gpu_used': gpu_used, 'subtitles': count, 'seconds': round(elapsed, 3)}


async def smoke(args, directory):
    probe = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration',
                            '-of', 'default=noprint_wrappers=1:nokey=1', str(args.media)],
                           capture_output=True, text=True, check=True, timeout=30)
    duration = float(probe.stdout.strip())
    assert duration > 0, 'Empty input media'
    cases = []
    for name, flags, hidden in [('cpu', ('--no-gpu',), False), ('gpu', (), False),
                                ('gpu_unavailable', (), True)]:
        cases.append(await run_case(args, directory, name, duration, flags, hidden))
    if args.vad:
        cases.append(await run_case(args, directory, 'gpu_vad', duration, ('--vad',)))
    if os.name == 'posix':
        cases.append(await run_case(args, directory, 'cancel', duration, cancel=True))
    result = {'binary': str(args.binary), 'media': str(args.media), 'model': args.model,
              'profile_note': 'Smoke timings; no performance comparison is established.',
              'cases': cases}
    (directory / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/srt-transcribe')
    parser.add_argument('--media', type=Path, default=ROOT / 'Test_Subs/fixtures/detour/detour_clip_90s.mp4')
    parser.add_argument('--model', default='base', help='Already installed model; no downloads are performed')
    parser.add_argument('--device', help='Expected device name substring in native Vulkan logs')
    parser.add_argument('--vad', action='store_true', help='Also run VAD; its model must already be installed')
    parser.add_argument('--output-dir', type=Path, help='New directory in which to retain logs and transcripts')
    parser.add_argument('--timeout', type=float, default=180)
    parser.add_argument('--cancel-timeout', type=float, default=15)
    args = parser.parse_args()
    if not args.binary.is_file():
        parser.error('Build first: cargo build -p srt-transcribe-cli --features vulkan')
    if args.timeout <= 0 or args.cancel_timeout <= 0:
        parser.error('Timeouts must be positive')
    if args.output_dir:
        args.output_dir.mkdir(parents=True, exist_ok=False)
        asyncio.run(smoke(args, args.output_dir))
    else:
        with tempfile.TemporaryDirectory(prefix='vesta-gpu-smoke-') as directory:
            asyncio.run(smoke(args, Path(directory)))


if __name__ == '__main__':
    main()
