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
