# Contributing

Search existing issues before opening a new one. For major changes, discuss the scope in an issue first.

## Issues

Use the bug report or feature request template. Include exact versions, reproduction steps, expected and actual behavior. Use small sample files when relevant; remove personal content and credentials from attachments.

## Pull requests

Keep changes focused. Explain the problem and resulting behavior, link related issues, and report the checks you ran and their results. Add regression coverage for behavior changes and update affected documentation. For interface changes, include a screenshot and update translations where needed.

## Development

Run frontend checks from `apps/srt-gui/`:

```sh
npm ci
npm run check
npm test
npm run build
```

From the repository root:

```sh
cargo fmt --all --check
cargo test --workspace --exclude vesta --exclude whisper-bench
```

See [docs/QUALITY.md](docs/QUALITY.md) for desktop checks and release validation.

## Media and performance changes

Use small reproducible fixtures and check the generated media and note fields,
not only whether a command succeeds. For performance claims, record the inputs,
settings, machine, FFmpeg version and repeated timings. Compare identical output
when attributing gains to an optimization; comparisons between product defaults
must describe differences in encoding and media counts. See
[the benchmark guide](docs/BENCHMARK_STEPS.md).

## Translations

Update the English catalog and all supported locales when changing interface
copy. Preserve interpolation tokens and run `npm run check:i18n` in
`apps/srt-gui/`. Product names and formats can remain untranslated.
