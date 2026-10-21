/** Small AST gate for error-prone async patterns. Not a general correctness proof. */
import ts from 'typescript';
import { parse } from 'svelte/compiler';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const isAsync = node => !!node?.modifiers?.some(m => m.kind === ts.SyntaxKind.AsyncKeyword);

export function inspectSource(source, filename = 'input.ts') {
  const findings = [];
  // The Svelte parser locates both scripts without confusing markup/string contents
  // with tags. Preserve offsets so diagnostics point into the original component.
  const ranges = filename.endsWith('.svelte')
    ? (() => { const ast = parse(source); return [ast.instance, ast.module].filter(Boolean).map(s => [s.content.start, s.content.end]); })()
    : [[0, source.length]];
  for (const [start, end] of ranges) {
    const tree = ts.createSourceFile(filename + '.ts', source.slice(start, end), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const report = (node, rule, message) => {
      const offset = start + node.getStart(tree);
      const lines = source.slice(0, offset).split('\n');
      findings.push({ file: filename, line: lines.length, column: lines.at(-1).length + 1, rule, message });
    };
    const visit = node => {
      if (ts.isDebuggerStatement(node)) report(node, 'debugger', 'Remove the debugger statement.');
      if (ts.isNewExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'Promise' && isAsync(node.arguments?.[0])) {
        report(node, 'async-promise-executor', 'Async Promise executors can lose rejections; use an async function directly.');
      }
      if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression) && node.expression.name.text === 'forEach' && isAsync(node.arguments[0])) {
        report(node, 'async-for-each', 'forEach discards async callback promises; await a loop or Promise.all instead.');
      }
      ts.forEachChild(node, visit);
    };
    visit(tree);
  }
  return findings;
}

export function scanDirectory(root) {
  return readdirSync(root, { withFileTypes: true }).sort((a,b) => a.name.localeCompare(b.name)).flatMap(entry => {
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) return scanDirectory(path);
    return /\.(ts|svelte)$/.test(entry.name) ? inspectSource(readFileSync(path, 'utf8'), path) : [];
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const app = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const findings = scanDirectory(resolve(app, 'src'));
  for (const f of findings) console.error(`${relative(app, f.file)}:${f.line}:${f.column} [${f.rule}] ${f.message}`);
  console.log(`Code pattern check: ${findings.length} finding(s).`);
  process.exitCode = findings.length ? 1 : 0;
}
