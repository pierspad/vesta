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
