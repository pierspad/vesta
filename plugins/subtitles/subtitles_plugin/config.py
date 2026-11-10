# SPDX-License-Identifier: GPL-3.0-or-later
"""Configuration and credential management for the Subtitles Vesta plugin.

Manages OpenSubtitles API credentials,
language preferences, and subtitle extraction settings with strict precedence:
1. Plugin settings screen UI (persisted in dataDir/settings.json)
2. Process environment variables (os.environ)
3. Repository root .env file (.env)

Secrets are masked and never logged.
"""

from __future__ import annotations

import json
import os
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any


def mask_secret(secret: str | None) -> str:
    """Safely mask sensitive API keys and passwords for display in logs or UI."""
    if not secret:
        return "(not configured)"
    secret = secret.strip()
    if len(secret) <= 8:
        return "********"
    return f"{secret[:4]}...{secret[-4:]}"


def parse_env_file(path: Path) -> dict[str, str]:
    """Parse a simple KEY=VALUE formatted .env file without external dependencies.

    Supports:
    - Comments starting with #
    - Empty lines
    - Leading/trailing whitespace stripping
    - Single and double quoted values
    - Export prefixes: 'export KEY=VAL'
    """
    if not path.is_file():
        return {}

    env_vars: dict[str, str] = {}
    try:
        content = path.read_text(encoding="utf-8")
    except Exception:
        return {}

    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[len("export ") :].strip()
        if "=" not in line:
            continue

        key, _, val = line.partition("=")
        key = key.strip()
        val = val.strip()

        # Strip surrounding quotes if matching
        if len(val) >= 2 and ((val[0] == '"' and val[-1] == '"') or (val[0] == "'" and val[-1] == "'")):
            val = val[1:-1]

        if key:
            env_vars[key] = val

    return env_vars


@dataclass
class OpenSubtitlesCredentials:
    """Resolved OpenSubtitles credentials and metadata."""

    api_key: str | None
    username: str | None
    password: str | None
    source: str  # 'settings', 'environment', 'dotenv', or 'none'

    @property
    def is_configured(self) -> bool:
        return bool(self.api_key and self.api_key.strip())

    @property
    def has_user_auth(self) -> bool:
        return bool(self.username and self.username.strip() and self.password and self.password.strip())

    @property
    def masked_key(self) -> str:
        return mask_secret(self.api_key)


@dataclass
class PluginSettings:
    """Persisted and runtime plugin settings."""

    opensubtitles_api_key: str = ""
    saved_api_keys: list[str] = field(default_factory=list)
    opensubtitles_username: str = ""
    opensubtitles_password: str = ""
    preferred_languages: str = ""
    default_download_dir: str = ""
    dry_run_default: bool = False
    ffmpeg_path: str = ""
    ffprobe_path: str = ""


class ConfigManager:
    """Manages plugin settings, environment overrides, and file-based persistence."""

    def __init__(
        self,
        repo_root: Path | None = None,
        data_dir: Path | None = None,
        settings_file_name: str = "settings.json",
    ):
        self.repo_root = repo_root or Path(__file__).resolve().parent.parent
        self.data_dir = data_dir
        self.settings_file_name = settings_file_name
        self.settings = PluginSettings()
        self.dotenv_vars: dict[str, str] = {}

        # Load .env file at repo root if exists
        self.load_dotenv()

        # Load persisted settings if data directory is known
        if self.data_dir:
            self.load_settings()

    def set_data_dir(self, data_dir: Path) -> None:
        """Sets the plugin data directory and loads persisted settings."""
        self.data_dir = data_dir
        self.load_settings()

    def load_dotenv(self) -> None:
        """Loads and parses the .env file in the repository root."""
        env_path = self.repo_root / ".env"
        self.dotenv_vars = parse_env_file(env_path)

    def load_settings(self) -> None:
        """Loads settings from data_dir/settings.json."""
        if not self.data_dir:
            return
        settings_path = self.data_dir / self.settings_file_name
        if not settings_path.is_file():
            return

        try:
            raw = json.loads(settings_path.read_text(encoding="utf-8"))
            if isinstance(raw, dict):
                self._apply_dict_to_settings(raw)
        except Exception:
            pass

    def save_settings(self) -> bool:
        """Persists current settings to data_dir/settings.json."""
        if not self.data_dir:
            return False
        try:
            self.data_dir.mkdir(parents=True, exist_ok=True)
            settings_path = self.data_dir / self.settings_file_name
            payload = asdict(self.settings)
            settings_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
            return True
        except Exception:
            return False

    def update_settings(self, updates: dict[str, Any], save: bool = True) -> None:
        """Updates in-memory settings and optionally saves to disk."""
        self._apply_dict_to_settings(updates)
        if save:
            self.save_settings()

    def _apply_dict_to_settings(self, data: dict[str, Any]) -> None:
        if "saved_api_keys" in data and isinstance(data["saved_api_keys"], list):
            self.settings.saved_api_keys = [
                str(k).strip() for k in data["saved_api_keys"] if str(k).strip()
            ]
        if "opensubtitles_api_key" in data:
            key_val = str(data["opensubtitles_api_key"]).strip()
            self.settings.opensubtitles_api_key = key_val
            if key_val and key_val not in self.settings.saved_api_keys:
                self.settings.saved_api_keys.append(key_val)
        if "opensubtitles_username" in data:
            self.settings.opensubtitles_username = str(data["opensubtitles_username"]).strip()
        if "opensubtitles_password" in data:
            self.settings.opensubtitles_password = str(data["opensubtitles_password"]).strip()
        if "preferred_languages" in data:
            self.settings.preferred_languages = str(data["preferred_languages"]).strip()
        if "default_download_dir" in data:
            self.settings.default_download_dir = str(data["default_download_dir"]).strip()
        if "dry_run_default" in data:
            val = data["dry_run_default"]
            if isinstance(val, str):
                self.settings.dry_run_default = val.lower() in ("true", "1", "yes")
            else:
                self.settings.dry_run_default = bool(val)
        if "ffmpeg_path" in data:
            self.settings.ffmpeg_path = str(data["ffmpeg_path"]).strip()
        if "ffprobe_path" in data:
            self.settings.ffprobe_path = str(data["ffprobe_path"]).strip()

    def resolve_field(self, setting_attr: str, env_key: str) -> tuple[str, str]:
        """Resolves a setting value following strict precedence:
        1. settings (UI/persisted)
        2. os.environ
        3. .env file
        """
        # 1. Check settings
        val = getattr(self.settings, setting_attr, "")
        if val:
            return str(val).strip(), "settings"

        # 2. Check process environment
        if env_key in os.environ and os.environ[env_key].strip():
            return os.environ[env_key].strip(), "environment"

        # 3. Check .env file
        if env_key in self.dotenv_vars and self.dotenv_vars[env_key].strip():
            return self.dotenv_vars[env_key].strip(), "dotenv"

        return "", "none"

    def resolve_credentials(self) -> OpenSubtitlesCredentials:
        """Resolves active OpenSubtitles credentials with strict precedence."""
        api_key, key_src = self.resolve_field("opensubtitles_api_key", "OPENSUBTITLES_API_KEY")
        username, user_src = self.resolve_field("opensubtitles_username", "OPENSUBTITLES_USERNAME")
        password, pass_src = self.resolve_field("opensubtitles_password", "OPENSUBTITLES_PASSWORD")

        source = key_src if key_src != "none" else (user_src if user_src != "none" else pass_src)
        return OpenSubtitlesCredentials(
            api_key=api_key or None,
            username=username or None,
            password=password or None,
            source=source,
        )

    def resolve_preferred_languages(self) -> list[str]:
        """Returns normalized list of preferred ISO language codes (e.g. ['en', 'ja'])."""
        lang_str, _ = self.resolve_field("preferred_languages", "OPENSUBTITLES_LANGUAGES")
        if not lang_str:
            lang_str = "en"
        codes = [c.strip().lower() for c in lang_str.replace(";", ",").split(",") if c.strip()]
        return codes or ["en"]

    def resolve_download_dir(self) -> Path:
        """Resolves destination download directory."""
        dir_str, _ = self.resolve_field("default_download_dir", "OPENSUBTITLES_DOWNLOAD_DIR")
        if dir_str:
            target = Path(dir_str).expanduser().resolve()
            target.mkdir(parents=True, exist_ok=True)
            return target

        if self.data_dir:
            target = self.data_dir / "downloads"
            target.mkdir(parents=True, exist_ok=True)
            return target

        fallback = self.repo_root / "downloads"
        fallback.mkdir(parents=True, exist_ok=True)
        return fallback

    def resolve_dry_run(self) -> bool:
        """Resolves dry-run execution preference."""
        if self.settings.dry_run_default:
            return True

        env_val = os.environ.get("OPENSUBTITLES_DRY_RUN")
        if env_val is not None:
            return env_val.strip().lower() in ("true", "1", "yes")

        dotenv_val = self.dotenv_vars.get("OPENSUBTITLES_DRY_RUN")
        if dotenv_val is not None:
            return dotenv_val.strip().lower() in ("true", "1", "yes")

        return False
