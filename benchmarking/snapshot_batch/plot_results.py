#!/usr/bin/env python3
"""Render measured native A/B timings; never combines unrelated benchmark data."""
import argparse
import json
from pathlib import Path
import statistics


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('results', type=Path, help='results.json from verify_native.py')
    args = parser.parse_args()
    data = json.loads(args.results.read_text())
    if data['status'] != 'complete':
        parser.error('The benchmark must be complete before generating publication charts')
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    directory = args.results.parent / 'charts'
    directory.mkdir(exist_ok=True)
    for profile in data['settings']['profiles']:
        cases = [c for c in data['cases'] if c['distribution'] == 'full' and c['profile'] == profile]
        if not cases:
            continue
        labels, baseline, optimized = [], [], []
        for case in cases:
            rows = [r for r in data['runs'] if r['case'] == case['id']]
            expected = {(rep, mode) for rep in range(data['settings']['repetitions']) for mode in ('baseline', 'optimized')}
            if {(r['rep'], r['mode']) for r in rows} != expected or len(rows) != len(expected):
                raise ValueError('Incomplete or duplicate run matrix')
            if not all(r['validated'] and r['status'] == 'completed' and r['returncode'] == 0 and r['fingerprints'] == rows[0]['fingerprints'] for r in rows):
                raise ValueError('Quality validation failed')
            labels.append(Path(case['video']).stem.replace('.', ' '))
            baseline.append(statistics.median(r['seconds'] for r in rows if r['mode'] == 'baseline'))
            optimized.append(statistics.median(r['seconds'] for r in rows if r['mode'] == 'optimized'))
        fig, axis = plt.subplots(figsize=(10, 6))
        indices = list(range(len(labels)))
        axis.barh([i - .19 for i in indices], baseline, height=.36, label='Original extraction', color='#64748b')
        bars = axis.barh([i + .19 for i in indices], optimized, height=.36, label='Rust batching enabled', color='#2563eb')
        for bar, original, improved in zip(bars, baseline, optimized):
            axis.text(bar.get_width() + 3, bar.get_y() + bar.get_height()/2,
                      f'{(1-improved/original)*100:+.1f}% time saved', va='center', fontsize=9)
        axis.set_yticks(indices, labels)
        axis.invert_yaxis()
        axis.set_xlim(0, max(baseline + optimized)*1.35)
        axis.set_xlabel('Median wall time (seconds); lower is better')
        axis.set_title(f'Vesta: complete films, {profile} snapshot profile')
        axis.legend(loc='lower right')
        axis.grid(axis='x', alpha=.2)
        axis.set_axisbelow(True)
        fig.text(.01, .015, f"{data['settings']['repetitions']} alternating A/B pairs · audio + snapshots + APKG · identical media and note fields\n"
                 f"{data['settings']['jobs']} requested workers · OS cache uncontrolled · excludes video clips and subs2srs", fontsize=8)
        fig.tight_layout(rect=(0, .07, 1, 1))
        for extension in ('svg', 'png'):
            output = directory / f'full-films-{profile}.{extension}'
            fig.savefig(output, dpi=160)
            print(output)
        plt.close(fig)


if __name__ == '__main__':
    main()
