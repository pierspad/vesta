# SPDX-License-Identifier: GPL-3.0-or-later
"""OpenSubtitles.com REST API v1 Client.

Independent client interacting with OpenSubtitles.com REST API v1:
- Subtitle search by text query, TMDB ID, or IMDb ID with language filtering.
- Strict quality filtering: excludes machine-translated and AI-translated subtitles.
- Prioritizes verified, human-authored subtitles from trusted sources.
- Subtitle file download workflow via POST /api/v1/download.
- Optional user authentication for increased download quotas.
- Zero external runtime dependencies (urllib.request standard library).
"""

from __future__ import annotations

import json
import shutil
import tempfile
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from subtitles_plugin.config import OpenSubtitlesCredentials

BASE_URL = "https://api.opensubtitles.com/api/v1"
USER_AGENT = "subtitles-veplugin v1.0.0"


@dataclass
class SubtitleFile:
    """Individual subtitle file inside an OpenSubtitles item."""

    file_id: int
    cd_number: int = 1
    file_name: str = ""


@dataclass
class SubtitleItem:
    """Normalized subtitle metadata returned by OpenSubtitles search."""

    id: str
    subtitle_id: str
    language: str
    download_count: int
    new_download_count: int
    hearing_impaired: bool
    hd: bool
    fps: float | None
    ratings: float | None
    votes: int
    from_trusted: bool
    foreign_parts_only: bool
    ai_translated: bool
    machine_translated: bool
    release: str
    comments: str
    movie_name: str
    feature_id: int | None = None
    files: list[SubtitleFile] = field(default_factory=list)

    @property
    def primary_file_id(self) -> int | None:
        if self.files:
            return self.files[0].file_id
        return None

    @property
    def primary_file_name(self) -> str:
        if self.files and self.files[0].file_name:
            return self.files[0].file_name
        if self.release:
            return f"{self.release}.srt"
        return f"subtitle_{self.id}.srt"

    @property
    def is_human_made(self) -> bool:
        return not (self.machine_translated or self.ai_translated)

    @property
    def badge_text(self) -> str:
        badges = []
        if self.from_trusted:
            badges.append("TRUSTED")
        if self.hearing_impaired:
            badges.append("HI")
        if self.hd:
            badges.append("HD")
        return " | ".join(badges)


class OpenSubtitlesClient:
    """Client for OpenSubtitles.com REST API v1."""

    def __init__(
        self,
        credentials: OpenSubtitlesCredentials | None = None,
        base_url: str = BASE_URL,
        user_agent: str = USER_AGENT,
        timeout: int = 15,
    ):
        self.credentials = credentials or OpenSubtitlesCredentials(None, None, None, "none")
        self.base_url = base_url.rstrip("/")
        self.user_agent = user_agent
        self.timeout = timeout
        self.auth_token: str | None = None

    def _get_headers(self, requires_auth: bool = False) -> dict[str, str]:
        headers: dict[str, str] = {
            "User-Agent": self.user_agent,
            "Accept": "application/json",
            "Content-Type": "application/json",
        }
        if self.credentials.api_key:
            headers["Api-Key"] = self.credentials.api_key.strip()
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"
        return headers

    def _request(
        self,
        method: str,
        path: str,
        params: dict[str, Any] | None = None,
        body: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        url = f"{self.base_url}/{path.lstrip('/')}"
        if params:
            query_str = urllib.parse.urlencode({k: v for k, v in params.items() if v is not None and v != ""})
            if query_str:
                url = f"{url}?{query_str}"

        data_bytes = None
        if body is not None:
            data_bytes = json.dumps(body).encode("utf-8")

        req = urllib.request.Request(
            url=url,
            data=data_bytes,
            headers=self._get_headers(),
            method=method.upper(),
        )

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                resp_bytes = resp.read()
                if not resp_bytes:
                    return {}
                return json.loads(resp_bytes.decode("utf-8"))
        except urllib.error.HTTPError as e:
            raw_err = e.read().decode("utf-8", errors="replace")
            try:
                err_json = json.loads(raw_err)
                msg = err_json.get("message") or err_json.get("error") or raw_err
            except Exception:
                msg = raw_err or str(e)
            raise RuntimeError(f"OpenSubtitles API error (HTTP {e.code}): {msg}") from e
        except urllib.error.URLError as e:
            raise RuntimeError(f"OpenSubtitles connection failed: {e.reason}") from e

    def login(self, username: str | None = None, password: str | None = None) -> str:
        """Authenticates with OpenSubtitles to obtain a session bearer token."""
        user = username or (self.credentials.username if self.credentials else None)
        pwd = password or (self.credentials.password if self.credentials else None)

        if not user or not pwd:
            raise ValueError("Username and password are required for login.")

        payload = {"username": user, "password": pwd}
        res = self._request("POST", "/login", body=payload)
        token = res.get("token")
        if not token:
            raise RuntimeError("Login succeeded but no auth token returned.")
        self.auth_token = str(token)
        return self.auth_token

    def search(
        self,
        query: str = "",
        tmdb_id: int | str | None = None,
        imdb_id: int | str | None = None,
        languages: str | list[str] | None = None,
        hearing_impaired: str | None = None,  # 'include', 'exclude', 'only'
        exclude_machine_and_ai: bool = True,
        trusted_sources_only: bool = False,
    ) -> list[SubtitleItem]:
        """Search subtitles on OpenSubtitles.com.

        Excludes machine and AI translations and prioritizes human-made subtitles.
        """
        params: dict[str, Any] = {}

        if query and query.strip():
            params["query"] = query.strip()

        if tmdb_id:
            params["tmdb_id"] = str(tmdb_id).strip()

        if imdb_id:
            raw_imdb = str(imdb_id).strip()
            # If formatted like tt1375666, convert to numeric or keep as appropriate
            if raw_imdb.startswith("tt"):
                numeric_part = raw_imdb[2:]
                params["imdb_id"] = int(numeric_part) if numeric_part.isdigit() else raw_imdb
            else:
                params["imdb_id"] = int(raw_imdb) if raw_imdb.isdigit() else raw_imdb

        if languages:
            if isinstance(languages, list):
                params["languages"] = ",".join(languages)
            else:
                params["languages"] = str(languages).strip()

        if hearing_impaired:
            params["hearing_impaired"] = hearing_impaired

        if trusted_sources_only:
            params["trusted_sources"] = "only"

        # Ask API to exclude machine and AI translated subtitles
        if exclude_machine_and_ai:
            params["machine_translated"] = "exclude"
            params["ai_translated"] = "exclude"

        params["order_by"] = "download_count"
        params["order_direction"] = "desc"

        res = self._request("GET", "/subtitles", params=params)
        raw_items = res.get("data") or []

        parsed: list[SubtitleItem] = []
        for item in raw_items:
            attrs = item.get("attributes") or {}
            is_machine = bool(attrs.get("machine_translated", False))
            is_ai = bool(attrs.get("ai_translated", False))

            # Strictly enforce exclusion filter on client-side
            if exclude_machine_and_ai and (is_machine or is_ai):
                continue

            # Parse files
            files_list: list[SubtitleFile] = []
            for f in attrs.get("files") or []:
                fid = f.get("file_id")
                if fid is not None:
                    files_list.append(
                        SubtitleFile(
                            file_id=int(fid),
                            cd_number=int(f.get("cd_number", 1)),
                            file_name=str(f.get("file_name", "")),
                        )
                    )

            feat = attrs.get("feature_details") or {}
            movie_title = feat.get("movie_name") or feat.get("title") or ""

            subtitle = SubtitleItem(
                id=str(item.get("id", attrs.get("subtitle_id", ""))),
                subtitle_id=str(attrs.get("subtitle_id", item.get("id", ""))),
                language=str(attrs.get("language", "")).lower(),
                download_count=int(attrs.get("download_count", 0)),
                new_download_count=int(attrs.get("new_download_count", 0)),
                hearing_impaired=bool(attrs.get("hearing_impaired", False)),
                hd=bool(attrs.get("hd", False)),
                fps=float(attrs["fps"]) if attrs.get("fps") is not None else None,
                ratings=float(attrs["ratings"]) if attrs.get("ratings") is not None else None,
                votes=int(attrs.get("votes", 0)),
                from_trusted=bool(attrs.get("from_trusted", False)),
                foreign_parts_only=bool(attrs.get("foreign_parts_only", False)),
                ai_translated=is_ai,
                machine_translated=is_machine,
                release=str(attrs.get("release", "")).strip(),
                comments=str(attrs.get("comments", "")).strip(),
                movie_name=movie_title,
                feature_id=feat.get("feature_id"),
                files=files_list,
            )
            parsed.append(subtitle)

        # Prioritize human-authored / trusted subtitles, then ratings and downloads
        parsed.sort(
            key=lambda s: (
                1 if s.from_trusted else 0,
                1 if s.is_human_made else 0,
                s.ratings or 0.0,
                s.download_count,
            ),
            reverse=True,
        )

        return parsed

    def get_download_link(self, file_id: int) -> dict[str, Any]:
        """Requests a temporary subtitle download link for a file_id."""
        payload = {"file_id": file_id}
        return self._request("POST", "/download", body=payload)

    def download_subtitle(
        self,
        file_id: int,
        destination_dir: str | Path,
        custom_filename: str | None = None,
    ) -> Path:
        """Downloads a subtitle file given its file_id and writes it to destination_dir."""
        dest_dir = Path(destination_dir).resolve()
        dest_dir.mkdir(parents=True, exist_ok=True)

        info = self.get_download_link(file_id)
        download_url = info.get("link")
        if not download_url:
            raise RuntimeError("OpenSubtitles download response did not provide a download link.")

        file_name = custom_filename or info.get("file_name") or f"sub_{file_id}.srt"
        if not isinstance(file_name, str) or file_name in {".", ".."} or any(
            char in file_name for char in ("/", "\\", ":", "\x00")
        ):
            raise ValueError("Invalid subtitle filename.")
        if not file_name.lower().endswith((".srt", ".vtt")):
            file_name = f"{file_name}.srt"

        out_path = dest_dir / file_name

        req = urllib.request.Request(
            download_url,
            headers={"User-Agent": self.user_agent},
        )
        temporary = None
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                with tempfile.NamedTemporaryFile(dir=dest_dir, delete=False) as output:
                    temporary = Path(output.name)
                    shutil.copyfileobj(resp, output)
            temporary.replace(out_path)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
        return out_path

    def test_connection(self) -> tuple[bool, str]:
        """Tests the OpenSubtitles API key and connection."""
        if not self.credentials.api_key:
            return False, "OpenSubtitles API key is not configured."

        try:
            # If user auth is configured and not yet logged in, attempt login
            if self.credentials.has_user_auth and not self.auth_token:
                try:
                    self.login()
                except Exception as e:
                    return False, f"API key valid but user login failed: {e}"

            # Make a lightweight query to verify API key
            res = self._request("GET", "/subtitles", params={"query": "test", "languages": "en"})
            total = res.get("total_count", 0)
            user_suffix = " (authenticated user)" if self.auth_token else " (consumer key)"
            return True, f"Connected to OpenSubtitles successfully{user_suffix}. Found {total} test matches."
        except Exception as e:
            return False, f"Connection test failed: {e}"
