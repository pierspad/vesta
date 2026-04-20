#!/usr/bin/env python3
"""Audit locale JSON files against English keys.

Supports multiple locale directories (e.g. locales/ and locales/info/).

A key is marked as missing for a locale when:
- the key does not exist in that locale file
- the translation is empty/blank
- the translation text is exactly the same as English

Additionally reports:
- Orphan keys: keys present in a locale but absent from en.json
- Per-directory breakdown
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


LOCALE_FILE_RE = re.compile(r"^[a-z]{2}\.json$", re.IGNORECASE)


@dataclass
class MissingItem:
    locale: str
    reason: str
    directory: str = ""


@dataclass
class OrphanItem:
    locale: str
    key: str
    directory: str = ""


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError(f"{path} does not contain a JSON object at top-level")
    return data


def normalize_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    return str(value).strip()


def discover_locale_dirs(base_dir: Path) -> list[Path]:
    """Return a list of directories containing locale JSON files."""
    dirs = [base_dir]
    for child in sorted(base_dir.iterdir()):
        if child.is_dir() and (child / "en.json").exists():
            dirs.append(child)
    return dirs


def audit_single_dir(
    locales_dir: Path,
) -> tuple[
    dict[str, list[MissingItem]],
    dict[str, int],
    list[str],
    list[OrphanItem],
]:
    en_path = locales_dir / "en.json"
    if not en_path.exists():
        raise FileNotFoundError(f"English locale not found: {en_path}")

    en_data = load_json(en_path)
    locale_paths = sorted(
        p
        for p in locales_dir.glob("*.json")
        if p.name != "en.json" and LOCALE_FILE_RE.match(p.name)
    )
    locale_codes = [p.stem for p in locale_paths]

    all_missing: dict[str, list[MissingItem]] = {}
    per_locale_count = {code: 0 for code in locale_codes}
    orphans: list[OrphanItem] = []

    loaded_locales = {p.stem: load_json(p) for p in locale_paths}
    dir_label = locales_dir.name

    # Check for missing / empty / same-as-english
    for key, en_value in en_data.items():
        en_text = normalize_text(en_value)
        for code, data in loaded_locales.items():
            reason: str | None = None
            if key not in data:
                reason = "missing_key"
            else:
                value_text = normalize_text(data.get(key))
                if value_text == "":
                    reason = "empty_value"
                elif value_text == en_text:
                    reason = "same_as_english"

            if reason is not None:
                all_missing.setdefault(key, []).append(
                    MissingItem(locale=code, reason=reason, directory=dir_label)
                )
                per_locale_count[code] += 1

    # Check for orphan keys (in locale but not in en.json)
    en_keys = set(en_data.keys())
    for code, data in loaded_locales.items():
        for key in data:
            if key not in en_keys:
                orphans.append(OrphanItem(locale=code, key=key, directory=dir_label))

    return all_missing, per_locale_count, locale_codes, orphans


def audit_all_dirs(
    locale_dirs: list[Path],
) -> tuple[
    dict[str, list[MissingItem]],
    dict[str, int],
    list[str],
    list[OrphanItem],
    dict[str, int],
]:
    combined_missing: dict[str, list[MissingItem]] = {}
    combined_counts: dict[str, int] = {}
    all_codes: list[str] = []
    combined_orphans: list[OrphanItem] = []
    en_keys_per_dir: dict[str, int] = {}

    for d in locale_dirs:
        missing, counts, codes, orphans = audit_single_dir(d)
        en_keys_per_dir[d.name] = len(load_json(d / "en.json"))

        for key, items in missing.items():
            combined_missing.setdefault(key, []).extend(items)

        for code, count in counts.items():
            combined_counts[code] = combined_counts.get(code, 0) + count
            if code not in all_codes:
                all_codes.append(code)

        combined_orphans.extend(orphans)

    all_codes.sort()
    return combined_missing, combined_counts, all_codes, combined_orphans, en_keys_per_dir


def build_report(locale_dirs: list[Path]) -> dict[str, Any]:
    (
        missing_by_key,
        per_locale_count,
        locale_codes,
        orphans,
        en_keys_per_dir,
    ) = audit_all_dirs(locale_dirs)

    missing_keys = {}
    for key, issues in sorted(missing_by_key.items()):
        missing_keys[key] = {
            "locales": list(dict.fromkeys(item.locale for item in issues)),
            "details": [
                {"locale": item.locale, "reason": item.reason, "directory": item.directory}
                for item in issues
            ],
        }

    orphan_entries = [
        {"locale": o.locale, "key": o.key, "directory": o.directory}
        for o in orphans
    ]

    total_en = sum(en_keys_per_dir.values())

    return {
        "directories": [str(d) for d in locale_dirs],
        "en_keys_per_directory": en_keys_per_dir,
        "total_english_keys": total_en,
        "checked_locales": locale_codes,
        "missing_counts_by_locale": per_locale_count,
        "total_keys_with_issues": len(missing_keys),
        "orphan_keys_count": len(orphan_entries),
        "orphan_keys": orphan_entries,
        "missing_keys": missing_keys,
    }


def count_issues_by_reason(report: dict[str, Any], reasons: set[str]) -> tuple[int, dict[str, int]]:
    per_locale: dict[str, int] = {}
    total = 0

    for issue in report.get("missing_keys", {}).values():
        for detail in issue.get("details", []):
            reason = detail.get("reason")
            locale = detail.get("locale")
            if reason in reasons and isinstance(locale, str):
                per_locale[locale] = per_locale.get(locale, 0) + 1
                total += 1

    return total, dict(sorted(per_locale.items(), key=lambda item: item[0]))


def count_by_reason_and_locale(report: dict[str, Any]) -> dict[str, dict[str, int]]:
    """Return {locale: {reason: count}} for a nice table."""
    table: dict[str, dict[str, int]] = {}
    for issue in report.get("missing_keys", {}).values():
        for detail in issue.get("details", []):
            reason = detail.get("reason", "?")
            locale = detail.get("locale", "?")
            table.setdefault(locale, {})
            table[locale][reason] = table[locale].get(reason, 0) + 1
    return dict(sorted(table.items()))


def main() -> int:
    script_root = Path(__file__).resolve().parents[1]
    default_locales = script_root / "src" / "lib" / "i18n" / "locales"
    default_report = script_root / "reports" / "missing_translations_report.json"

    parser = argparse.ArgumentParser(
        description="Find missing or untranslated locale keys compared to en.json"
    )
    parser.add_argument(
        "--locales-dir",
        default=str(default_locales),
        help="Base directory containing locale JSON files (sub-dirs like info/ are auto-discovered)",
    )
    parser.add_argument(
        "--output",
        default=str(default_report),
        help="Output JSON report path",
    )
    parser.add_argument(
        "--fail-on-issues",
        action="store_true",
        help="Exit with code 1 if issues exist for the configured block reasons",
    )
    parser.add_argument(
        "--block-reasons",
        default="missing_key,empty_value",
        help="Comma-separated reasons that should block when --fail-on-issues is set",
    )
    args = parser.parse_args()

    locales_dir = Path(args.locales_dir)
    if not locales_dir.exists():
        raise FileNotFoundError(f"Locales directory does not exist: {locales_dir}")

    locale_dirs = discover_locale_dirs(locales_dir)
    report = build_report(locale_dirs)

    block_reasons = {
        token.strip()
        for token in str(args.block_reasons).split(",")
        if token.strip()
    }

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)

    # ─── Summary ───────────────────────────────────────────
    print(f"Report written to: {output_path}")
    print()

    for d in locale_dirs:
        label = d.name
        count = report["en_keys_per_directory"].get(label, "?")
        print(f"  📁 {d}  ({count} English keys)")
    print()

    total_en = report["total_english_keys"]
    print(f"Total English keys: {total_en}")
    print(f"Keys with issues:   {report['total_keys_with_issues']}")
    print()

    # Per-locale breakdown table
    table = count_by_reason_and_locale(report)
    if table:
        reasons_set = set()
        for counts in table.values():
            reasons_set.update(counts.keys())
        reasons_list = sorted(reasons_set)

        header = f"{'Locale':<8}"
        for r in reasons_list:
            header += f"  {r:<16}"
        header += f"  {'Total':<8}"
        print(header)
        print("─" * len(header))

        for locale, counts in table.items():
            row = f"{locale:<8}"
            row_total = 0
            for r in reasons_list:
                c = counts.get(r, 0)
                row_total += c
                row += f"  {c:<16}"
            row += f"  {row_total:<8}"
            print(row)
    else:
        print("✅ No issues found across any locale!")

    # Orphan keys
    orphans = report.get("orphan_keys", [])
    if orphans:
        print()
        print(f"⚠  Orphan keys (in locale but missing from en.json): {len(orphans)}")
        # Group by key
        by_key: dict[str, list[str]] = {}
        for o in orphans:
            by_key.setdefault(o["key"], []).append(f'{o["locale"]} ({o["directory"]})')
        for key, locales in sorted(by_key.items()):
            print(f"  • {key}  →  {', '.join(locales)}")

    # Fail-on-issues
    if args.fail_on_issues:
        blocking_total, blocking_by_locale = count_issues_by_reason(report, block_reasons)
        if blocking_total > 0:
            print()
            print(
                "❌ Blocking i18n issues found "
                f"for reasons: {', '.join(sorted(block_reasons))}"
            )
            for locale, count in blocking_by_locale.items():
                print(f"  - {locale}: {count}")
            return 1
        print()
        print(
            "✅ No blocking i18n issues found "
            f"for reasons: {', '.join(sorted(block_reasons))}"
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
