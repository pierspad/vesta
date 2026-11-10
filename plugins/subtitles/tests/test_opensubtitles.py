# SPDX-License-Identifier: GPL-3.0-or-later
"""Tests for OpenSubtitles API client, search filtering, and download handling."""

import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from subtitles_plugin.config import OpenSubtitlesCredentials
from subtitles_plugin.opensubtitles import OpenSubtitlesClient, SubtitleItem


SAMPLE_SEARCH_RESPONSE = {
    "total_pages": 1,
    "total_count": 4,
    "page": 1,
    "data": [
        {
            "id": "sub_1",
            "type": "subtitle",
            "attributes": {
                "subtitle_id": "1001",
                "language": "en",
                "download_count": 1500,
                "new_download_count": 20,
                "hearing_impaired": False,
                "hd": True,
                "fps": 23.976,
                "ratings": 8.5,
                "votes": 45,
                "from_trusted": True,
                "foreign_parts_only": False,
                "ai_translated": False,
                "machine_translated": False,
                "release": "Inception.2010.1080p.BluRay.x264-SPARKS",
                "comments": "Synced by human translator",
                "feature_details": {
                    "feature_id": 55,
                    "title": "Inception",
                    "movie_name": "Inception (2010)",
                    "tmdb_id": 27205,
                    "imdb_id": 1375666,
                },
                "files": [
                    {"file_id": 9901, "cd_number": 1, "file_name": "Inception.2010.SPARKS.srt"}
                ],
            },
        },
        {
            "id": "sub_2",
            "type": "subtitle",
            "attributes": {
                "subtitle_id": "1002",
                "language": "en",
                "download_count": 8000,
                "new_download_count": 50,
                "hearing_impaired": True,
                "hd": True,
                "fps": 23.976,
                "ratings": 9.1,
                "votes": 120,
                "from_trusted": False,
                "foreign_parts_only": False,
                "ai_translated": False,
                "machine_translated": False,
                "release": "Inception.2010.720p.BrRip",
                "comments": "Human SDH version",
                "feature_details": {
                    "feature_id": 55,
                    "title": "Inception",
                    "movie_name": "Inception (2010)",
                    "tmdb_id": 27205,
                    "imdb_id": 1375666,
                },
                "files": [
                    {"file_id": 9902, "cd_number": 1, "file_name": "Inception.SDH.srt"}
                ],
            },
        },
        {
            "id": "sub_ai",
            "type": "subtitle",
            "attributes": {
                "subtitle_id": "1003",
                "language": "en",
                "download_count": 200,
                "hearing_impaired": False,
                "from_trusted": False,
                "ai_translated": True,
                "machine_translated": False,
                "release": "Inception.AI.Sub",
                "files": [{"file_id": 9903, "file_name": "Inception.AI.srt"}],
            },
        },
        {
            "id": "sub_machine",
            "type": "subtitle",
            "attributes": {
                "subtitle_id": "1004",
                "language": "en",
                "download_count": 100,
                "hearing_impaired": False,
                "from_trusted": False,
                "ai_translated": False,
                "machine_translated": True,
                "release": "Inception.Machine.Sub",
                "files": [{"file_id": 9904, "file_name": "Inception.Machine.srt"}],
            },
        },
    ],
}


def test_search_subtitles_filters_machine_and_ai():
    creds = OpenSubtitlesCredentials(api_key="test_api_key", username=None, password=None, source="test")
    client = OpenSubtitlesClient(credentials=creds)

    mock_resp = io.BytesIO(json.dumps(SAMPLE_SEARCH_RESPONSE).encode("utf-8"))

    with patch("urllib.request.urlopen", return_value=mock_resp) as mock_urlopen:
        results = client.search(query="Inception", languages="en", exclude_machine_and_ai=True)

        assert len(results) == 2
        assert all(not (r.ai_translated or r.machine_translated) for r in results)
        assert all(r.is_human_made for r in results)

        assert results[0].id == "sub_1"
        assert results[0].from_trusted is True
        assert results[0].primary_file_id == 9901
        assert "TRUSTED" in results[0].badge_text

        assert results[1].id == "sub_2"
        assert results[1].hearing_impaired is True
        assert "HI" in results[1].badge_text

        req_arg = mock_urlopen.call_args[0][0]
        url = req_arg.full_url
        assert "query=Inception" in url
        assert "languages=en" in url
        assert "machine_translated=exclude" in url
        assert "ai_translated=exclude" in url
        assert req_arg.get_header("Api-key") == "test_api_key"


def test_search_subtitles_by_tmdb_and_imdb():
    creds = OpenSubtitlesCredentials(api_key="test_key", username=None, password=None, source="test")
    client = OpenSubtitlesClient(credentials=creds)

    mock_resp = io.BytesIO(json.dumps({"data": []}).encode("utf-8"))
    with patch("urllib.request.urlopen", return_value=mock_resp) as mock_urlopen:
        client.search(tmdb_id=27205, imdb_id="tt1375666", languages=["en", "ja"])

        req_arg = mock_urlopen.call_args[0][0]
        url = req_arg.full_url
        assert "tmdb_id=27205" in url
        assert "imdb_id=1375666" in url
        assert "languages=en%2Cja" in url


def test_download_subtitle(tmp_path: Path):
    creds = OpenSubtitlesCredentials(api_key="test_key", username=None, password=None, source="test")
    client = OpenSubtitlesClient(credentials=creds)

    srt_content = b"1\n00:00:01,000 --> 00:00:04,000\nHello, world!\n"

    def fake_urlopen(req, timeout=15):
        method = req.get_method() if hasattr(req, "get_method") else "GET"
        if method == "POST":
            return io.BytesIO(
                json.dumps({
                    "link": "https://files.opensubtitles.org/sub/test.srt",
                    "file_name": "Inception.2010.BluRay.srt",
                    "requests": 1,
                    "remaining": 19,
                }).encode("utf-8")
            )
        return io.BytesIO(srt_content)

    with patch("urllib.request.urlopen", side_effect=fake_urlopen):
        saved_file = client.download_subtitle(
            file_id=9901,
            destination_dir=tmp_path,
        )

        assert saved_file.is_file()
        assert saved_file.name == "Inception.2010.BluRay.srt"
        assert saved_file.read_bytes() == srt_content


def test_user_login():
    creds = OpenSubtitlesCredentials(
        api_key="test_key",
        username="myuser",
        password="mypassword",
        source="test",
    )
    client = OpenSubtitlesClient(credentials=creds)

    mock_login_resp = io.BytesIO(
        json.dumps({
            "token": "mock_bearer_token_xyz",
            "status": 200,
            "user": {"user_id": 1234},
        }).encode("utf-8")
    )

    with patch("urllib.request.urlopen", return_value=mock_login_resp):
        token = client.login()
        assert token == "mock_bearer_token_xyz"
        assert client.auth_token == "mock_bearer_token_xyz"
        headers = client._get_headers()
        assert headers["Authorization"] == "Bearer mock_bearer_token_xyz"


def test_test_connection_no_key():
    client = OpenSubtitlesClient(credentials=OpenSubtitlesCredentials(None, None, None, "none"))
    ok, msg = client.test_connection()
    assert ok is False
    assert "not configured" in msg


def test_test_connection_success():
    creds = OpenSubtitlesCredentials(api_key="valid_key", username=None, password=None, source="test")
    client = OpenSubtitlesClient(credentials=creds)

    mock_resp = io.BytesIO(json.dumps({"total_count": 42}).encode("utf-8"))
    with patch("urllib.request.urlopen", return_value=mock_resp):
        ok, msg = client.test_connection()
        assert ok is True
        assert "Connected to OpenSubtitles successfully" in msg
        assert "42 test matches" in msg


def test_download_rejects_remote_path_and_preserves_existing_file_on_error(tmp_path: Path):
    client = OpenSubtitlesClient()
    link = "https://files.example/sub.srt"
    dest = tmp_path / "downloads"
    with patch.object(client, "get_download_link", return_value={"link": link, "file_name": "../outside.srt"}):
        with patch("urllib.request.urlopen") as download:
            try:
                client.download_subtitle(1, dest)
            except ValueError as exc:
                assert "Invalid subtitle filename" in str(exc)
            else:
                raise AssertionError("Expected ValueError")
            download.assert_not_called()
    assert not (tmp_path / "outside.srt").exists()

    existing = dest / "movie.srt"
    existing.write_bytes(b"previous subtitle")

    class BrokenDownload(io.BytesIO):
        def read(self, size=-1):
            if self.tell():
                raise OSError("interrupted")
            return super().read(size)

    with patch.object(client, "get_download_link", return_value={"link": link, "file_name": "movie.srt"}):
        with patch("urllib.request.urlopen", return_value=BrokenDownload(b"partial")):
            try:
                client.download_subtitle(1, dest)
            except OSError as exc:
                assert "interrupted" in str(exc)
            else:
                raise AssertionError("Expected OSError")

    assert existing.read_bytes() == b"previous subtitle"
    assert list(dest.iterdir()) == [existing]


class TestOpenSubtitlesSuite(unittest.TestCase):
    def test_all(self):
        test_search_subtitles_filters_machine_and_ai()
        test_search_subtitles_by_tmdb_and_imdb()
        test_user_login()
        test_test_connection_no_key()
        test_test_connection_success()
        with tempfile.TemporaryDirectory() as td:
            test_download_subtitle(Path(td))
        with tempfile.TemporaryDirectory() as td:
            test_download_rejects_remote_path_and_preserves_existing_file_on_error(Path(td))

