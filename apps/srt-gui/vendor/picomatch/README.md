# Temporary picomatch security fork

Based on the MIT-licensed picomatch 4.0.7 distribution:
https://github.com/micromatch/picomatch/tree/4.0.7

GHSA-fc5q-39xc-vjpf / CVE-2026-112366 affects single-branch repeated extglobs
such as `+(*)` and `+(*a)`. There is no patched npm release as of 2026-11-18.
The local patch applies the existing risky-extglob literalization policy when a
single repeated branch contains an unbounded glob wildcard. Escaped stars,
quoted literals and bracket classes do not trigger the new guard. The upstream
`maxExtglobRecursion: false` opt-out remains unchanged.

An npm override routes all build-tool consumers to this source. Audit does not
inspect vendored code; `npm run test:glob` checks the installed overrides through
Vite and Vitest, ordinary glob semantics, and timeout-bounded adversarial cases.
The regression runs in `npm test`. No audit threshold or advisory ignore is used.

Remove the fork and override once upstream publishes a fixed release; rerun the
glob regressions, frontend suite, cold CSS checks, build and npm audit.
