#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Entry point for the Subtitles Vesta plugin.

First-party plugin communicating with the Vesta application via JSON-RPC 2.0 over stdin/stdout.
"""

import sys
from pathlib import Path

# Add repo root to sys.path
REPO_ROOT = Path(__file__).resolve().parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from subtitles_plugin.config import ConfigManager
from subtitles_plugin.plugin import SubtitlesPlugin
from subtitles_plugin.rpc import JsonRpcServer


def main() -> None:
    config_mgr = ConfigManager(repo_root=REPO_ROOT)
    plugin = SubtitlesPlugin(config_manager=config_mgr)
    server = JsonRpcServer(plugin=plugin)
    server.run()


if __name__ == "__main__":
    main()
