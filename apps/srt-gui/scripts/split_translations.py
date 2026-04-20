#!/usr/bin/env python3
"""
split_translations.py
─────────────────────
Splits Vesta i18n JSON files into smaller chunks suitable for LLM review.

Produces a structured output under:
  <output_root>/
    i18n/
      <lang>/
        chunk_001.json
        chunk_002.json
        ...
    info/
      <lang>/
        chunk_001.json
        ...

Usage:
  python3 split_translations.py [--max-keys N] [--output-dir DIR]

Defaults:
  --max-keys   50
  --output-dir ./translation_chunks
"""

import argparse
import json
import math
from pathlib import Path


# ── Paths ─────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent
SRC_ROOT   = SCRIPT_DIR.parent / "src" / "lib" / "i18n" / "locales"
I18N_DIR   = SRC_ROOT          # main locale files: ar.json, de.json, …
INFO_DIR   = SRC_ROOT / "info" # info/help locale files

LANGUAGES = ["ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "pl", "pt", "ru", "tr", "zh"]


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def save_json(data: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


def split_dict(d: dict, chunk_size: int) -> list[dict]:
    """Split a flat dict into a list of dicts, each with at most chunk_size keys."""
    items = list(d.items())
    chunks = []
    for i in range(0, len(items), chunk_size):
        chunks.append(dict(items[i : i + chunk_size]))
    return chunks


def split_locale_set(src_dir: Path, output_root: Path, label: str, max_keys: int) -> None:
    """Split all language files in src_dir into chunks under output_root/label/<lang>/."""
    print(f"\n{'='*60}")
    print(f"  Splitting: {label}  (source: {src_dir})")
    print(f"{'='*60}")

    for lang in LANGUAGES:
        src_file = src_dir / f"{lang}.json"
        if not src_file.exists():
            print(f"  [{lang}] ⚠  not found, skipping")
            continue

        data = load_json(src_file)
        chunks = split_dict(data, max_keys)
        n_chunks = len(chunks)
        n_pad = max(3, len(str(n_chunks)))

        lang_out = output_root / label / lang
        lang_out.mkdir(parents=True, exist_ok=True)

        for idx, chunk in enumerate(chunks, start=1):
            chunk_name = f"chunk_{idx:0{n_pad}d}.json"
            chunk_path = lang_out / chunk_name
            save_json(chunk, chunk_path)

        total_keys = len(data)
        print(
            f"  [{lang}] ✓  {total_keys} keys → {n_chunks} chunk(s) of ≤{max_keys}  "
            f"→  {lang_out.relative_to(output_root.parent)}"
        )


def main() -> None:
    parser = argparse.ArgumentParser(description="Split Vesta i18n JSONs into LLM-review chunks")
    parser.add_argument(
        "--max-keys", type=int, default=50, metavar="N",
        help="Maximum number of keys per chunk file (default: 50)"
    )
    parser.add_argument(
        "--output-dir", type=str, default=str(SCRIPT_DIR / "translation_chunks"), metavar="DIR",
        help="Root directory for output chunks (default: scripts/translation_chunks)"
    )
    args = parser.parse_args()

    output_root = Path(args.output_dir).resolve()
    max_keys    = args.max_keys

    print(f"\nVesta i18n Splitter")
    print(f"  Max keys per chunk : {max_keys}")
    print(f"  Output root        : {output_root}")

    split_locale_set(I18N_DIR, output_root, "i18n", max_keys)
    split_locale_set(INFO_DIR, output_root, "info", max_keys)

    print(f"\n✅  Done. Chunks saved to: {output_root}")
    print(
        f"\nNext steps:\n"
        f"  1. Open a chunk file (e.g. translation_chunks/i18n/it/chunk_001.json)\n"
        f"  2. Ask an LLM to review/correct the translations against the EN reference\n"
        f"  3. Save the corrected file in place\n"
        f"  4. Run merge_translations.py to reassemble the originals\n"
    )


if __name__ == "__main__":
    main()
