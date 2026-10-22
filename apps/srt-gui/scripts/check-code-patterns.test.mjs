import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inspectSource } from './check-code-patterns.mjs';

test('reports async callbacks and executors with source locations', () => {
  const findings = inspectSource('const x = 1;\nitems.forEach(async item => save(item));\nnew Promise(async resolve => resolve(1));\ndebugger;');
  assert.deepEqual(findings.map(f => [f.rule, f.line, f.column]), [
    ['async-for-each', 2, 1], ['async-promise-executor', 3, 1], ['debugger', 4, 1],
  ]);
});
test('ignores comments, strings and safe async iteration', () => {
  assert.deepEqual(inspectSource('// debugger;\nconst text = "new Promise(async () => {})";\nawait Promise.all(items.map(async x => save(x)));\nitems.forEach(x => log(x));'), []);
});
test('checks instance and module scripts at their original offsets', () => {
  const source = '<script module lang="ts">\nnew Promise(async () => {});\n</script>\n<p>debugger; items.forEach(async x => x)</p>\n<script lang="ts">\nitems.forEach(async x => save(x));\n</script>';
  assert.deepEqual(inspectSource(source, 'View.svelte').map(f => [f.rule, f.line]).sort((a,b) => a[1]-b[1]), [['async-promise-executor', 2], ['async-for-each', 6]]);
});
test('does not ignore malformed Svelte', () => {
  assert.throws(() => inspectSource('<script>const =</script>', 'Bad.svelte'));
});
