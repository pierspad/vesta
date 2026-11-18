#!/usr/bin/env python3
"""Exercise runner APT configuration without touching the host package manager."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'build-scripts/setup_ci_apt.sh'


class RunnerAptTests(unittest.TestCase):
    def run_setup(self, root, *, architecture='amd64', actions='true', runner='Linux'):
        tools = root / 'bin'
        tools.mkdir(exist_ok=True)
        for name, content in {
            'sudo': '#!/bin/sh\nexec "$@"\n',
            'dpkg': '#!/bin/sh\nprintf "%s\\n" "$FIXTURE_ARCH"\n',
        }.items():
            path = tools / name
            path.write_text(content)
            path.chmod(0o755)
        env = {**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
               'VESTA_APT_ETC': str(root / 'apt config'), 'FIXTURE_ARCH': architecture,
               'GITHUB_ACTIONS': actions, 'RUNNER_OS': runner}
        return subprocess.run(['bash', str(SCRIPT)], env=env, capture_output=True, text=True)

    def test_rejects_non_runner_invocations_without_writing(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for actions, runner in [('false', 'Linux'), ('true', 'Windows')]:
                self.assertNotEqual(self.run_setup(root, actions=actions, runner=runner).returncode, 0)
                self.assertFalse((root / 'apt config').exists())

    def test_canonical_mirror_minimal_installs_and_bounded_retries_are_idempotent(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            apt = root / 'apt config'
            (apt / 'sources.list.d').mkdir(parents=True)
            (apt / 'apt-mirrors.txt').write_text('http://us-east-2.ec2.archive.ubuntu.com/ubuntu\n')
            third_party = apt / 'sources.list.d/lunarg.list'
            third_party.write_text('deb https://packages.lunarg.com/vulkan noble main\n')
            for _ in range(2):
                result = self.run_setup(root)
                self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((apt / 'apt-mirrors.txt').read_text(), 'https://archive.ubuntu.com/ubuntu\n')
            self.assertEqual(third_party.read_text(), 'deb https://packages.lunarg.com/vulkan noble main\n')
            config = (apt / 'apt.conf.d/80-vesta-ci').read_text()
            for setting in ['Acquire::Retries "2";', 'Acquire::http::Timeout "30";',
                            'Acquire::https::Timeout "30";', 'APT::Install-Recommends "false";',
                            'APT::Update::Error-Mode "any";']:
                self.assertIn(setting, config)
            for bypass in ['AllowUnauthenticated', 'Verify-Peer', 'AllowInsecureRepositories']:
                self.assertNotIn(bypass, config)

    def test_ports_and_runners_without_mirror_lists(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            apt = root / 'apt config'
            apt.mkdir()
            self.assertEqual(self.run_setup(root).returncode, 0)
            self.assertFalse((apt / 'apt-mirrors.txt').exists())
            (apt / 'apt-mirrors.txt').touch()
            self.assertEqual(self.run_setup(root, architecture='arm64').returncode, 0)
            self.assertEqual((apt / 'apt-mirrors.txt').read_text(), 'https://ports.ubuntu.com/ubuntu-ports\n')


if __name__ == '__main__':
    unittest.main()
