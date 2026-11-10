# SPDX-License-Identifier: GPL-3.0-or-later
"""Tests for Subtitles plugin controller, declarative UI, patches, and JSON-RPC 2.0 stdio server."""

import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest.mock import patch

from subtitles_plugin.config import ConfigManager, OpenSubtitlesCredentials
from subtitles_plugin.extractor import ExtractionResult, LocalSubExtractor, SubtitleTrackInfo
from subtitles_plugin.opensubtitles import OpenSubtitlesClient, SubtitleFile, SubtitleItem
from subtitles_plugin.plugin import SubtitlesPlugin
from subtitles_plugin.rpc import JsonRpcServer
from subtitles_plugin.schema import (
    apply_patches,
    column,
    external_link,
    find_node,
    heading,
    language_select,
    make_patch,
    password_input,
    path_input,
    row,
    segmented,
    text,
    text_input,
)


class FakeOpenSubtitlesClient(OpenSubtitlesClient):
    """Mock client for offline testing."""

    def __init__(self):
        super().__init__(OpenSubtitlesCredentials("fake_key", None, None, "test"))
        self.items = [
            SubtitleItem(
                id="sub_test_1",
                subtitle_id="101",
                language="en",
                download_count=4200,
                new_download_count=10,
                hearing_impaired=False,
                hd=True,
                fps=23.976,
                ratings=8.8,
                votes=50,
                from_trusted=True,
                foreign_parts_only=False,
                ai_translated=False,
                machine_translated=False,
                release="The.Matrix.1999.1080p.BluRay",
                comments="Human verified",
                movie_name="The Matrix",
                files=[
                    SubtitleFile(file_id=8801, cd_number=1, file_name="The.Matrix.1999.srt")
                ],
            )
        ]

    def search(self, **kwargs) -> list[SubtitleItem]:
        query = kwargs.get("query", "").lower()
        if "matrix" in query or kwargs.get("tmdb_id") or kwargs.get("imdb_id"):
            return self.items
        return []

    def download_subtitle(self, file_id: int, destination_dir: str | Path, custom_filename: str | None = None) -> Path:
        target = Path(destination_dir) / (custom_filename or "The.Matrix.1999.srt")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("1\n00:00:01,000 --> 00:00:05,000\nWelcome to the real world.\n", encoding="utf-8")
        return target

    def test_connection(self) -> tuple[bool, str]:
        return True, "Mock connected to OpenSubtitles successfully"


class FakeLocalSubExtractor(LocalSubExtractor):
    """Mock extractor for offline testing."""

    def __init__(self):
        super().__init__(dry_run=True)

    def check_binaries(self) -> dict[str, bool]:
        return {"ffmpeg": True, "ffprobe": True}

    def inspect_video_tracks(self, video_path: str | Path) -> list[SubtitleTrackInfo]:
        return [
            SubtitleTrackInfo(
                index=2,
                subtitle_index=0,
                codec_name="subrip",
                language="eng",
                title="English Dialogue",
            ),
            SubtitleTrackInfo(
                index=3,
                subtitle_index=1,
                codec_name="ass",
                language="jpn",
                title="Japanese Commentary",
            ),
        ]

    def extract_track(self, video_path: str | Path, stream_index: int, **kwargs) -> ExtractionResult:
        out_path = Path(kwargs.get("output_dir", "/tmp")) / f"video.track_{stream_index}.srt"
        return ExtractionResult(
            success=True,
            command=["ffmpeg", "-y", "-i", str(video_path), "-map", f"0:{stream_index}", "-c:s", "srt", str(out_path)],
            output_path=str(out_path),
            dry_run=True,
            message=f"Subtitle track {stream_index} extracted successfully to {out_path.name}.",
        )


def test_plugin_full_flow(tmp_path: Path):
    data_dir = tmp_path / "data"
    mgr = ConfigManager(repo_root=tmp_path, data_dir=data_dir)
    client = FakeOpenSubtitlesClient()
    extractor = FakeLocalSubExtractor()

    notifications: list[tuple[str, dict[str, Any]]] = []

    def record_notify(method: str, params: dict[str, Any]):
        notifications.append((method, params))

    plugin = SubtitlesPlugin(
        config_manager=mgr,
        opensubtitles_client=client,
        extractor=extractor,
        notifier=record_notify,
    )

    # 1. Initialize
    init_res = plugin.initialize(
        api_version=1,
        data_dir=str(data_dir),
        locale="en",
        settings={"preferred_languages": "en,fr"},
    )
    assert init_res["status"] == "ok"
    assert init_res["ready"] is True
    assert init_res["root"]["type"] == "column"
    assert mgr.settings.preferred_languages == "en,fr"

    # 2. Search for Matrix
    search_res = plugin.handle_event(
        screen="search",
        widget_id="btn_do_search",
        event_type="click",
        values={"search_query": "Matrix", "search_languages": "en"},
    )
    assert "root" in search_res
    assert len(plugin.search_results) == 1
    assert plugin.search_results[0].release == "The.Matrix.1999.1080p.BluRay"

    # 3. Select subtitle item
    select_res = plugin.handle_event(
        screen="search",
        widget_id="action_select_sub_test_1",
        event_type="click",
        values={},
    )
    assert plugin.selected_subtitle is not None
    assert plugin.selected_subtitle.id == "sub_test_1"

    # 4. Download subtitle
    dl_res = plugin.handle_event(
        screen="search",
        widget_id="btn_download_now",
        event_type="click",
        values={
            "download_dest_dir": str(tmp_path / "subs"),
            "download_custom_name": "Matrix.srt",
        },
    )
    assert plugin.downloaded_subtitle_path is not None
    assert "Matrix.srt" in plugin.downloaded_subtitle_path

    # 5. Emit Flashcards handoff
    fc_res = plugin.handle_event(
        screen="search",
        widget_id="btn_use_in_flashcards",
        event_type="click",
        values={"video_for_flashcards": "/media/matrix.mkv"},
    )
    assert "patch" in fc_res
    # Verify notification was sent
    flashcard_notifs = [n for n in notifications if n[0] == "vesta.openInFlashcards"]
    assert len(flashcard_notifs) == 1
    assert flashcard_notifs[0][1]["videoPath"] == "/media/matrix.mkv"
    assert "Matrix.srt" in flashcard_notifs[0][1]["subtitlePath"]


def test_plugin_local_extraction_flow(tmp_path: Path):
    data_dir = tmp_path / "data"
    mgr = ConfigManager(repo_root=tmp_path, data_dir=data_dir)
    client = FakeOpenSubtitlesClient()
    extractor = FakeLocalSubExtractor()

    notifications: list[tuple[str, dict[str, Any]]] = []

    def record_notify(method: str, params: dict[str, Any]):
        notifications.append((method, params))

    plugin = SubtitlesPlugin(
        config_manager=mgr,
        opensubtitles_client=client,
        extractor=extractor,
        notifier=record_notify,
    )

    # 1. Switch to local screen
    nav_res = plugin.handle_event(
        screen="search",
        widget_id="btn_nav_local",
        event_type="click",
        values={},
    )
    assert plugin.current_screen == "local"

    # 2. Inspect embedded tracks
    inspect_res = plugin.handle_event(
        screen="local",
        widget_id="btn_inspect_tracks",
        event_type="click",
        values={"local_video_path": "/media/anime.mkv"},
    )
    assert len(plugin.local_tracks) == 2
    assert plugin.local_tracks[0].codec_name == "subrip"
    assert plugin.local_tracks[1].codec_name == "ass"

    # 3. Extract track #2
    extract_res = plugin.handle_event(
        screen="local",
        widget_id="action_extract_2",
        event_type="click",
        values={},
    )
    assert plugin.extracted_subtitle_path is not None
    assert "track_2.srt" in plugin.extracted_subtitle_path

    # 4. Use in flashcards from local screen
    fc_res = plugin.handle_event(
        screen="local",
        widget_id="btn_use_in_flashcards_local",
        event_type="click",
        values={"local_video_path": "/media/anime.mkv"},
    )
    assert "patch" in fc_res
    flashcard_notifs = [n for n in notifications if n[0] == "vesta.openInFlashcards"]
    assert len(flashcard_notifs) == 1
    assert flashcard_notifs[0][1]["videoPath"] == "/media/anime.mkv"
    assert "track_2.srt" in flashcard_notifs[0][1]["subtitlePath"]


def test_plugin_settings_save_and_test(tmp_path: Path):
    data_dir = tmp_path / "data"
    mgr = ConfigManager(repo_root=tmp_path, data_dir=data_dir)
    client = FakeOpenSubtitlesClient()
    plugin = SubtitlesPlugin(config_manager=mgr, opensubtitles_client=client)

    rejected = plugin.handle_event("settings", "btn_save_settings", "click", values={"cfg_api_key": "  "})
    assert rejected["snackbar"]["type"] == "warning"
    assert not mgr.settings.opensubtitles_api_key

    # Test settings save
    save_res = plugin.handle_event(
        screen="settings",
        widget_id="btn_save_settings",
        event_type="click",
        values={
            "cfg_api_key": "new_saved_key",
            "cfg_username": "new_user",
            "cfg_preferred_languages": "ja,en",
            "cfg_dry_run_default": True,
        },
    )
    assert mgr.settings.opensubtitles_api_key == "new_saved_key"
    assert mgr.settings.opensubtitles_username == "new_user"
    assert mgr.settings.preferred_languages == "ja,en"
    assert mgr.settings.dry_run_default is True

    # Test connection test
    test_res = plugin.handle_event(
        screen="settings",
        widget_id="btn_test_credentials",
        event_type="click",
        values={"cfg_api_key": "test_key"},
    )
    assert "Mock connected" in plugin.status_message


def test_json_rpc_server_loop(tmp_path: Path):
    in_stream = io.StringIO(
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"apiVersion": 1}}) + "\n"
        + json.dumps({"jsonrpc": "2.0", "id": 2, "method": "render", "params": {"screen": "search"}}) + "\n"
        + json.dumps({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": {}}) + "\n"
    )
    out_stream = io.StringIO()

    mgr = ConfigManager(repo_root=tmp_path)
    client = FakeOpenSubtitlesClient()
    plugin = SubtitlesPlugin(config_manager=mgr, opensubtitles_client=client)

    server = JsonRpcServer(plugin=plugin, input_stream=in_stream, output_stream=out_stream)
    server.run()

    lines = [json.loads(line) for line in out_stream.getvalue().strip().splitlines() if line]
    assert len(lines) == 3

    # Resp 1: initialize
    assert lines[0]["id"] == 1
    assert lines[0]["result"]["status"] == "ok"
    assert lines[0]["result"]["ready"] is True

    # Resp 2: render
    assert lines[1]["id"] == 2
    assert "root" in lines[1]["result"]

    # Resp 3: shutdown
    assert lines[2]["id"] == 3
    assert lines[2]["result"]["status"] == "ok"


def test_schema_single_object_patch():
    initial_tree = column(
        [
            heading("Title", level=1, id="title_node"),
            text("Initial text", id="desc_node", variant="normal"),
            row(
                [
                    text_input("inp_1", label="First"),
                    text_input("inp_2", label="Second"),
                ],
                id="row_1",
            ),
        ],
        id="main_root",
    )

    patch = make_patch(
        set_props={
            "desc_node": {"text": "Updated text", "variant": "success"},
            "inp_1": {"disabled": True},
        },
        visibility={
            "title_node": False,
        },
    )

    updated_tree = apply_patches(initial_tree, patch)

    # Check updated props
    desc, _, _ = find_node(updated_tree, "desc_node")
    assert desc is not None
    assert desc["text"] == "Updated text"
    assert desc["variant"] == "success"

    # Check visibility
    title, _, _ = find_node(updated_tree, "title_node")
    assert title is not None
    assert title["hidden"] is True

    # Check input disabled prop
    inp1, _, _ = find_node(updated_tree, "inp_1")
    assert inp1 is not None
    assert inp1["disabled"] is True


def test_ui_layout_and_expert_widgets(tmp_path: Path):
    mgr = ConfigManager(repo_root=tmp_path)
    client = FakeOpenSubtitlesClient()
    plugin = SubtitlesPlugin(config_manager=mgr, opensubtitles_client=client)

    # 1. Search screen layout
    search_root = plugin.render_screen("search")
    q_node, _, _ = find_node(search_root, "search_query")
    assert q_node is not None
    assert q_node["flex"] == 3
    assert q_node["icon"] == "search"
    assert q_node["label"] == "Title"

    lang_node, _, _ = find_node(search_root, "search_languages")
    assert lang_node is not None
    assert lang_node["type"] == "languageSelect"
    assert lang_node["flex"] == 1

    tmdb_node, _, _ = find_node(search_root, "search_tmdb_id")
    assert tmdb_node is not None
    assert tmdb_node["icon"] == "tmdb"
    assert tmdb_node["flex"] == 1

    imdb_node, _, _ = find_node(search_root, "search_imdb_id")
    assert imdb_node is not None
    assert imdb_node["icon"] == "imdb"
    assert imdb_node["flex"] == 1

    # Navigation bar buttons have monochrome icons (Local Video File is inside Settings)
    nav_search, _, _ = find_node(search_root, "btn_nav_search")
    assert nav_search is None

    nav_local, _, _ = find_node(search_root, "btn_nav_local")
    assert nav_local is None

    nav_settings, _, _ = find_node(search_root, "btn_nav_settings")
    assert nav_settings is not None
    assert nav_settings["icon"] == "settings"
    assert nav_settings["placement"] == "header"
    assert "⚙" not in nav_settings["label"]

    # 2. Local video extraction is embedded in Settings with expertOnly
    settings_root = plugin.render_screen("settings")
    local_vid_node, _, _ = find_node(settings_root, "local_video_path")
    assert local_vid_node is not None
    assert local_vid_node.get("expertOnly") is True
    api_key_node, _, _ = find_node(settings_root, "cfg_api_key")
    assert api_key_node is not None
    assert api_key_node["link"]["url"] == "https://www.opensubtitles.com/en/consumers"

    # Preferred language is languageSelect
    pref_node, _, _ = find_node(settings_root, "cfg_preferred_languages")
    assert pref_node is not None
    assert pref_node["type"] == "languageSelect"

    # Download dir is pathInput
    dl_node, _, _ = find_node(settings_root, "cfg_default_download_dir")
    assert dl_node is not None
    assert dl_node["type"] == "pathInput"
    assert dl_node["pickerType"] == "folder"

    # Expert-only fields
    for expert_id in ("cfg_username", "cfg_password", "cfg_dry_run_default", "cfg_ffmpeg_path", "cfg_ffprobe_path"):
        f_node, _, _ = find_node(settings_root, expert_id)
        assert f_node is not None, f"Node {expert_id} missing"
        assert f_node.get("expertOnly") is True, f"Node {expert_id} not expertOnly"



def test_search_guidance_and_local_key(tmp_path: Path):
    clean_env = {k: v for k, v in os.environ.items() if k != "OPENSUBTITLES_API_KEY"}
    with patch.dict(os.environ, clean_env, clear=True):
        mgr = ConfigManager(repo_root=tmp_path)
        plugin = SubtitlesPlugin(config_manager=mgr)
        plugin.initialize(data_dir=str(tmp_path / "data"), locale="it-IT")
        root = plugin.render_screen("search")
        search, _, _ = find_node(root, "btn_do_search")
        settings, _, _ = find_node(root, "btn_nav_settings")
        alternatives, _, _ = find_node(root, "search_id_alternatives")
        assert search["disabled"] is True
        assert search["disabledFocus"] == "btn_nav_settings"
        assert search["requiresAny"] == ["search_query", "search_tmdb_id", "search_imdb_id"]
        assert settings["status"] == "attention"
        assert alternatives["separator"] == "or"
        assert find_node(root, "api_warning_box")[0] is None

        settings_root = plugin.render_screen("settings")
        guidance, _, _ = find_node(settings_root, "opensubtitles_setup_guidance")
        field, _, _ = find_node(settings_root, "cfg_api_key")
        assert guidance is None
        assert field["emptyWarning"]

        mgr.update_settings({"opensubtitles_api_key": "saved-key"})
        restored = SubtitlesPlugin(config_manager=ConfigManager(repo_root=tmp_path))
        restored.initialize(data_dir=str(tmp_path / "data"))
        ready = restored.render_screen("search")
        assert find_node(ready, "btn_do_search")[0]["disabled"] is False
        assert find_node(ready, "btn_nav_settings")[0].get("status") is None

        assert find_node(restored.render_screen("settings"), "opensubtitles_setup_guidance")[0] is None


def test_remove_active_key_clears_form_and_disk(tmp_path: Path):
    clean_env = {k: v for k, v in os.environ.items() if k != "OPENSUBTITLES_API_KEY"}
    with patch.dict(os.environ, clean_env, clear=True):
        data = tmp_path / "data"
        manager = ConfigManager(repo_root=tmp_path, data_dir=data)
        plugin = SubtitlesPlugin(config_manager=manager)
        manager.update_settings({"opensubtitles_api_key": "old-key"})

        result = plugin.handle_event("settings", "action_delete_key_0", "click", {"cfg_api_key": "old-key"})
        assert result["patch"]["values"] == {"cfg_api_key": ""}
        assert find_node(result["root"], "saved_keys_list")[0] is None

        reloaded = ConfigManager(repo_root=tmp_path, data_dir=data)
        assert not reloaded.resolve_credentials().api_key
        assert reloaded.settings.saved_api_keys == []

        manager.settings.opensubtitles_api_key = "other-key"
        plugin.handle_event("settings", "btn_test_credentials", "click", {"cfg_api_key": ""})
        assert plugin.status_variant == "danger"
        assert "not configured" in plugin.status_message


class TestRpcAndPluginSuite(unittest.TestCase):
    def test_all(self):
        test_schema_single_object_patch()
        for fn in (
            test_plugin_full_flow,
            test_plugin_local_extraction_flow,
            test_plugin_settings_save_and_test,
            test_json_rpc_server_loop,
            test_ui_layout_and_expert_widgets,
            test_search_guidance_and_local_key,
            test_remove_active_key_clears_form_and_disk,
        ):
            with tempfile.TemporaryDirectory() as td:
                fn(Path(td))

