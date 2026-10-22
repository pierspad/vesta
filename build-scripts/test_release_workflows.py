#!/usr/bin/env python3
"""Guard release publishing against cancellation, partial builds, and tag mismatch."""
from pathlib import Path
import os
import subprocess
import tempfile
import textwrap

ROOT = Path(__file__).resolve().parents[1]


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def run_block(text, step_name):
    """Extract an indented shell run block; execute it as bash in a fixture."""
    lines = text.splitlines()
    start = next(i for i, line in enumerate(lines) if line.strip() == f'- name: {step_name}')
    run = next(i for i in range(start + 1, len(lines)) if lines[i].strip() == 'run: |')
    indent = len(lines[run]) - len(lines[run].lstrip())
    end = run + 1
    while end < len(lines) and (not lines[end].strip() or len(lines[end]) - len(lines[end].lstrip()) > indent):
        end += 1
    return textwrap.dedent('\n'.join(lines[run + 1:end]))


def check_shell_behavior(build):
    tag_check = run_block(build, 'Verify release tag matches source version')
    fallback = run_block(build, 'Prepare fallback release body')
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / 'build-scripts').mkdir()
        (root / 'build-scripts/PKGBUILD').write_text('pkgver=0.29.1\n', encoding='utf-8')
        subprocess.run(['git', 'init', '-q', directory], check=True)
        subprocess.run(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                        'commit', '--allow-empty', '-qm', 'test fixture'], cwd=root, check=True)
        env = {**os.environ, 'GITHUB_REPOSITORY': 'pierspad/vesta'}
        for tag, succeeds in [('v0.29.1', True), ('v0.29.0', False)]:
            result = subprocess.run(['bash', '-c', tag_check], cwd=root, env={**env, 'RELEASE_TAG': tag}, capture_output=True)
            require((result.returncode == 0) == succeeds, f'tag validation behavior is incorrect for {tag}')
        # Tags are data: fallback generation must never interpret shell syntax.
        tag = 'v0.29.1$(touch unexpected-file)'
        subprocess.run(['bash', '-c', fallback], cwd=root, env={**env, 'RELEASE_TAG': tag}, check=True)
        body = (root / 'release-notes-for-tag.md').read_text(encoding='utf-8')
        sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
        require(tag in body and f'https://github.com/pierspad/vesta/commit/{sha}' in body,
                'fallback must contain the literal tag and the checked-out source commit')
        require(not (root / 'unexpected-file').exists(), 'fallback interpreted the tag as shell code')


def main():
    release = (ROOT / '.github/workflows/release.yml').read_text(encoding='utf-8')
    build = (ROOT / '.github/workflows/build.yml').read_text(encoding='utf-8')

    require('cancel-in-progress: false' in release,
            'release runs must not cancel an active release')

    release_job_start = build.rfind('\n  release:\n')
    require(release_job_start >= 0, 'build workflow must define its final release job')
    release_job = build[release_job_start:]
    required_builds = ('build-linux', 'build-linux-arch', 'build-linux-flatpak', 'build-windows')
    for job in required_builds:
        require(f"needs.{job}.result == 'success'" in release_job,
                f'release publishing must require successful {job}')

    require('Verify release tag matches source version' in release_job
            and 'TAG_VERSION="${RELEASE_TAG#v}"' in release_job
            and "PKGBUILD_VERSION=$(sed -n 's/^pkgver=//p'" in release_job
            and 'if [[ "$TAG_VERSION" != "$PKGBUILD_VERSION" ]]' in release_job,
            'release publishing must reject a tag that differs from PKGBUILD pkgver')

    fallback_start = release_job.find('- name: Prepare fallback release body')
    fallback_end = release_job.find('- name: Create Release and Upload Assets', fallback_start)
    fallback = release_job[fallback_start:fallback_end]
    require(fallback_start >= 0 and fallback_end > fallback_start
            and 'Source commit:' in fallback and 'git rev-parse HEAD' in fallback
            and '"$RELEASE_TAG" "$GITHUB_REPOSITORY" "$TAG_SHA"' in fallback,
            'fallback release notes must safely link to the checked-out source commit')
    require('CHANGELOG.md' not in fallback,
            'fallback release notes must not link to a changelog that is not generated')

    check_shell_behavior(build)
    print('Release workflow safety checks passed (including executable shell fixtures).')


if __name__ == '__main__':
    main()
