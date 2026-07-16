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
PLACEHOLDER_RE = re.compile(r"\{\{[^}]+\}\}|\{[A-Za-z0-9_.-]+\}")
EXPECTED_LOCALES = [
    "ar",
    "de",
    "en",
    "es",
    "fr",
    "hi",
    "it",
    "ja",
    "ko",
    "nl",
    "pl",
    "pt",
    "ru",
    "tr",
    "zh",
]
SAME_AS_ENGLISH_ALLOWED_KEYS = {
    "app.title",
    "common.no",
    "flashcards.audioField",
    "flashcards.notesField",
    "flashcards.cpuEco",
    "flashcards.previewStatus",
    "flashcards.videoField",
    "keys.alt",
    "keys.arrowdown",
    "keys.arrowleft",
    "keys.arrowright",
    "keys.arrowup",
    "keys.ctrl",
    "keys.enter",
    "keys.escape",
    "keys.shift",
    "keys.space",
    "keys.tab",
    "provider.openrouter",
    "settings.endpointStatus.offline",
    "settings.endpointStatus.online",
    "settings.modal.providerGoogleDesc",
    "settings.providerStatus.offline",
    "settings.section.overview",
    "settings.section.whisper",
    "shortcuts.action.offsetUp",
    "sync.audioFormats",
    "sync.offset",
    "sync.offsetAdjust",
    "sync.statusTitle",
    "sync.video",
    "transcribe.modelMedium",
    "transcribe.segmentMedium",
    "translate.batchTurbo",
    "translate.context",
    "translate.model",
    "translate.subPerBatch",
    # ── Internationally shared terms ────────────────────────────────────────────
    # These English words are identical (or the accepted standard) in the target
    # language, so flagging them as "same as English" is a false positive.
    "flashcards.filterMinChars",      # "Minimum" — universal in DE/FR/NL/PL/TR
    "flashcards.filterMaxChars",      # "Maximum" — universal in DE/FR/NL
    "flashcards.filterMinDuration",   # "Minimum" — universal in DE/FR/NL/PL/TR
    "flashcards.filterMaxDuration",   # "Maximum" — universal in DE/FR/NL
    "refine.mode.manual",             # "Manual" — used unchanged in ES/PT
    "refine.total",                   # "Total: {{count}}" — identical in ES/PT
    "refine.notesLabel",              # "Notes" — identical in FR
    "refine.action.fileLabel",        # "File:" — identical in IT
    "refine.llm.status.offline",      # "server offline" — identical in IT/NL
    "align.page",                     # "Page" — identical in FR
    "translate.overlapMinimal",       # "Minimal" — identical in DE/FR/TR
    "transcribe.error",               # "Error" — identical in ES
    "translate.contextTitle",         # "Context" — identical in NL
    "provider.anthropic",
    "provider.github",
    "provider.mistral",
    "provider.nvidia",
    "settings.modal.provider",
    "tiers.budget",
    "tiers.fallback",
    "tiers.rpm",
    "tiers.unlimited",
    "transcribe.log",
    "translate.batch",
    "translate.provider",
    "settings.section.llm",
    "provider.mistral.desc",
    "settings.modal.documentation",
    "tiers.auto",
    "tiers.model",
    "transcribe.cloudModel",
    "experimental.anki.title",
    "experimental.condense.modeVad",
    "flashcards.videoEncoderX264",
    "experimental.badge",
    "nav.experimental",
    "experimental.condense.resultSegments",
    "flashcards.exportAnkiConnectBadge",
}


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


def placeholders(value: Any) -> set[str]:
    return set(PLACEHOLDER_RE.findall(normalize_text(value)))


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
    dict[str, Any],           # en_data
    dict[str, dict[str, Any]], # loaded_locales
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
    expected_non_en = [code for code in EXPECTED_LOCALES if code != "en"]
    per_locale_count = {code: 0 for code in expected_non_en}
    orphans: list[OrphanItem] = []

    loaded_locales = {p.stem: load_json(p) for p in locale_paths}
    dir_label = locales_dir.name

    for code in expected_non_en:
        if code in loaded_locales:
            continue

        for key in en_data:
            all_missing.setdefault(key, []).append(
                MissingItem(locale=code, reason="missing_locale", directory=dir_label)
            )
            per_locale_count[code] += 1

    # Check for missing / empty / same-as-english
    for key, en_value in en_data.items():
        en_text = normalize_text(en_value)
        for code, data in loaded_locales.items():
            if code not in expected_non_en:
                continue
            reason: str | None = None
            if key not in data:
                reason = "missing_key"
            else:
                value_text = normalize_text(data.get(key))
                if value_text == "":
                    reason = "empty_value"
                elif placeholders(value_text) != placeholders(en_text):
                    reason = "placeholder_mismatch"
                elif value_text == en_text and key not in SAME_AS_ENGLISH_ALLOWED_KEYS:
                    reason = "same_as_english"

            if reason is not None:
                all_missing.setdefault(key, []).append(
                    MissingItem(locale=code, reason=reason, directory=dir_label)
                )
                per_locale_count[code] += 1

    # Check for orphan keys (in locale but not in en.json)
    en_keys = set(en_data.keys())
    for code, data in loaded_locales.items():
        if code not in expected_non_en:
            continue
        for key in data:
            if key not in en_keys:
                orphans.append(OrphanItem(locale=code, key=key, directory=dir_label))

    return all_missing, per_locale_count, locale_codes, orphans, en_data, loaded_locales


def audit_all_dirs(
    locale_dirs: list[Path],
) -> tuple[
    dict[str, list[MissingItem]],
    dict[str, int],
    list[str],
    list[OrphanItem],
    dict[str, int],
    dict[str, Any],            # merged en_data (last dir wins on collision)
    dict[str, dict[str, Any]], # merged loaded_locales
]:
    combined_missing: dict[str, list[MissingItem]] = {}
    combined_counts: dict[str, int] = {}
    all_codes: list[str] = []
    combined_orphans: list[OrphanItem] = []
    en_keys_per_dir: dict[str, int] = {}
    merged_en: dict[str, Any] = {}
    merged_locales: dict[str, dict[str, Any]] = {}

    for d in locale_dirs:
        missing, counts, codes, orphans, en_data, loaded_locales = audit_single_dir(d)
        en_keys_per_dir[d.name] = len(en_data)

        merged_en.update(en_data)
        for code, data in loaded_locales.items():
            merged_locales.setdefault(code, {}).update(data)

        for key, items in missing.items():
            combined_missing.setdefault(key, []).extend(items)

        for code, count in counts.items():
            combined_counts[code] = combined_counts.get(code, 0) + count
            if code not in all_codes:
                all_codes.append(code)

        combined_orphans.extend(orphans)

    all_codes.sort()
    return combined_missing, combined_counts, all_codes, combined_orphans, en_keys_per_dir, merged_en, merged_locales


def build_translation_tasks(
    missing_by_key: dict[str, list[MissingItem]],
    en_data: dict[str, Any],
    loaded_locales: dict[str, dict[str, Any]],
) -> dict[str, list[dict[str, Any]]]:
    """Build a per-locale map of translation tasks, each with full context.

    Structure emitted::

        {
          "de": [
            {
              "key": "nav.refine",
              "reason": "same_as_english",
              "en": "Refine",
              "current_value": "Refine"   # null when key is missing entirely
            },
            ...
          ],
          ...
        }

    Designed to be fed directly to an LLM or MCP tool:
    - One entry = one translation to fix.
    - `en` = authoritative English string to translate from.
    - `current_value` = what is wrong in the file right now (null if key absent).
    - `reason` = the exact issue category for filtering/routing.
    """
    # Group issues by locale
    by_locale: dict[str, list[dict[str, Any]]] = {}
    for key, issues in sorted(missing_by_key.items()):
        en_value = en_data.get(key, "")
        for item in issues:
            locale_data = loaded_locales.get(item.locale, {})
            current = locale_data.get(key)  # None if missing_key / missing_locale
            by_locale.setdefault(item.locale, []).append(
                {
                    "key": key,
                    "reason": item.reason,
                    "en": en_value,
                    "current_value": current,
                }
            )
    return dict(sorted(by_locale.items()))


def build_report(locale_dirs: list[Path]) -> dict[str, Any]:
    (
        missing_by_key,
        per_locale_count,
        locale_codes,
        orphans,
        en_keys_per_dir,
        merged_en,
        merged_locales,
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

    translation_tasks = build_translation_tasks(missing_by_key, merged_en, merged_locales)

    # Summary statistics per locale for translation_tasks
    tasks_summary: dict[str, dict[str, int]] = {}
    for locale, tasks in translation_tasks.items():
        reason_counts: dict[str, int] = {}
        for t in tasks:
            r = t["reason"]
            reason_counts[r] = reason_counts.get(r, 0) + 1
        tasks_summary[locale] = {"total": len(tasks), **reason_counts}

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
        # ── LLM/MCP-optimized section ──────────────────────────────────────────
        # `translation_tasks` is the primary surface for automated tooling.
        # Iterate over each locale, then submit its task list to an LLM.
        # The LLM should return a JSON object { key: translated_string, ... }
        # which can be merged back into the locale file directly.
        "translation_tasks_summary": tasks_summary,
        "translation_tasks": translation_tasks,
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
        default="missing_locale,missing_key,empty_value,placeholder_mismatch",
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

    # Translation tasks summary (for LLM/MCP consumers)
    tasks_summary = report.get("translation_tasks_summary", {})
    tasks = report.get("translation_tasks", {})
    if tasks_summary:
        print()
        print("📋 Translation tasks per locale (LLM/MCP view):")
        print("─" * 60)
        for locale, stats in tasks_summary.items():
            total = stats.get("total", 0)
            reasons_detail = "  ".join(
                f"{reason}={count}"
                for reason, count in stats.items()
                if reason != "total"
            )
            preview_keys = [t["key"] for t in tasks.get(locale, [])[:3]]
            preview = ", ".join(preview_keys)
            if len(tasks.get(locale, [])) > 3:
                preview += f", ... (+{len(tasks[locale]) - 3} more)"
            print(f"  {locale:<4}  {total:>3} tasks  [{reasons_detail}]")
            print(f"        e.g. {preview}")
        print()
        print(f"  ℹ  Use report['translation_tasks']['<locale>'] for the full task list.")
        print(f"     Each task: {{key, reason, en, current_value}}")

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
