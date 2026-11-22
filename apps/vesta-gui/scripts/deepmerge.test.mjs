import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import test from 'node:test';

// Resolve through the plugin so this also verifies the installed override.
const pluginRequire = createRequire(import.meta.resolve('@sveltejs/vite-plugin-svelte'));
const merge = pluginRequire('deepmerge');

test('target __proto__ properties cannot poison the result or nested objects', () => {
  for (const target of [
    JSON.parse('{"__proto__":{"polluted":true}}'),
    { nested: JSON.parse('{"__proto__":{"polluted":true}}') },
  ]) {
    const result = merge(target, {});
    for (const object of [result, result.nested].filter(Boolean)) {
      assert.equal(Object.getPrototypeOf(object), Object.prototype);
      assert.equal(object.polluted, undefined);
    }
    assert.equal({}.polluted, undefined);
  }
});

test('null-prototype sources and own target/source __proto__ merges stay safe', () => {
  const source = Object.assign(Object.create(null), JSON.parse('{"__proto__":{"polluted":true}}'));
  for (const target of [{}, Object.create(null), JSON.parse('{"__proto__":{"existing":true}}')]) {
    const result = merge(target, source);
    assert.equal(Object.getPrototypeOf(result), Object.prototype);
    assert.equal(result.polluted, undefined);
    assert.equal({}.polluted, undefined);
  }
});

test('Svelte config merging replaces arrays, clones nested options and preserves functions', () => {
  const preprocess = () => {};
  const target = { compilerOptions: { dev: false }, preprocess: ['old'] };
  const source = { compilerOptions: { css: 'external' }, preprocess: [preprocess] };
  const result = merge(target, source, { arrayMerge: (_target, source) => source });
  assert.deepEqual(result, {
    compilerOptions: { dev: false, css: 'external' }, preprocess: [preprocess],
  });
  assert.notEqual(result.compilerOptions, target.compilerOptions);
  assert.deepEqual(target.preprocess, ['old']);
});

test('default arrays, symbols, special objects and custom mergers retain upstream behavior', () => {
  const symbol = Symbol('option');
  const date = new Date(0);
  const result = merge({ list: [{ a: 1 }], nested: { a: 1 } }, {
    list: [{ b: 2 }], nested: { b: 2 }, [symbol]: date,
  });
  assert.deepEqual(result.list, [{ a: 1 }, { b: 2 }]);
  assert.deepEqual(result.nested, { a: 1, b: 2 });
  assert.equal(result[symbol], date);
  assert.deepEqual(merge.all([{ a: 1 }, { b: 2 }]), { a: 1, b: 2 });
  assert.deepEqual(merge({ nested: {} }, { nested: {} }, {
    customMerge: () => () => 'custom',
  }), { nested: 'custom' });
});
