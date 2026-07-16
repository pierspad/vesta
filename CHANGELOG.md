## [0.14.0-dev.6](https://github.com/pierspad/vesta/compare/v0.14.0-dev.5...v0.14.0-dev.6) (2026-07-16)

### 🐛 Bug Fixes

* **whisper-rs-sys:** include vulkan backend symbols in bundled bindings ([8d1286c](https://github.com/pierspad/vesta/commit/8d1286c11b4c26754a08089d30c2ef52bb4aaba6))

## [0.14.0-dev.5](https://github.com/pierspad/vesta/compare/v0.14.0-dev.4...v0.14.0-dev.5) (2026-07-16)

### 🐛 Bug Fixes

* **branding:** finish VESTA->vesta rename in remaining scripts, packaging metadata and benchmarks ([32543f5](https://github.com/pierspad/vesta/commit/32543f5e625a0920931dab60e7c0408b9cadd914))
* **ci:** stop Arch package build from 404ing on the not-yet-published GitHub release ([88139ab](https://github.com/pierspad/vesta/commit/88139ab980c1bac4416f99bdd7619a4ea6bfd52f))
* **i18n:** add missing AnkiConnect export translations for 13 locales ([4ed2ff0](https://github.com/pierspad/vesta/commit/4ed2ff0c3655797acf419a12c28705fff305e791))
* **i18n:** sync and backfill translation keys across all 15 locales ([4bb14af](https://github.com/pierspad/vesta/commit/4bb14af11c1d0440ea73196e5823a80235a28997))
* **packaging:** align product branding to lowercase vesta and declare Vulkan runtime deps ([806c013](https://github.com/pierspad/vesta/commit/806c0133b2e7944ace6849291848eb19628cea06))

### ✨ New Features

* **autosync:** drive auto-sync transcription through the srt-transcribe VAD pipeline ([cd1ef39](https://github.com/pierspad/vesta/commit/cd1ef399d872c3656241109c8138c77f7ca9733f))
* **experimental:** add condensed-audio generation and direct AnkiConnect import ([0994e10](https://github.com/pierspad/vesta/commit/0994e1005e2e4ad401bb3a1f0b6913bce1996bc0))
* **flashcards:** extend media handling and flashcard generation options ([91534c3](https://github.com/pierspad/vesta/commit/91534c35059eac17bdf51d35707de7e65410d064))
* **refine:** move flashcard AI refinement onto the tiered LLM pool with cancellation ([6690008](https://github.com/pierspad/vesta/commit/669000875ce3185b5b8012d21a86ac5771ef8f87))
* **settings:** add VAD/model management and shared LLM tier helpers ([8f5be2b](https://github.com/pierspad/vesta/commit/8f5be2b8ba79e3291ef31efb52f33ea21c83cd2e))
* **transcribe:** add VAD-gated decoding and cloud transcription providers to srt-transcribe ([ca106ca](https://github.com/pierspad/vesta/commit/ca106ca2e7dbf8360fb19ba6d4dc6818dc8c4637))
* **ui:** update align, sync, translate and shortcuts tabs for the new tiered/experimental features ([97ab28e](https://github.com/pierspad/vesta/commit/97ab28e425d556c0a9762b632edffddf4d3b9585))

## [0.14.0-dev.4](https://github.com/pierspad/vesta/compare/v0.14.0-dev.3...v0.14.0-dev.4) (2026-07-14)

### 🐛 Bug Fixes

* **ci:** skip MSI bundling and sanitize Arch pkgver for pre-release tags ([b79b3bb](https://github.com/pierspad/vesta/commit/b79b3bbac606ed113a95bfe7122512771583b845))

## [0.14.0-dev.3](https://github.com/pierspad/vesta/compare/v0.14.0-dev.2...v0.14.0-dev.3) (2026-07-14)

### 🐛 Bug Fixes

* **branding:** use fireplace logo for all app and window icons ([8ad608a](https://github.com/pierspad/vesta/commit/8ad608aa7a82ffcf91692da46f886881e9276372))
* **ci:** cancel stale queued release runs to avoid version-calc race ([945295b](https://github.com/pierspad/vesta/commit/945295bf5d4197959a26dfe9ba29ebaeae0fa41a))
* **parser:** auto-detect subtitle file encoding (BOM, UTF-16, legacy code pages) ([ba99159](https://github.com/pierspad/vesta/commit/ba991593c14a3b0746ba04254efcc7c68cb0a5ea))
* updated missing/broken translations ([0e39445](https://github.com/pierspad/vesta/commit/0e39445171ffd15770dcf3db4f973f99d03a7672))

### ✨ New Features

* **core:** extract engine logic into reusable library crates and CLI tools ([ba2712a](https://github.com/pierspad/vesta/commit/ba2712a75b768763688db2dbd4815d7e361dcd44))

## [0.14.0-dev.2](https://github.com/pierspad/Vesta/compare/v0.14.0-dev.1...v0.14.0-dev.2) (2026-07-11)

### 🐛 Bug Fixes

* **ci:** restrict AUR publish to stable releases and enable prerelease flag on GitHub ([ce2bf71](https://github.com/pierspad/Vesta/commit/ce2bf71d3357101186f2084353e0ad1581bc60d6))
* **ci:** support suffix-based versions in update_project_info.sh ([98dbaec](https://github.com/pierspad/Vesta/commit/98dbaec8d43abd1e1d40de3922af75bf0fb5df64))

## [0.14.0-dev.1](https://github.com/pierspad/Vesta/compare/v0.13.0...v0.14.0-dev.1) (2026-07-11)

### 🐛 Bug Fixes

* **ci:** add conventional-changelog-conventionalcommits dependency to release action ([01dbd74](https://github.com/pierspad/Vesta/commit/01dbd743b2b054890b6372a760168c0ad55b4035))
* **i18n:** synchronize and backfill missing translation keys across all locales ([165a519](https://github.com/pierspad/Vesta/commit/165a5197121e0f228173cfe84060336c9f06cdd1))
* **pre-push:** correct project root path calculation inside .git/hooks context ([4012505](https://github.com/pierspad/Vesta/commit/4012505f6eadf6bbe54e9732ee8943f38d249cd8))
* **transcribe:** local GPU detection & overlap preset controls ([9a17738](https://github.com/pierspad/Vesta/commit/9a17738bb1a9c68c71b00375e263d3ef2747cd0f))

### ✨ New Features

* **ci:** integrate semantic-release for fully automated release builds ([2205776](https://github.com/pierspad/Vesta/commit/2205776fb3626585bd04b00ae43cc729f920d428))
* complete release automation, layout polishing, and benchmark improvements ([803b123](https://github.com/pierspad/Vesta/commit/803b1238228ea4a896f98297ee34980b4bd15b3c))
* **core:** extraction of headless flashcard engine & expert mode UI ([0bcee31](https://github.com/pierspad/Vesta/commit/0bcee31b744913c48804ea6754d682334da3fa03))
* **icons,desktop:** move desktop entries to build-scripts and use stylized woman logo as app icon; add git pre-push hooks ([a8790e6](https://github.com/pierspad/Vesta/commit/a8790e6a3cef6f21e812af55d38b8623c02e74bf))
* **settings:** physical RAM retrieval & temporary subtitle exporter ([7d3d412](https://github.com/pierspad/Vesta/commit/7d3d412dcec0ef1c062027333500af216b84a2e8))
* **transcribe:** C-level Whisper abort callback & shimmer progress indicators ([af4b507](https://github.com/pierspad/Vesta/commit/af4b507e7e127b4ca26131941a80e1d8fba25a69))
