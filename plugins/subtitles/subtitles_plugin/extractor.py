# SPDX-License-Identifier: GPL-3.0-or-later
"""Local subtitle track extraction using ffprobe and ffmpeg.

Provides local inspection of video files to discover embedded subtitle tracks
and extracts selected tracks to standard SubRip (.srt) files.
Supports safe dry-run mode and graceful error handling when binaries are missing.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


class BinaryNotFoundError(RuntimeError):
    """Raised when ffprobe or ffmpeg executable is not found on PATH."""


@dataclass
class SubtitleTrackInfo:
    """Information about an embedded subtitle track inside a video container."""

    index: int  # Absolute stream index in container
    subtitle_index: int  # 0-based relative subtitle stream index
    codec_name: str
    language: str
    title: str

    @property
    def display_label(self) -> str:
        parts = [f"Track #{self.index}"]
        if self.language:
            parts.append(f"[{self.language}]")
        parts.append(f"({self.codec_name})")
        if self.title:
            parts.append(f"- {self.title}")
        return " ".join(parts)


@dataclass
class ExtractionResult:
    """Result of an extraction operation."""

    success: bool
    command: list[str]
    output_path: str
    dry_run: bool = False
    message: str = ""
    error: str | None = None


class LocalSubExtractor:
    """Discovers and extracts embedded subtitle tracks from media files."""

    def __init__(
        self,
        ffmpeg_bin: str = "ffmpeg",
        ffprobe_bin: str = "ffprobe",
        dry_run: bool = False,
    ):
        self.ffmpeg_bin = ffmpeg_bin
        self.ffprobe_bin = ffprobe_bin
        self.dry_run = dry_run

    def check_binaries(self) -> dict[str, bool]:
        """Checks if ffmpeg and ffprobe binaries are available."""
        return {
            "ffmpeg": bool(shutil.which(self.ffmpeg_bin) or Path(self.ffmpeg_bin).is_file()),
            "ffprobe": bool(shutil.which(self.ffprobe_bin) or Path(self.ffprobe_bin).is_file()),
        }

    def inspect_video_tracks(self, video_path: str | Path) -> list[SubtitleTrackInfo]:
        """Runs ffprobe on a video file to discover all embedded subtitle streams."""
        path = Path(video_path).resolve()
        if not path.is_file():
            raise FileNotFoundError(f"Video file not found: {path}")

        bin_status = self.check_binaries()
        if not bin_status.get("ffprobe"):
            raise BinaryNotFoundError(
                f"Executable '{self.ffprobe_bin}' not found on system PATH. Please install FFmpeg."
            )

        cmd = [
            self.ffprobe_bin,
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=index,codec_name:stream_tags=language,title",
            "-of",
            "json",
            str(path),
        ]

        try:
            res = subprocess.run(cmd, capture_output=True, text=True, check=True)
            data = json.loads(res.stdout) if res.stdout else {}
        except subprocess.CalledProcessError as e:
            err = e.stderr.strip() if e.stderr else str(e)
            raise RuntimeError(f"ffprobe failed to inspect file: {err}") from e
        except Exception as e:
            raise RuntimeError(f"Failed to parse ffprobe output: {e}") from e

        raw_streams = data.get("streams") or []
        tracks: list[SubtitleTrackInfo] = []

        for sub_idx, s in enumerate(raw_streams):
            stream_idx = int(s.get("index", sub_idx))
            codec = str(s.get("codec_name", "unknown"))
            tags = s.get("tags") or {}
            lang = str(tags.get("language") or "und").lower()
            title = str(tags.get("title") or "")

            tracks.append(
                SubtitleTrackInfo(
                    index=stream_idx,
                    subtitle_index=sub_idx,
                    codec_name=codec,
                    language=lang,
                    title=title,
                )
            )

        return tracks

    def build_extract_command(
        self,
        video_path: str | Path,
        stream_index: int,
        output_path: str | Path,
    ) -> list[str]:
        """Builds the ffmpeg command to extract a specific subtitle track to .srt."""
        return [
            self.ffmpeg_bin,
            "-y",
            "-i",
            str(Path(video_path).resolve()),
            "-map",
            f"0:{stream_index}",
            "-c:s",
            "srt",
            str(Path(output_path).resolve()),
        ]

    def extract_track(
        self,
        video_path: str | Path,
        stream_index: int,
        output_path: str | Path | None = None,
        output_dir: str | Path | None = None,
        dry_run: bool | None = None,
    ) -> ExtractionResult:
        """Extracts an embedded subtitle stream to a .srt file.

        If dry_run is True (or enabled by default), the ffmpeg command is constructed
        and returned without spawning the subprocess.
        """
        path = Path(video_path).resolve()
        if not path.is_file():
            return ExtractionResult(
                success=False,
                command=[],
                output_path="",
                error=f"Video file not found: {path}",
                message=f"Video file not found: {path}",
            )

        # Determine target output path
        if output_path:
            target_out = Path(output_path).resolve()
        else:
            base_dir = Path(output_dir).resolve() if output_dir else path.parent
            base_dir.mkdir(parents=True, exist_ok=True)
            target_out = base_dir / f"{path.stem}.track_{stream_index}.srt"

        cmd = self.build_extract_command(path, stream_index, target_out)
        is_dry_run = self.dry_run if dry_run is None else dry_run

        if is_dry_run:
            return ExtractionResult(
                success=True,
                command=cmd,
                output_path=str(target_out),
                dry_run=True,
                message=f"[Dry-run] Subtitle extraction command generated for track {stream_index}.",
            )

        bin_status = self.check_binaries()
        if not bin_status.get("ffmpeg"):
            return ExtractionResult(
                success=False,
                command=cmd,
                output_path=str(target_out),
                error=f"Executable '{self.ffmpeg_bin}' not found on system PATH. Please install FFmpeg.",
                message="FFmpeg executable missing on system.",
            )

        try:
            target_out.parent.mkdir(parents=True, exist_ok=True)
            res = subprocess.run(cmd, capture_output=True, text=True, check=False)
            if res.returncode == 0:
                return ExtractionResult(
                    success=True,
                    command=cmd,
                    output_path=str(target_out),
                    dry_run=False,
                    message=f"Subtitle track {stream_index} extracted successfully to {target_out.name}.",
                )
            else:
                err_text = res.stderr.strip() if res.stderr else f"ffmpeg exited with code {res.returncode}"
                return ExtractionResult(
                    success=False,
                    command=cmd,
                    output_path=str(target_out),
                    dry_run=False,
                    error=err_text,
                    message=f"Subtitle extraction failed: {err_text}",
                )
        except Exception as e:
            return ExtractionResult(
                success=False,
                command=cmd,
                output_path=str(target_out),
                dry_run=False,
                error=str(e),
                message=f"Subtitle extraction error: {e}",
            )
