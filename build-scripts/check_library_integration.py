#!/usr/bin/env python3
"""Compile a real downstream Cargo consumer of all Vesta library crates."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "build-scripts" / "library-smoke"
LIBRARIES = [
    "srt-apkg", "srt-download", "srt-parser", "srt-ankiconnect",
    "srt-autosync", "srt-extract", "srt-flashcards", "srt-refine",
    "srt-sync", "srt-transcribe", "srt-translate",
]
GPU_FEATURES = {"cuda", "rocm", "sycl", "vulkan"}


def run(command: list[str], *, env: dict[str, str], cwd: Path, show_output: bool = True) -> str:
    if show_output:
        print("+", " ".join(command), flush=True)
    result = subprocess.run(command, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if show_output:
        print(result.stdout, end="", flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
    return result.stdout


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true", help="Pass Cargo's --offline flag (requires cached dependencies).")
    parser.add_argument("--individual", action="store_true", help="Also resolve and cargo-check every crate separately.")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="vesta-library-consumer-") as temp:
        project = Path(temp) / "consumer"
        shutil.copytree(FIXTURE, project)
        manifest = project / "Cargo.toml"
        manifest.write_text(render_manifest(manifest.read_text(), ROOT))

        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target" / "integration")
        offline = ["--offline"] if args.offline else []
        if args.individual:
            for crate in LIBRARIES:
                single = Path(temp) / f"single-{crate}"
                single.mkdir()
                single_manifest = single / "Cargo.toml"
                relative = f"core/{crate}" if crate in {"srt-apkg", "srt-download", "srt-parser"} else f"lib/{crate}"
                single_manifest.write_text(
                    "[package]\nname = \"vesta-single-smoke\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
                    f"[dependencies]\n{crate} = {{ path = {json.dumps(str(ROOT / relative))} }}\n"
                    "[patch.crates-io]\n"
                    f"whisper-rs-sys = {{ path = {json.dumps(str(ROOT / 'vendor/whisper-rs-sys-0.15.0'))} }}\n"
                )
                (single / "src").mkdir()
                (single / "src" / "lib.rs").write_text("")
                run(["cargo", "check", "--manifest-path", str(single_manifest), *offline], env=env, cwd=single)
            print(f"Individually checked all {len(LIBRARIES)} Vesta crates.", flush=True)

        metadata = json.loads(run(["cargo", "metadata", "--quiet", "--format-version", "1", "--manifest-path", str(manifest), *offline], env=env, cwd=project, show_output=False))
        packages = {package["name"]: package for package in metadata["packages"]}
        missing = sorted(set(LIBRARIES) - packages.keys())
        if missing:
            raise SystemExit(f"Downstream dependency graph is missing libraries: {missing}")
        forbidden = [name for name in packages if any(term in name.lower() for term in ("tauri", "webkit", "gtk"))]
        if forbidden:
            raise SystemExit(f"Desktop dependencies leaked into the library graph: {forbidden}")
        for package_name in ("srt-transcribe", "srt-autosync", "whisper-rs", "whisper-rs-sys"):
            node = next((node for node in metadata["resolve"]["nodes"] if packages_by_id_name(metadata, node["id"]) == package_name), None)
            if node is None:
                raise SystemExit(f"Could not inspect resolved {package_name} features")
            gpu_features = GPU_FEATURES.intersection(node["features"])
            if gpu_features:
                raise SystemExit(f"Smoke fixture unexpectedly enables GPU features for {package_name}: {sorted(gpu_features)}")
        print(f"Resolved {len(LIBRARIES)} Vesta library crates with no desktop or Whisper GPU features.", flush=True)

        run(["cargo", "run", "--manifest-path", str(manifest), *offline], env=env, cwd=project)


def packages_by_id_name(metadata: dict, package_id: str) -> str | None:
    return next((package["name"] for package in metadata["packages"] if package["id"] == package_id), None)


def render_manifest(content: str, root: Path) -> str:
    import re

    replacements = {
        "../../core/srt-parser": root / "core/srt-parser",
        "../../core/srt-apkg": root / "core/srt-apkg",
        "../../core/srt-download": root / "core/srt-download",
        "../../lib/srt-ankiconnect": root / "lib/srt-ankiconnect",
        "../../lib/srt-autosync": root / "lib/srt-autosync",
        "../../lib/srt-extract": root / "lib/srt-extract",
        "../../lib/srt-flashcards": root / "lib/srt-flashcards",
        "../../lib/srt-refine": root / "lib/srt-refine",
        "../../lib/srt-sync": root / "lib/srt-sync",
        "../../lib/srt-transcribe": root / "lib/srt-transcribe",
        "../../lib/srt-translate": root / "lib/srt-translate",
        "../../vendor/whisper-rs-sys-0.15.0": root / "vendor/whisper-rs-sys-0.15.0",
    }
    for old, path in replacements.items():
        content = re.sub(re.escape(f'path = "{old}"'), f"path = {json.dumps(str(path))}", content)
    return content


if __name__ == "__main__":
    main()
