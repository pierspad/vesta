#!/usr/bin/env python3
"""Regression tests for pruning Cargo build output without touching source files."""
import os
from pathlib import Path
import tempfile
import time
import unittest

from quality_check import prune_stale_target


class PruneTargetTests(unittest.TestCase):
    def test_handles_flat_and_nested_cargo_layouts_and_retains_latest(self):
        for nested in (False, True):
            with self.subTest(nested=nested), tempfile.TemporaryDirectory() as directory:
                target = Path(directory)
                build = target / 'debug' / 'build'
                parent = build / 'example' if nested else build
                names = ['a' * 16, 'b' * 16, 'c' * 16]
                paths = [parent / (name if nested else f'example-{name}') for name in names]
                for index, path in enumerate(paths):
                    path.mkdir(parents=True)
                    (path / 'artifact').write_bytes(b'cache')
                    age = 10 * 86400 if index == 0 else 100 - index
                    os.utime(path, (time.time() - age,) * 2)
                self.assertGreater(prune_stale_target(target, 7), 0)
                self.assertFalse(paths[0].exists())
                self.assertTrue(paths[1].exists())
                self.assertTrue(paths[2].exists())
                prune_stale_target(target, 0)
                self.assertFalse(paths[1].exists())
                self.assertTrue(paths[2].exists())

    def test_keeps_symlinks_and_unrecognized_directories(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            outside = root / 'outside'
            outside.mkdir()
            (outside / 'source').write_text('keep')
            build = root / 'target' / 'debug' / 'build'
            build.mkdir(parents=True)
            (build / 'linked').symlink_to(outside, target_is_directory=True)
            (build / 'example').mkdir()
            (build / 'example' / ('a' * 16)).symlink_to(outside, target_is_directory=True)
            (build / 'example' / 'unrecognized').mkdir()
            prune_stale_target(root / 'target', 0)
            self.assertEqual((outside / 'source').read_text(), 'keep')
            self.assertTrue((build / 'linked').is_symlink())
            self.assertTrue((build / 'example' / 'unrecognized').exists())


if __name__ == '__main__':
    unittest.main()
