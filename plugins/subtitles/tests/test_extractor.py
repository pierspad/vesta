# SPDX-License-Identifier: GPL-3.0-or-later
"""Tests for local video subtitle track inspection and extraction via ffprobe/ffmpeg."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

from subtitles_plugin.extractor import (
    BinaryNotFoundError,
    ExtractionResult,
    LocalSubExtractor,
    SubtitleTrackInfo,
)


SAMPLE_FFPROBE_OUTPUT = {
    "streams": [
        {
            "index": 2,
            "codec_name": "subrip",
            "tags": {
                "language": "eng",
                "title": "Full English Dialogue",
            },
        },
        {
            "index": 3,
            "codec_name": "hdmv_pgs_subtitle",
            "tags": {
                "language": "spa",
                "title": "Spanish SDH",
            },
        },
        {
            "index": 5,
            "codec_name": "ass",
            "tags": {
                "language": "jpn",
            },
        },
    ]
}


def test_check_binaries_missing():
    extractor = LocalSubExtractor(ffmpeg_bin="nonexistent_ffmpeg_binary_xyz", ffprobe_bin="nonexistent_ffprobe_xyz")
    status = extractor.check_binaries()
    assert status["ffmpeg"] is False
    assert status["ffprobe"] is False


def test_inspect_video_tracks_missing_ffprobe(tmp_path: Path):
    dummy_video = tmp_path / "movie.mkv"
    dummy_video.write_bytes(b"dummy")

    extractor = LocalSubExtractor(ffprobe_bin="missing_probe_binary")
    try:
        extractor.inspect_video_tracks(dummy_video)
    except BinaryNotFoundError:
        pass
    else:
        raise AssertionError("Expected BinaryNotFoundError")


def test_inspect_video_tracks_success(tmp_path: Path):
    dummy_video = tmp_path / "movie.mkv"
    dummy_video.write_bytes(b"dummy")

    extractor = LocalSubExtractor(ffprobe_bin="ffprobe")

    mock_proc = MagicMock()
    mock_proc.stdout = json.dumps(SAMPLE_FFPROBE_OUTPUT)
    mock_proc.returncode = 0

    with patch("shutil.which", return_value="/usr/bin/ffprobe"), patch("subprocess.run", return_value=mock_proc):
        tracks = extractor.inspect_video_tracks(dummy_video)

        assert len(tracks) == 3

        assert tracks[0].index == 2
        assert tracks[0].subtitle_index == 0
        assert tracks[0].codec_name == "subrip"
        assert tracks[0].language == "eng"
        assert tracks[0].title == "Full English Dialogue"
        assert "[eng]" in tracks[0].display_label

        assert tracks[1].index == 3
        assert tracks[1].subtitle_index == 1
        assert tracks[1].codec_name == "hdmv_pgs_subtitle"
        assert tracks[1].language == "spa"

        assert tracks[2].index == 5
        assert tracks[2].subtitle_index == 2
        assert tracks[2].language == "jpn"
        assert tracks[2].title == ""


def test_build_extract_command(tmp_path: Path):
    extractor = LocalSubExtractor(ffmpeg_bin="ffmpeg")
    video = tmp_path / "movie.mkv"
    out_sub = tmp_path / "movie.track_2.srt"

    cmd = extractor.build_extract_command(video, stream_index=2, output_path=out_sub)
    assert cmd == [
        "ffmpeg",
        "-y",
        "-i",
        str(video.resolve()),
        "-map",
        "0:2",
        "-c:s",
        "srt",
        str(out_sub.resolve()),
    ]


def test_extract_track_dry_run(tmp_path: Path):
    video = tmp_path / "movie.mkv"
    video.write_bytes(b"dummy")

    extractor = LocalSubExtractor(ffmpeg_bin="ffmpeg", dry_run=True)
    res = extractor.extract_track(video, stream_index=2)

    assert res.success is True
    assert res.dry_run is True
    assert "[Dry-run]" in res.message
    assert "movie.track_2.srt" in res.output_path
    assert "-map" in res.command
    assert "0:2" in res.command


def test_extract_track_missing_ffmpeg(tmp_path: Path):
    video = tmp_path / "movie.mkv"
    video.write_bytes(b"dummy")

    extractor = LocalSubExtractor(ffmpeg_bin="missing_ffmpeg", dry_run=False)
    with patch("shutil.which", return_value=None):
        res = extractor.extract_track(video, stream_index=2)
        assert res.success is False
        assert "not found on system PATH" in res.error


def test_extract_track_execution(tmp_path: Path):
    video = tmp_path / "movie.mkv"
    video.write_bytes(b"dummy")

    extractor = LocalSubExtractor(ffmpeg_bin="ffmpeg", dry_run=False)

    mock_proc = MagicMock()
    mock_proc.returncode = 0
    mock_proc.stdout = ""
    mock_proc.stderr = ""

    with patch("shutil.which", return_value="/usr/bin/ffmpeg"), patch("subprocess.run", return_value=mock_proc):
        out_target = tmp_path / "custom_out.srt"
        res = extractor.extract_track(video, stream_index=2, output_path=out_target)

        assert res.success is True
        assert res.dry_run is False
        assert res.output_path == str(out_target.resolve())
        assert "extracted successfully" in res.message


class TestExtractorSuite(unittest.TestCase):
    def test_all(self):
        test_check_binaries_missing()
        for fn in (
            test_inspect_video_tracks_missing_ffprobe,
            test_inspect_video_tracks_success,
            test_build_extract_command,
            test_extract_track_dry_run,
            test_extract_track_missing_ffmpeg,
            test_extract_track_execution,
        ):
            with tempfile.TemporaryDirectory() as td:
                fn(Path(td))

