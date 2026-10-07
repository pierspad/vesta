"""Tests of benchmark validation and process supervision, not speed measurements."""
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('benchmark', Path(__file__).with_name('verify_native.py'))
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


class RunnerTests(unittest.TestCase):
    def test_discovery_handles_year_suffix_and_reports_missing_subtitles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Detour(1945).mp4').touch()
            (root / 'missing.mkv').touch()
            (root / 'Detour-en.srt').write_text('1\n00:00:00,100 --> 00:00:01,200\nFirst\n\n2\n00:00:02,100 --> 00:00:03,200\nSecond\n')
            films, skipped = benchmark.discover(root, 'en')
            self.assertEqual(len(films), 1)
            self.assertEqual(len(skipped), 1)
            cases = benchmark.make_cases(films, ['25', 'all'], ['default'], 2)
            self.assertEqual([case['cards'] for case in cases], [2, 2])
            self.assertEqual(cases[0]['distribution'], 'full')
            self.assertEqual(cases[1]['entries'], films[0]['entries'])

    def package(self, folder, text='Hello', empty=False, broken_ref=False):
        db = folder / 'input.sqlite'
        with sqlite3.connect(db) as connection:
            connection.execute('create table notes(flds text)')
            connection.execute('create table col(models text)')
            connection.execute('insert into col values (?)', (json.dumps({'1': {'flds': [{'name': 'Text'}, {'name': 'Audio'}, {'name': 'Image'}]}}),))
            connection.execute('insert into notes values (?)', (text + '\x1f[sound:' + ('missing.mp3' if broken_ref else 'audio.mp3') + ']\x1f<img src="image.webp">',))
        package = folder / 'deck.apkg'
        with zipfile.ZipFile(package, 'w') as archive:
            archive.writestr('media', json.dumps({'0': 'audio.mp3', '1': 'image.webp'}))
            archive.writestr('0', b'audio')
            archive.writestr('1', b'' if empty else b'image')
            archive.write(db, 'collection.anki2')
        db.unlink()
        return package

    def test_validation_detects_empty_media_dangling_references_and_changed_notes(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            first = benchmark.validate(self.package(folder), 1, folder)
            second = benchmark.validate(self.package(folder, text='Changed'), 1, folder)
            self.assertEqual(first['media'], second['media'])
            self.assertNotEqual(first['notes_sha256'], second['notes_sha256'])
            with self.assertRaises(ValueError):
                benchmark.validate(self.package(folder, empty=True), 1, folder)
            with self.assertRaises(ValueError):
                benchmark.validate(self.package(folder, broken_ref=True), 1, folder)

    def test_large_logs_do_not_block_and_timeout_reaps_process(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            result = benchmark.execute([sys.executable, '-c', 'import sys; sys.stderr.write("x"*2000000)'], folder, 5)
            self.assertEqual(result['returncode'], 0)
            self.assertEqual((folder / 'stderr.log').stat().st_size, 2000000)
            result = benchmark.execute([sys.executable, '-c', 'import time; time.sleep(60)'], folder, .1)
            self.assertEqual(result['status'], 'timeout')
            self.assertNotEqual(result['returncode'], 0)
            self.assertLess(result['seconds'], 4)


if __name__ == '__main__':
    unittest.main()
