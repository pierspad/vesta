#!/usr/bin/env python3
"""
merge_translations.py
─────────────────────
Reassembles Vesta i18n JSON files from LLM-reviewed chunks and overwrites
the original locale files.

Reads from:
  <chunks_root>/
    i18n/<lang>/chunk_001.json  chunk_002.json  …
    info/<lang>/chunk_001.json  …

Writes back to:
  src/lib/i18n/locales/<lang>.json
  src/lib/i18n/locales/info/<lang>.json

Usage:
  python3 merge_translations.py [--chunks-dir DIR] [--dry-run]

Options:
  --chunks-dir DIR   Root of the chunk directory (default: scripts/translation_chunks)
  --dry-run          Show what would be written without touching the originals
  --lang LANG        Only merge a single language (e.g. --lang it)
  --type {i18n,info} Only merge one type (default: both)
"""

import argparse
import json
import sys
from pathlib import Path


# ── Paths ─────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent
SRC_ROOT   = SCRIPT_DIR.parent / "src" / "lib" / "i18n" / "locales"
I18N_DST   = SRC_ROOT          # <lang>.json files live here
INFO_DST   = SRC_ROOT / "info" # info/<lang>.json files

LANGUAGES = ["ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "pl", "pt", "ru", "tr", "zh"]


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def save_json(data: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


def natural_sort_key(path: Path) -> tuple:
    """Sort chunk_001, chunk_002 … numerically rather than lexicographically."""
    import re
    parts = re.split(r"(\d+)", path.stem)
    return tuple(int(p) if p.isdigit() else p for p in parts)


def merge_chunks(chunk_dir: Path) -> dict:
    """Merge all chunk_NNN.json files in chunk_dir into a single ordered dict."""
    chunk_files = sorted(chunk_dir.glob("chunk_*.json"), key=natural_sort_key)
    if not chunk_files:
        return {}
    merged: dict = {}
    for cf in chunk_files:
        data = load_json(cf)
        overlap = set(merged) & set(data)
        if overlap:
            print(f"    ⚠  Duplicate keys in {cf.name}: {', '.join(sorted(overlap))}")
        merged.update(data)
    return merged


def merge_locale_set(
    chunks_root: Path,
    label: str,
    dst_dir: Path,
    dry_run: bool,
    only_lang: str | None,
) -> None:
    """Merge chunks for one locale set (i18n or info) and write to dst_dir."""

    src_label_dir = chunks_root / label
    if not src_label_dir.exists():
        print(f"  ⚠  No chunks directory found for '{label}' at {src_label_dir}")
        return

    langs_to_process = [only_lang] if only_lang else LANGUAGES

    print(f"\n{'='*60}")
    print(f"  Merging: {label}  →  {dst_dir.relative_to(SCRIPT_DIR.parent)}")
    print(f"{'='*60}")

    for lang in langs_to_process:
        chunk_dir = src_label_dir / lang
        if not chunk_dir.exists():
            print(f"  [{lang}] ⚠  no chunk directory found, skipping")
            continue

        merged = merge_chunks(chunk_dir)
        if not merged:
            print(f"  [{lang}] ⚠  no chunks found in {chunk_dir}, skipping")
            continue

        dst_file = dst_dir / f"{lang}.json"
        n_keys = len(merged)

        if dry_run:
            print(
                f"  [{lang}] DRY-RUN  would write {n_keys} keys → {dst_file.relative_to(SCRIPT_DIR.parent)}"
            )
        else:
            save_json(merged, dst_file)
            print(
                f"  [{lang}] ✓  {n_keys} keys → {dst_file.relative_to(SCRIPT_DIR.parent)}"
            )


def validate_against_en(chunks_root: Path, label: str) -> None:
    """Cross-check every language against the EN reference, report missing/extra keys."""

    en_dir = chunks_root / label / "en"
    if not en_dir.exists():
        print(f"\n  ⚠  No EN chunks for '{label}', skipping validation")
        return

    en_data = merge_chunks(en_dir)
    en_keys = set(en_data.keys())

    print(f"\n── Validation: {label} (reference: EN, {len(en_keys)} keys) ──")

    for lang in LANGUAGES:
        if lang == "en":
            continue
        lang_dir = chunks_root / label / lang
        if not lang_dir.exists():
            continue
        lang_data = merge_chunks(lang_dir)
        lang_keys = set(lang_data.keys())

        missing = sorted(en_keys - lang_keys)
        extra   = sorted(lang_keys - en_keys)

        if not missing and not extra:
            print(f"  [{lang}] ✓  all {len(lang_keys)} keys present")
        else:
            if missing:
                print(f"  [{lang}] ✗  MISSING {len(missing)} key(s): {', '.join(missing[:10])}"
                      + (" …" if len(missing) > 10 else ""))
            if extra:
                print(f"  [{lang}] ⚠  EXTRA   {len(extra)} key(s): {', '.join(extra[:10])}"
                      + (" …" if len(extra) > 10 else ""))


def main() -> None:
    parser = argparse.ArgumentParser(description="Merge Vesta i18n chunks back into locale files")
    parser.add_argument(
        "--chunks-dir", type=str,
        default=str(SCRIPT_DIR / "translation_chunks"),
        metavar="DIR",
        help="Root of the chunk directory (default: scripts/translation_chunks)",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Print what would be written without modifying any files",
    )
    parser.add_argument(
        "--lang", type=str, default=None, metavar="LANG",
        help="Only merge a specific language code (e.g. it, fr, de)",
    )
    parser.add_argument(
        "--type", choices=["i18n", "info", "both"], default="both",
        help="Which locale set to merge (default: both)",
    )
    parser.add_argument(
        "--validate", action="store_true",
        help="After merging, validate all languages against the EN reference",
    )
    args = parser.parse_args()

    chunks_root = Path(args.chunks_dir).resolve()
    if not chunks_root.exists():
        print(f"❌  Chunks directory not found: {chunks_root}")
        print("    Run split_translations.py first.")
        sys.exit(1)

    print(f"\nVesta i18n Merger")
    print(f"  Chunks root : {chunks_root}")
    print(f"  Dry-run     : {args.dry_run}")
    if args.lang:
        print(f"  Language    : {args.lang} only")

    do_i18n = args.type in ("i18n", "both")
    do_info = args.type in ("info", "both")

    if do_i18n:
        merge_locale_set(chunks_root, "i18n", I18N_DST, args.dry_run, args.lang)
    if do_info:
        merge_locale_set(chunks_root, "info", INFO_DST, args.dry_run, args.lang)

    if args.validate:
        if do_i18n:
            validate_against_en(chunks_root, "i18n")
        if do_info:
            validate_against_en(chunks_root, "info")

    if args.dry_run:
        print("\n(Dry-run complete — no files were modified)")
    else:
        print("\n✅  Done. Original locale files have been overwritten.")
        print(
            "\nNote: the chunk files in translation_chunks/ are still present.\n"
            "You can delete them once you have verified the merged result."
        )


if __name__ == "__main__":
    main()
