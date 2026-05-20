## Release Notes

### Fixes

* **Release Publishing**: GitHub Releases now use the tag-specific section from `docs/release-notes.md` when available
* **CI Warnings**: GitHub Actions now use Node.js 24-compatible action versions to avoid Node.js 20 deprecation warnings
* **Localization Checks**: Release and CI validation now block missing locale files, missing keys, empty translations, and placeholder mismatches across all 15 UI languages
* **UI Bugfixes**: Resolved issues in the synchronization tab and corrected various UI element inconsistencies
* **Audio Synchronization**: Fixed audio-related bugs affecting sync behavior
* **Notifications**: Now notification will remain enabled/disabled as per user choice
* **Translation**: i18n files aligned

### Improvements

* **Release Automation**: The release script now automatically formats the release notes version header, and cleans/resets both the operational changelog and the release notes files upon successful release
* **Dutch Localization**: Added Dutch (`nl`) UI and help translations, matching TextMerger's 15-language set
* **Release Scripts**: Aligned release and AUR publishing scripts with TextMerger's validation-first structure
* **Project Governance**: LLM/Copilot instructions now require curated release notes, synchronized i18n, reusable components, and technical debt reduction
* **Settings Menu**: Improved clarity and usability
* **Efficiency**: Optimized performance across the interface
