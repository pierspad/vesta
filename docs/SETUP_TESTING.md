# Setup and installer testing

This guide describes repeatable manual checks for first-run setup, audio
preparation and updates. Use a disposable account or virtual machine for tests
that install packages or replace saved preferences.

## Preview setup without saving preferences

Open **Experimental → Try without saving**. It opens the first-run setup flow
with quick and custom paths, language, export, audio and transcription choices.
Close it at any point or complete the flow. Preview mode must not save
preferences, change the app language, or download fonts, Whisper or VAD. It
checks the UI flow and choices; it does not simulate downloads.

## Run first-time setup

**Experimental → Restart setup** restarts the app and forces the real setup.
Completing it replaces the preferences managed by setup; API keys and other
preferences remain in their own stores.

Check both quick and custom setup. Quick setup chooses the recommended defaults;
custom setup lets the user select export, audio and local transcription options.
When downloads are requested, verify that existing resources are reused, failed
downloads show an error and can be retried, and completed downloads remain
available. Confirm that preferences are saved only after setup completes.

For an authentic first-run check, use a fresh system account or VM: install
Vesta, launch it, complete setup, close and reopen the app, and confirm setup is
shown only on the first launch. Do not delete the configuration used for normal
work.

## Check sync audio preparation

1. Load an SRT and an MP3; playback should work without conversion.
2. Load an MKV with Opus or Vorbis audio; preparation should first try copying
   the audio track without re-encoding.
3. Try an MKV whose audio cannot be copied into OGG; preparation should fall
   back to audio conversion. Flashcard export must retain its own audio settings.
4. Load the same file again and verify that the completed preparation is reused.
5. Load an invalid media file; expect a visible error and a way to choose another
   file. FFmpeg child processes have a five-minute limit.
6. With no media loaded, verify that the confirmation action is disabled.

First-time preparation depends on media duration and codec. These checks establish
behavior, not a performance result.

## Check installer and internal updates

The first-run setup configures Vesta after installation. The installer places
the application on the system; test those as separate flows.

In dedicated Windows and Linux VMs, install a release package, launch Vesta,
complete setup, close and reopen the app, and uninstall it. Successful package
builds do not verify installer windows or system integration on a real machine.

The internal updater checks the latest stable release, selects an installer for
the supported system and architecture, downloads it, verifies SHA-256 and opens
the installer. The in-app flow supports Windows, DEB and RPM packages; AUR,
Flatpak, Snap and AppImage updates use their package manager or the release page.
A `tauri dev` build does not use the installed-app updater flow.

To test updates, install a previous stable release in a VM, open **Settings →
Overview → Check for updates**, then install the offered update. Verify that the
installer does not launch if download or checksum verification fails. Development
prereleases are installed from the release page; the in-app check offers stable
releases only.
