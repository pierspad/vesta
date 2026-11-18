import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const require = createRequire(import.meta.url);
const match = require('picomatch');

test('Vite and Vitest use the local security fix', () => {
  for (const consumer of ['vite', 'vitest', 'fdir', 'tinyglobby']) {
    const fromConsumer = createRequire(import.meta.resolve(consumer));
    assert.equal(fromConsumer.resolve('picomatch'), require.resolve('picomatch'));
  }
});

test('single-branch wildcard repetition cannot block the event loop', () => {
  const child = spawnSync(process.execPath, ['-e', `
    const assert = require('node:assert/strict');
    const match = require(process.argv[1]);
    for (const pattern of ['+(*)b', '+(*a)b', '*(*)b', '*(*a)b', '+(a*)b', '+(??*)b']) {
      assert.equal(match(pattern)('a'.repeat(80) + '!'), false);
    }
  `, require.resolve('picomatch')], { timeout: 5000, encoding: 'utf8' });
  assert.equal(child.error, undefined, String(child.error));
  assert.equal(child.status, 0, child.stderr);
});

test('ordinary globbing, bounded extglobs and literal wildcards retain their semantics', () => {
  for (const [pattern, input, expected] of [
    ['src/**/*.svelte', 'src/lib/App.svelte', true],
    ['src/**/*.{ts,svelte}', 'src/lib/App.ts', true],
    ['src/**/*.ts', 'src/lib/App.js', false],
    ['**/node_modules/**', '/repo/node_modules/vite/index.js', true],
    ['+(ab)', 'abab', true],
    ['+(a|b)', 'abba', true],
    ['+([ab])', 'abba', true],
    ['+(\\*)', '***', true],
    ['+([*])', '***', true],
    ['+("*")', '***', true],
    ['+(*(a)|*(b))', 'abba', true],
    ['+(*)b', '+(*)b', true],
  ]) assert.equal(match(pattern)(input), expected, `${pattern} / ${input}`);
});
