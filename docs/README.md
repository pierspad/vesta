https://archive.org/details/detour1945HD

link per scaricare il film Detour (1945) direttamente dall'archive.org

# Developer Guide

## Build and Release Scripts

From the project root, use the helper scripts in `build-scripts/` for release work:

```bash
# Run GUI quickly in dev mode
sh run_gui.sh

# Sync versions/metadata from build-scripts/PKGBUILD
sh build-scripts/update_project_info.sh

# Verify internal crate version consistency
sh build-scripts/check_internal_crate_versions.sh

# Audit UI translations across all 15 locales
(cd apps/srt-gui && npm run i18n:audit)

# Build local Arch package from freshly built .deb
sh build-scripts/build-aur.sh

# Guided release flow
sh build-scripts/git-release.sh

# Preview the release notes section that will be published for a tag
sh build-scripts/extract-release-notes.sh v0.7.1
```

## Agent / LLM Guidelines

### Release Notes and Versioning
1. **Do Not Delete Release Notes After Release**: The `git-release.sh` script does not delete or reset `docs/release-notes.md` or `docs/list_of_things_changed.md`.
2. **Detecting Post-Release State**: When making a new change, check the version number in `build-scripts/PKGBUILD` and the version number in `docs/release-notes.md` (e.g. `## Release Notes v0.8.0` or similar). If they are equal, it indicates that the current version was already released.
3. **Handling the Post-Release Change**:
   - You must first increment/bump the version in `build-scripts/PKGBUILD` (e.g., from `0.8.0` to `0.8.1`).
   - Run `build-scripts/update_project_info.sh` to propagate this new version across all project files.
   - Overwrite/update `docs/release-notes.md` and `docs/list_of_things_changed.md` with the new version section and list the new changes under preparation.

