# SPDX-License-Identifier: GPL-3.0-or-later
"""Subtitles Vesta Plugin Controller.

Coordinates screens, events, OpenSubtitles search, local track extraction,
and spaced-repetition flashcards integration.
Emits declarative widget UI schemas and single-object PluginPatch updates
compliant with the Vesta Plugin System Specification (v1).
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any, Callable

from subtitles_plugin.config import ConfigManager, OpenSubtitlesCredentials
from subtitles_plugin.extractor import ExtractionResult, LocalSubExtractor, SubtitleTrackInfo
from subtitles_plugin.opensubtitles import OpenSubtitlesClient, SubtitleItem
from subtitles_plugin.schema import (
    apply_patches,
    button,
    checkbox,
    column,
    divider,
    external_link,
    file_output,
    heading,
    language_select,
    link,
    list_item,
    list_widget,
    make_patch,
    password_input,
    path_input,
    progress,
    row,
    segmented,
    select,
    text,
    text_input,
)


class SubtitlesPlugin:
    """Core state and screen controller for the Subtitles Vesta plugin."""

    def __init__(
        self,
        config_manager: ConfigManager | None = None,
        opensubtitles_client: OpenSubtitlesClient | None = None,
        extractor: LocalSubExtractor | None = None,
        notifier: Callable[[str, dict[str, Any]], None] | None = None,
        requester: Callable[[str, dict[str, Any]], Any] | None = None,
    ):
        self.config_manager = config_manager or ConfigManager()
        self._custom_client = opensubtitles_client
        self._custom_extractor = extractor
        self.notifier = notifier
        self.requester = requester

        # Navigation & Screen State
        self.current_screen: str = "search"  # 'search', 'local', 'settings'
        self.status_message: str | None = None
        self.status_variant: str = "normal"  # 'normal', 'muted', 'danger', 'success'

        # Search Screen State
        self.search_query: str = ""
        self.search_tmdb_id: str = ""
        self.search_imdb_id: str = ""
        self.search_languages: str = ""
        self.search_results: list[SubtitleItem] = []
        self.selected_subtitle: SubtitleItem | None = None
        self.downloaded_subtitle_path: str | None = None
        self.associated_video_path: str = ""

        # Local File Extraction State
        self.local_video_path: str = ""
        self.local_tracks: list[SubtitleTrackInfo] = []
        self.extracted_subtitle_path: str | None = None
        self.last_extraction_result: ExtractionResult | None = None

    @property
    def opensubtitles_client(self) -> OpenSubtitlesClient:
        return self.create_client()

    def create_client(self, credentials: OpenSubtitlesCredentials | None = None) -> OpenSubtitlesClient:
        """Returns the configured OpenSubtitles client (or custom mock client)."""
        if self._custom_client:
            return self._custom_client
        creds = credentials or self.config_manager.resolve_credentials()
        return OpenSubtitlesClient(credentials=creds)

    @property
    def extractor(self) -> LocalSubExtractor:
        if self._custom_extractor:
            return self._custom_extractor
        return LocalSubExtractor(
            ffmpeg_bin=self.config_manager.settings.ffmpeg_path or "ffmpeg",
            ffprobe_bin=self.config_manager.settings.ffprobe_path or "ffprobe",
            dry_run=self.config_manager.resolve_dry_run(),
        )


    def emit_ui_update(
        self,
        screen: str = "search",
        patch: dict[str, Any] | None = None,
        root: dict[str, Any] | None = None,
    ) -> None:
        """Emits a 'ui.update' JSON-RPC notification to the Vesta host."""
        if not (self.notifier and callable(self.notifier)):
            return

        params: dict[str, Any] = {"screen": screen}
        if patch is not None:
            unwrapped = patch.get("patch", patch) if isinstance(patch, dict) else patch
            params["patch"] = unwrapped
            params["patches"] = unwrapped
        if root is not None:
            params["root"] = root

        self.notifier("ui.update", params)

    def notify(self, msg: str, level: str = "info") -> None:
        """Emits a standard toast 'ui.notify' to Vesta (info, success, warn, error)."""
        if self.notifier and callable(self.notifier):
            self.notifier("ui.notify", {"level": level, "msg": msg})

    def report_progress(self, id: str, value: float, label: str = "") -> None:
        """Emits a 'ui.progress' notification to Vesta."""
        if self.notifier and callable(self.notifier):
            self.notifier("ui.progress", {"id": id, "value": value, "label": label})

    def open_in_flashcards(self, video_path: str, subtitle_path: str) -> None:
        """Emits 'vesta.openInFlashcards' notification to switch to Flashcards."""
        if self.notifier and callable(self.notifier):
            self.notifier(
                "vesta.openInFlashcards",
                {"videoPath": str(video_path), "subtitlePath": str(subtitle_path)},
            )

    def get_values(self, screen: str = "search") -> dict[str, Any]:
        """Queries interactive screen values from Vesta via 'ui.getValues'."""
        if self.requester and callable(self.requester):
            res = self.requester("ui.getValues", {"screen": screen})
            if isinstance(res, dict) and "values" in res:
                return res["values"]
            return res if isinstance(res, dict) else {}
        return {}

    # Lifecycle

    def initialize(
        self,
        api_version: int = 1,
        data_dir: str | None = None,
        locale: str = "en",
        settings: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Handles Vesta 'initialize' method."""
        if data_dir:
            self.config_manager.set_data_dir(Path(data_dir))
        if settings and isinstance(settings, dict):
            self.config_manager.update_settings(settings)

        return {
            "status": "ok",
            "ready": True,
            "initialScreen": "main",
            "root": self.render_screen("search"),
        }

    # Screen Rendering

    def _finalize_root(self, root: dict[str, Any]) -> dict[str, Any]:
        """Attach a host snackbar notification to the root widget."""
        if self.status_message:
            stype = (
                "error"
                if self.status_variant == "danger"
                else ("success" if self.status_variant == "success" else "info")
            )
            root["snackbar"] = {"message": self.status_message, "type": stype}
        return root

    def render_screen(self, screen: str, values: dict[str, Any] | None = None) -> dict[str, Any]:
        """Renders declarative widget tree for the requested screen."""
        normalized = "search" if screen in ("main", "search") else screen
        self.current_screen = normalized

        if normalized == "search":
            root = self._render_search_screen(values)
        elif normalized == "local":
            root = self._render_local_screen(values)
        elif normalized == "settings":
            root = self._render_settings_screen(values)
        else:
            root = self._render_search_screen(values)
        return self._finalize_root(root)

    def _settings_action(self) -> dict[str, Any]:
        key_ready = self.config_manager.resolve_credentials().is_configured
        return button(
            "btn_nav_settings", "Settings", variant="settings",
            status=None if key_ready else "attention",
            tooltip="OpenSubtitles key configured" if key_ready else "Add an OpenSubtitles API key",
            icon="settings", placement="header",
        )

    def _render_status_banner(self) -> dict[str, Any]:
        """Renders status message banner (hidden in favor of Vesta host snackbars)."""
        return text("", id="status_banner", hidden=True)

    # Screen: Search

    def _render_search_screen(self, values: dict[str, Any] | None = None) -> dict[str, Any]:
        creds = self.config_manager.resolve_credentials()
        pref_langs = ",".join(self.config_manager.resolve_preferred_languages())

        children: list[dict[str, Any]] = [
            self._settings_action(),
            self._render_status_banner(),
        ]

        # Search Input Controls
        children.extend(
            [
                row(
                    [
                        language_select(
                            "search_languages",
                            label="Language",
                            names="both",
                            defaultValue=self.search_languages or pref_langs,
                            flex=1,
                            tooltip="Filter subtitles by language",
                        ),
                        text_input(
                            "search_query",
                            label="Title",
                            placeholder="e.g. Inception, The Matrix, or Interstellar",
                            defaultValue=self.search_query,
                            flex=3,
                            icon="search",
                            submitAction="btn_do_search",
                        ),
                        button(
                            "btn_do_search",
                            "Search",
                            variant="accent",
                            icon="search",
                            tooltip="Enter a title, TMDB ID, or IMDb ID to search",
                            disabled=not creds.is_configured,
                            disabledTooltip="Open Settings and add an OpenSubtitles API key to search",
                            disabledFocus="btn_nav_settings",
                            requiresAny=["search_query", "search_tmdb_id", "search_imdb_id"],
                        ),
                        button(
                            "btn_clear_search",
                            "Clear",
                            variant="danger",
                            icon="trash",
                            tooltip="Clear search filters and results",
                            requiresAny=[] if self.search_results or self.selected_subtitle else [
                                "search_query", "search_tmdb_id", "search_imdb_id",
                            ],
                        ),
                    ],
                    gap=3,
                    align="end",
                ),
                row(
                    [
                        text_input(
                            "search_tmdb_id",
                            label="ID",
                            placeholder="e.g. 27205",
                            defaultValue=self.search_tmdb_id,
                            flex=1,
                            icon="tmdb",
                            labelTooltip="TMDB ID",
                            tooltip="TMDB ID",
                            submitAction="btn_do_search",
                        ),
                        text_input(
                            "search_imdb_id",
                            label="ID",
                            placeholder="e.g. tt1375666",
                            defaultValue=self.search_imdb_id,
                            flex=1,
                            icon="imdb",
                            labelTooltip="IMDb ID",
                            tooltip="IMDb ID",
                            submitAction="btn_do_search",
                        ),
                    ],
                    gap=3,
                    align="end",
                    id="search_id_alternatives",
                    separator="or",
                ),
                divider(),
            ]
        )

        # Search Results List
        if self.search_results:
            items: list[dict[str, Any]] = []
            for sub in self.search_results:
                title_line = f"{sub.movie_name} - {sub.release}" if sub.release else (sub.movie_name or sub.primary_file_name)
                rating_str = f"{sub.ratings:.1f}" if sub.ratings else "N/A"
                hi_str = "Yes" if sub.hearing_impaired else "No"
                sub_line = (
                    f"Downloads: {sub.download_count:,} • "
                    f"Rating: {rating_str} • HI: {hi_str} • Release: {sub.release or 'Standard'}"
                )
                items.append(
                    list_item(
                        id=f"item_{sub.id}",
                        title=title_line,
                        subtitle=sub_line,
                        language=sub.language.lower() if sub.language else None,
                        badge=sub.badge_text,
                        actions=[
                            {"id": f"action_select_{sub.id}", "label": "Select", "variant": "secondary", "tooltip": "Select this subtitle track"},
                        ],
                    )
                )

            children.extend(
                [
                    heading(f"Results ({len(self.search_results)})", level=3),
                    list_widget("search_results_list", items=items),
                    divider(),
                ]
            )
        elif self.search_query:
            children.extend(
                [
                    text("No subtitles found matching query.", variant="muted"),
                    divider(),
                ]
            )

        # Selected Subtitle & Download Section
        if self.selected_subtitle:
            sub = self.selected_subtitle
            dest_dir = str(self.config_manager.resolve_download_dir())
            rating_str = f"{sub.ratings:.1f}" if sub.ratings else "N/A"

            children.extend(
                [
                    heading(f"Selected: {sub.primary_file_name}", level=3),
                    column(
                        [
                            text(f"Movie / Title: {sub.movie_name or 'N/A'}"),
                            text(f"Release: {sub.release or 'Standard'}"),
                            text(f"Language: {sub.language.upper()} | Downloads: {sub.download_count:,} | Rating: {rating_str}"),
                            text(f"Hearing Impaired: {'Yes' if sub.hearing_impaired else 'No'} | Trusted Source: {'Yes' if sub.from_trusted else 'No'}"),
                        ],
                        gap=1,
                    ),
                    row(
                        [
                            path_input(
                                "download_dest_dir",
                                label="Destination Folder",
                                defaultValue=dest_dir,
                                picker="directory",
                                mode="directory",
                            ),
                            text_input(
                                "download_custom_name",
                                label="File Name",
                                defaultValue=sub.primary_file_name,
                            ),
                        ],
                        gap=3,
                    ),
                    row(
                        [
                            button("btn_download_now", "Download Subtitle", variant="primary", icon="download"),
                        ],
                        gap=2,
                    ),
                    divider(),
                ]
            )

        # Downloaded Subtitle & Flashcards Handoff
        if self.downloaded_subtitle_path:
            children.extend(
                [
                    heading("Subtitle Downloaded", level=3),
                    file_output(self.downloaded_subtitle_path, label="Saved Subtitle (.srt)"),
                    path_input(
                        "video_for_flashcards",
                        label="Associated Video File (optional, for Flashcards)",
                        placeholder="Path to matching video file...",
                        defaultValue=self.associated_video_path,
                        picker="file",
                        mode="file",
                    ),
                    row(
                        [
                            button("btn_use_in_flashcards", "Use in Flashcards", variant="primary"),
                        ],
                        gap=2,
                    ),
                ]
            )

        return column(children, gap=3, id="search_screen")

    # Screen: Local Video File Extraction

    def _render_local_screen(self, values: dict[str, Any] | None = None) -> dict[str, Any]:
        bin_status = self.extractor.check_binaries()
        ffprobe_ok = bin_status.get("ffprobe", False)
        ffmpeg_ok = bin_status.get("ffmpeg", False)

        children: list[dict[str, Any]] = [
            self._render_status_banner(),
        ]

        # Status of FFmpeg tools
        if not (ffprobe_ok and ffmpeg_ok):
            children.append(
                column(
                    [
                        text(
                            "FFmpeg / ffprobe tools were not found on your system PATH. "
                            "Extraction of embedded tracks requires FFmpeg.",
                            variant="danger",
                        ),
                        text(
                            f"Status: ffprobe={'Found' if ffprobe_ok else 'Missing'}, "
                            f"ffmpeg={'Found' if ffmpeg_ok else 'Missing'}",
                            variant="muted",
                            size="sm",
                        ),
                    ],
                    gap=1,
                    id="ffmpeg_warning_box",
                )
            )
        else:
            children.append(
                text(
                    "Inspect embedded subtitle tracks (SubRip, ASS, PGS) and extract to .srt.",
                    variant="muted",
                    size="sm",
                )
            )

        # Input video file
        children.extend(
            [
                path_input(
                    "local_video_path",
                    label="Video File Path",
                    placeholder="/path/to/movie.mkv or C:\\media\\movie.mp4",
                    defaultValue=self.local_video_path,
                    picker="file",
                    mode="file",
                ),
                row(
                    [
                        button("btn_inspect_tracks", "Inspect Tracks", variant="primary", icon="search"),
                    ],
                    gap=2,
                ),
                divider(),
            ]
        )

        # Discovered embedded tracks list
        if self.local_tracks:
            items: list[dict[str, Any]] = []
            for track in self.local_tracks:
                title_line = f"Track #{track.index} [{track.language.upper()}] - {track.codec_name}"
                sub_line = track.title or f"Embedded subtitle stream #{track.subtitle_index}"
                items.append(
                    list_item(
                        id=f"track_item_{track.index}",
                        title=title_line,
                        subtitle=sub_line,
                        badge=f"Stream #{track.index}",
                        actions=[
                            {"id": f"action_extract_{track.index}", "label": "Extract to .srt", "variant": "primary"},
                        ],
                    )
                )

            children.extend(
                [
                    heading(f"Embedded Subtitle Tracks ({len(self.local_tracks)})", level=3),
                    list_widget("local_tracks_list", items=items),
                    divider(),
                ]
            )
        elif self.local_video_path and not self.local_tracks:
            children.extend(
                [
                    text("No subtitle tracks detected in container.", variant="muted"),
                    divider(),
                ]
            )

        # Extraction result & Flashcards Handoff
        if self.extracted_subtitle_path:
            is_dry = bool(self.last_extraction_result and self.last_extraction_result.dry_run)
            children.extend(
                [
                    heading(
                        "Extracted Subtitle Ready" if not is_dry else "[Dry-run] Extraction Command Generated",
                        level=3,
                    ),
                    file_output(self.extracted_subtitle_path, label="Subtitle File (.srt)"),
                    row(
                        [
                            button("btn_use_in_flashcards_local", "Use in Flashcards", variant="primary"),
                        ],
                        gap=2,
                    ),
                ]
            )

        return column(children, gap=3, id="local_screen", expertOnly=True)

    # Screen: Settings

    def _render_settings_screen(self, values: dict[str, Any] | None = None) -> dict[str, Any]:
        from subtitles_plugin.config import mask_secret

        creds = self.config_manager.resolve_credentials()
        settings = self.config_manager.settings

        children: list[dict[str, Any]] = []
        children.append(
            password_input(
                "cfg_api_key",
                label="OpenSubtitles API Key",
                defaultValue=creds.api_key or "",
                emptyWarning="Enter an OpenSubtitles Consumer API key.",
                submitAction="btn_save_settings",
                link={"label": "Where do I find this key?", "url": "https://www.opensubtitles.com/en/consumers"},
            )
        )

        saved_keys = self._saved_keys()

        if saved_keys:
            key_items: list[dict[str, Any]] = []
            for idx, k in enumerate(saved_keys):
                is_active = k == (settings.opensubtitles_api_key or creds.api_key)
                actions: list[dict[str, Any]] = []
                if not is_active:
                    actions.append({"id": f"action_use_key_{idx}", "label": "Use", "variant": "primary"})
                actions.append({"id": f"action_delete_key_{idx}", "label": "Remove", "icon": "trash", "iconOnly": True, "variant": "danger"})
                key_items.append(
                    list_item(
                        id=f"saved_key_{idx}",
                        title=mask_secret(k),
                        badge="Active" if is_active else None,
                        actions=actions,
                    )
                )
            children.append(list_widget("saved_keys_list", items=key_items))

        children.extend(
            [
                row(
                    [
                        language_select(
                            "cfg_preferred_languages",
                            label="Preferred Language",
                            names="both",
                            defaultValue=settings.preferred_languages or "en",
                            flex=1,
                        ),
                        path_input(
                            "cfg_default_download_dir",
                            label="Default Download Directory",
                            defaultValue=settings.default_download_dir,
                            placeholder="Default: downloads folder",
                            picker="directory",
                            mode="directory",
                            flex=2,
                        ),
                    ],
                    gap=3,
                ),
                row(
                    [
                        text_input(
                            "cfg_username",
                            label="Username (optional)",
                            placeholder="OpenSubtitles username",
                            defaultValue=settings.opensubtitles_username or (creds.username or ""),
                            flex=1,
                            expertOnly=True,
                        ),
                        password_input(
                            "cfg_password",
                            label="Password (optional)",
                            placeholder="OpenSubtitles password",
                            defaultValue=settings.opensubtitles_password or (creds.password or ""),
                            flex=1,
                            expertOnly=True,
                        ),
                    ],
                    gap=3,
                    expertOnly=True,
                ),
                row(
                    [
                        text_input(
                            "cfg_ffmpeg_path",
                            label="Custom FFmpeg Path (optional)",
                            defaultValue=settings.ffmpeg_path,
                            placeholder="ffmpeg",
                            flex=1,
                            expertOnly=True,
                        ),
                        text_input(
                            "cfg_ffprobe_path",
                            label="Custom ffprobe Path (optional)",
                            defaultValue=settings.ffprobe_path,
                            placeholder="ffprobe",
                            flex=1,
                            expertOnly=True,
                        ),
                    ],
                    gap=3,
                    expertOnly=True,
                ),
                checkbox(
                    "cfg_dry_run_default",
                    label="Dry-run mode (simulate extraction and downloads)",
                    defaultValue=settings.dry_run_default,
                    expertOnly=True,
                ),
                divider(),
                row(
                    [
                        button(
                            "btn_save_settings", "Save Settings", variant="primary", icon="save",
                            requiresAny=["cfg_api_key"], disabledFocus="cfg_api_key",
                        ),
                        button(
                            "btn_test_credentials", "Test API Connection", variant="secondary", icon="refresh",
                            requiresAny=["cfg_api_key"], disabledFocus="cfg_api_key",
                        ),
                    ],
                    gap=2,
                ),
                divider(expertOnly=True),
                heading("Local Video Track Extraction", level=3, expertOnly=True),
                row(
                    [
                        path_input(
                            "local_video_path",
                            label="Video File Path",
                            placeholder="/path/to/movie.mkv",
                            defaultValue=self.local_video_path,
                            picker="file",
                            mode="file",
                            flex=3,
                            expertOnly=True,
                        ),
                        button(
                            "btn_inspect_tracks",
                            "Inspect Tracks",
                            variant="secondary",
                            icon="search",
                            expertOnly=True,
                        ),
                    ],
                    gap=3,
                    align="end",
                    expertOnly=True,
                ),
            ]
        )

        if self.local_tracks:
            track_items: list[dict[str, Any]] = []
            for track in self.local_tracks:
                title_line = f"Track #{track.index} [{track.language.upper()}] - {track.codec_name}"
                sub_line = track.title or f"Stream #{track.subtitle_index}"
                track_items.append(
                    list_item(
                        id=f"track_item_{track.index}",
                        title=title_line,
                        subtitle=sub_line,
                        badge=f"#{track.index}",
                        actions=[
                            {"id": f"action_extract_{track.index}", "label": "Extract .srt", "variant": "primary"},
                        ],
                    )
                )
            children.append(list_widget("local_tracks_list", items=track_items, expertOnly=True))

        if self.extracted_subtitle_path:
            children.extend(
                [
                    file_output(self.extracted_subtitle_path, label="Extracted Subtitle (.srt)", expertOnly=True),
                    button("btn_use_in_flashcards_local", "Use in Flashcards", variant="primary", expertOnly=True),
                ]
            )

        return column(children, gap=3, id="settings_screen")

    # Event Handling

    def handle_event(
        self,
        screen: str,
        widget_id: str,
        event_type: str,
        values: dict[str, Any],
    ) -> dict[str, Any]:
        destination = {
            "btn_nav_search": "main",
            "btn_nav_local": "local",
            "btn_nav_settings": "settings",
        }.get(widget_id)
        if destination:
            self.status_message = None
            return {"screen": destination, "root": self.render_screen(destination, values)}

        if event_type == "submit" and widget_id in ("search_query", "search_tmdb_id", "search_imdb_id"):
            widget_id = "btn_do_search"

        actions = {
            "btn_do_search": self._handle_search_event,
            "btn_clear_search": self._clear_search,
            "btn_download_now": self._handle_download_event,
            "btn_use_in_flashcards": self._handle_flashcards_event,
            "btn_inspect_tracks": self._handle_inspect_tracks,
            "btn_use_in_flashcards_local": self._handle_flashcards_local_event,
            "btn_save_settings": self._handle_save_settings,
            "btn_test_credentials": self._handle_test_credentials,
        }
        if handler := actions.get(widget_id):
            return handler(values)

        for prefix, handler in (
            ("action_select_", self._handle_select_subtitle),
            ("action_extract_", self._handle_extract_track),
            ("action_use_key_", self._use_key),
            ("action_delete_key_", self._remove_key),
        ):
            if widget_id.startswith(prefix):
                return handler(widget_id.removeprefix(prefix), values)
        return {}

    def _clear_search(self, values: dict[str, Any]) -> dict[str, Any]:
        self.search_query = self.search_tmdb_id = self.search_imdb_id = ""
        self.search_results = []
        self.selected_subtitle = None
        self.status_message = None
        return {
            "root": self.render_screen("search", values),
            "patch": {"values": {"search_query": "", "search_tmdb_id": "", "search_imdb_id": ""}},
        }

    def _saved_keys(self) -> list[str]:
        settings = self.config_manager.settings
        keys = list(settings.saved_api_keys)
        if settings.opensubtitles_api_key and settings.opensubtitles_api_key not in keys:
            keys.insert(0, settings.opensubtitles_api_key)
        return keys

    def _use_key(self, index: str, values: dict[str, Any]) -> dict[str, Any]:
        return self._change_saved_key(index, values, remove=False)

    def _remove_key(self, index: str, values: dict[str, Any]) -> dict[str, Any]:
        return self._change_saved_key(index, values, remove=True)

    def _change_saved_key(
        self, index: str, values: dict[str, Any], *, remove: bool
    ) -> dict[str, Any]:
        keys = self._saved_keys()
        try:
            idx = int(index)
        except ValueError:
            return {}
        if not 0 <= idx < len(keys):
            return {}

        settings = self.config_manager.settings
        old_key, old_keys = settings.opensubtitles_api_key, settings.saved_api_keys
        selected = keys[idx]
        if remove:
            keys = [key for key in keys if key != selected]
            new_key = (keys[0] if keys else "") if selected == old_key else old_key
        else:
            new_key = selected

        settings.opensubtitles_api_key = new_key
        settings.saved_api_keys = keys
        if not self.config_manager.save_settings():
            settings.opensubtitles_api_key, settings.saved_api_keys = old_key, old_keys
            return {"snackbar": {"message": "Could not save API key changes.", "type": "error"}}

        self.status_message = None
        response = {
            "root": self.render_screen("settings"),
            "snackbar": {
                "message": "Removed saved API key." if remove else "Switched active API key.",
                "type": "success",
            },
        }
        if not remove or selected == old_key or values.get("cfg_api_key") == selected:
            response["patch"] = {"values": {"cfg_api_key": new_key}}
        return response

    def _handle_search_event(self, values: dict[str, Any]) -> dict[str, Any]:
        self.search_query = str(values.get("search_query", "")).strip()
        self.search_tmdb_id = str(values.get("search_tmdb_id", "")).strip()
        self.search_imdb_id = str(values.get("search_imdb_id", "")).strip()
        self.search_languages = str(values.get("search_languages", "")).strip()

        if not (self.search_query or self.search_tmdb_id or self.search_imdb_id):
            self.status_message = "Please enter a search query, TMDB ID, or IMDb ID."
            self.status_variant = "danger"
            self.notify(self.status_message, level="warn")
            return {"root": self.render_screen("search", values)}

        self.report_progress("search_op", 25.0, "Searching OpenSubtitles...")

        try:
            results = self.opensubtitles_client.search(
                query=self.search_query,
                tmdb_id=self.search_tmdb_id or None,
                imdb_id=self.search_imdb_id or None,
                languages=self.search_languages or None,
                exclude_machine_and_ai=True,
            )
            self.search_results = results
            self.selected_subtitle = None
            self.status_message = None
            self.status_variant = "normal"
            self.report_progress("search_op", 100.0, "Search completed")
        except Exception as e:
            self.search_results = []
            self.status_message = f"Search failed: {e}"
            self.status_variant = "danger"
            self.notify(self.status_message, level="error")

        return {"root": self.render_screen("search", values)}

    def _handle_select_subtitle(self, sub_id: str, values: dict[str, Any]) -> dict[str, Any]:
        match = next((s for s in self.search_results if s.id == sub_id), None)
        if match:
            self.selected_subtitle = match
            self.status_message = f"Selected '{match.primary_file_name}'"
            self.status_variant = "normal"
        return {"root": self.render_screen("search", values)}

    def _handle_download_event(self, values: dict[str, Any]) -> dict[str, Any]:
        if not self.selected_subtitle:
            self.status_message = "No subtitle selected for download."
            self.status_variant = "danger"
            return {"root": self.render_screen("search", values)}

        fid = self.selected_subtitle.primary_file_id
        if fid is None:
            self.status_message = "Selected subtitle does not have an associated file ID."
            self.status_variant = "danger"
            self.notify(self.status_message, level="error")
            return {"root": self.render_screen("search", values)}

        dest_dir = values.get("download_dest_dir") or str(self.config_manager.resolve_download_dir())
        custom_name = values.get("download_custom_name") or self.selected_subtitle.primary_file_name

        self.report_progress("dl_op", 30.0, f"Downloading {custom_name}...")
        try:
            is_dry = self.config_manager.resolve_dry_run()
            if is_dry:
                out_path = Path(dest_dir).resolve() / custom_name
                self.downloaded_subtitle_path = str(out_path)
                self.status_message = f"[Dry-run] Simulated download of {custom_name} to {out_path}."
                self.status_variant = "success"
                self.notify(self.status_message, level="info")
            else:
                out_path = self.opensubtitles_client.download_subtitle(
                    file_id=fid,
                    destination_dir=dest_dir,
                    custom_filename=custom_name,
                )
                self.downloaded_subtitle_path = str(out_path)
                self.status_message = f"Downloaded subtitle saved to {out_path}."
                self.status_variant = "success"
                self.notify(self.status_message, level="success")

            self.report_progress("dl_op", 100.0, "Download complete")
        except Exception as e:
            self.status_message = f"Download failed: {e}"
            self.status_variant = "danger"
            self.notify(self.status_message, level="error")

        return {"root": self.render_screen("search", values)}

    def _handle_flashcards_event(self, values: dict[str, Any]) -> dict[str, Any]:
        vid_path = str(values.get("video_for_flashcards") or self.associated_video_path or "").strip()
        sub_path = str(self.downloaded_subtitle_path or "").strip()

        if not sub_path:
            self.status_message = "No subtitle file available to load into Flashcards."
            self.status_variant = "danger"
            self.notify(self.status_message, level="warn")
            return {"root": self.render_screen("search", values)}

        self.associated_video_path = vid_path
        self.open_in_flashcards(video_path=vid_path, subtitle_path=sub_path)
        self.status_message = f"Emitted vesta.openInFlashcards with {Path(sub_path).name}"
        self.status_variant = "success"
        self.notify("Loaded subtitle and video into Vesta Flashcards tab!", level="success")

        # Emit live patch update for instant feedback
        patch = make_patch(
            set_props={
                "status_banner": {"text": self.status_message, "variant": "success", "hidden": False}
            }
        )
        self.emit_ui_update(screen="search", patch=patch)
        return {"patch": patch}

    def _handle_inspect_tracks(self, values: dict[str, Any]) -> dict[str, Any]:
        target_screen = self.current_screen if self.current_screen in ("settings", "local") else "local"
        raw_path = str(values.get("local_video_path") or "").strip()
        if not raw_path:
            self.status_message = "Please provide an absolute path to a video file."
            self.status_variant = "danger"
            self.notify(self.status_message, level="warn")
            return {"root": self.render_screen(target_screen, values)}

        self.local_video_path = raw_path
        self.report_progress("probe_op", 20.0, "Inspecting subtitle streams...")

        try:
            tracks = self.extractor.inspect_video_tracks(self.local_video_path)
            self.local_tracks = tracks
            self.extracted_subtitle_path = None
            self.status_message = f"Found {len(tracks)} embedded subtitle track(s)."
            self.status_variant = "success"
            self.notify(self.status_message, level="info")
            self.report_progress("probe_op", 100.0, "Inspection complete")
        except Exception as e:
            self.local_tracks = []
            self.status_message = f"Inspection error: {e}"
            self.status_variant = "danger"
            self.notify(self.status_message, level="error")

        return {"root": self.render_screen(target_screen, values)}

    def _handle_extract_track(self, track_idx_str: str, values: dict[str, Any]) -> dict[str, Any]:
        target_screen = self.current_screen if self.current_screen in ("settings", "local") else "local"
        try:
            stream_idx = int(track_idx_str)
        except ValueError:
            self.status_message = f"Invalid track index: {track_idx_str}"
            self.status_variant = "danger"
            return {"root": self.render_screen(target_screen, values)}

        self.report_progress("extract_op", 30.0, f"Extracting track #{stream_idx}...")
        res = self.extractor.extract_track(
            video_path=self.local_video_path,
            stream_index=stream_idx,
            output_dir=self.config_manager.resolve_download_dir(),
        )
        self.last_extraction_result = res

        if res.success:
            self.extracted_subtitle_path = res.output_path
            self.status_message = res.message
            self.status_variant = "success"
            self.notify(res.message, level="success")
            self.report_progress("extract_op", 100.0, "Extraction complete")
        else:
            self.status_message = res.message
            self.status_variant = "danger"
            self.notify(res.message, level="error")

        return {"root": self.render_screen(target_screen, values)}

    def _handle_flashcards_local_event(self, values: dict[str, Any]) -> dict[str, Any]:
        target_screen = self.current_screen if self.current_screen in ("settings", "local") else "local"
        vid_path = str(self.local_video_path or values.get("local_video_path", "")).strip()
        sub_path = str(self.extracted_subtitle_path or "").strip()

        if not sub_path:
            self.status_message = "No extracted subtitle track available."
            self.status_variant = "danger"
            self.notify(self.status_message, level="warn")
            return {"root": self.render_screen(target_screen, values)}

        self.open_in_flashcards(video_path=vid_path, subtitle_path=sub_path)
        self.status_message = f"Loaded extracted subtitle into Flashcards ({Path(sub_path).name})"
        self.status_variant = "success"
        self.notify("Loaded extracted track into Vesta Flashcards tab!", level="success")

        patch = make_patch(
            set_props={
                "status_banner": {"text": self.status_message, "variant": "success", "hidden": False}
            }
        )
        self.emit_ui_update(screen=target_screen, patch=patch)
        return {"patch": patch}

    def _handle_save_settings(self, values: dict[str, Any]) -> dict[str, Any]:
        api_key = str(values.get("cfg_api_key") or "").strip()
        if not api_key:
            return {"snackbar": {"message": "Enter an OpenSubtitles Consumer API key.", "type": "warning"}}

        updates = {
            "opensubtitles_api_key": api_key,
            "opensubtitles_username": values.get("cfg_username", ""),
            "opensubtitles_password": values.get("cfg_password", ""),
            "preferred_languages": values.get("cfg_preferred_languages", "en"),
            "default_download_dir": values.get("cfg_default_download_dir", ""),
            "dry_run_default": values.get("cfg_dry_run_default", False),
            "ffmpeg_path": values.get("cfg_ffmpeg_path", ""),
            "ffprobe_path": values.get("cfg_ffprobe_path", ""),
        }
        self.config_manager.update_settings(updates, save=True)

        self.status_message = None
        root = self.render_screen("settings")
        root["snackbar"] = {"message": "Settings saved successfully.", "type": "success"}
        return {"root": root}

    def _handle_test_credentials(self, values: dict[str, Any]) -> dict[str, Any]:
        # Temporarily use UI values for testing if provided
        test_key = values.get("cfg_api_key", self.config_manager.settings.opensubtitles_api_key)
        test_user = values.get("cfg_username", self.config_manager.settings.opensubtitles_username)
        test_pass = values.get("cfg_password", self.config_manager.settings.opensubtitles_password)

        from subtitles_plugin.config import OpenSubtitlesCredentials

        temp_creds = OpenSubtitlesCredentials(
            api_key=str(test_key).strip() or None,
            username=str(test_user).strip() or None,
            password=str(test_pass).strip() or None,
            source="settings_test",
        )
        temp_client = self.create_client(temp_creds)

        ok, msg = temp_client.test_connection()
        self.status_message = msg
        self.status_variant = "success" if ok else "danger"
        self.notify(msg, level="success" if ok else "error")

        return {"root": self.render_screen("settings", values)}
