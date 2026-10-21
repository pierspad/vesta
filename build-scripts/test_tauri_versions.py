#!/usr/bin/env python3
import copy
import unittest
from check_tauri_versions import check_pairs


class TauriVersionTests(unittest.TestCase):
    def setUp(self):
        self.cargo = {'package': [{'name': 'tauri', 'version': '2.11.2'}]}
        self.npm = {'packages': {'node_modules/@tauri-apps/api': {'version': '2.11.1'}}}
        for name in ('opener', 'dialog', 'fs'):
            self.cargo['package'].append({'name': 'tauri-plugin-' + name, 'version': '2.5.1'})
            self.npm['packages']['node_modules/@tauri-apps/plugin-' + name] = {'version': '2.5.1'}

    def test_compatible(self):
        self.assertEqual(check_pairs(self.cargo, self.npm), [])

    def test_each_pair_mismatch(self):
        for name in ('api', 'plugin-opener', 'plugin-dialog', 'plugin-fs'):
            with self.subTest(name=name):
                npm = copy.deepcopy(self.npm)
                npm['packages']['node_modules/@tauri-apps/' + name]['version'] = '2.12.0'
                self.assertEqual(len(check_pairs(self.cargo, npm)), 1)

    def test_plugin_patch_mismatch(self):
        self.cargo['package'][-1]['version'] = '2.5.2'
        self.assertEqual(len(check_pairs(self.cargo, self.npm)), 1)

    def test_missing_and_multiple_versions(self):
        for version in (None, '2.11.3'):
            with self.subTest(version=version):
                cargo = copy.deepcopy(self.cargo)
                if version is None:
                    cargo['package'].pop(0)
                else:
                    cargo['package'].append({'name': 'tauri', 'version': version})
                self.assertEqual(len(check_pairs(cargo, self.npm)), 1)

    def test_nested_duplicate_js(self):
        self.npm['packages']['node_modules/consumer/node_modules/@tauri-apps/api'] = {'version': '2.12.0'}
        self.assertEqual(len(check_pairs(self.cargo, self.npm)), 1)


if __name__ == '__main__':
    unittest.main()
