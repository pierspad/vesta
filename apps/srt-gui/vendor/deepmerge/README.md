# Temporary deepmerge security fork

Based on the MIT-licensed CommonJS distribution of deepmerge 4.3.1:
https://github.com/TehShrike/deepmerge/tree/v4.3.1

GHSA-gcqx-w48m-w2h4 affects the upstream package, including copies of an own
`__proto__` property from the target. All three destination writes in
`mergeObject` now define own data properties instead of invoking inherited
setters. Existing merge options and array behavior are preserved.

The Svelte Vite plugin receives this package through an npm override referencing the direct local dependency.
No audit severity threshold or advisory suppression is used. The local package
has no external dependencies; npm audit cannot assess this vendored source, so
`npm run test:merge` checks the installed override's poisoning cases and the
plugin's array replacement semantics. It is also part of `npm test`.

Remove this fork and the override once the plugin uses an upstream fixed release,
then regenerate the lockfile and run the audit, merge tests, frontend suite,
production build and cold dev CSS checks.
