#!/usr/bin/env python3
# Generates dist/emoji.json — the single emoji source of truth for the iOS + Android
# Taigi keyboard. Backbone = Unicode emoji-test.txt (order, groups, names, version, and
# skin-tone variations folded into their base); keywords from CLDR (en + zh-Hant) plus a
# hand-maintained Taigi overlay in src/overrides.tsv.

from __future__ import annotations

import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = REPO_ROOT / "data"
OVERRIDES_PATH = REPO_ROOT / "src" / "overrides.tsv"

# Canonical output — the single emoji source of truth. Consumers (the Taigi keyboard iOS +
# Android apps) read this file directly; the drift-guard test fails if the committed copy is stale.
OUTPUT_PATH = REPO_ROOT / "dist" / "emoji.json"

# Pinned version. All bundled data/ is Unicode Emoji 17.0. Per-OS rendering gaps (older OS
# fonts lacking E17 glyphs) are handled by each platform's glyph filter at load (see
# CLAUDE.md § Platform integration), not by per-version files. See data/SOURCES.md.
MAX_EMOJI_VERSION = "E17.0"
CLDR_VERSION = "48"

SKIN_TONE_RANGE = range(0x1F3FB, 0x1F3FF + 1)
VARIATION_SELECTOR_16 = 0xFE0F
ZWJ = 0x200D

# Unicode emoji-test.txt group name -> our category id + display title. Order here is
# the order groups first appear in emoji-test.txt (the canonical CLDR keyboard order).
# "Component" (skin-tone / hair modifiers) is intentionally excluded — not user-facing.
GROUP_TO_CATEGORY: dict[str, tuple[str, str]] = {
    "Smileys & Emotion": ("smileys_emotion", "Smileys & Emotion"),
    "People & Body": ("people_body", "People & Body"),
    "Animals & Nature": ("animals_nature", "Animals & Nature"),
    "Food & Drink": ("food_drink", "Food & Drink"),
    "Travel & Places": ("travel_places", "Travel & Places"),
    "Activities": ("activities", "Activities"),
    "Objects": ("objects", "Objects"),
    "Symbols": ("symbols", "Symbols"),
    "Flags": ("flags", "Flags"),
}
EXCLUDED_GROUPS = {"Component"}

# From Emoji 15.1 the detailed-family ZWJ sequences are superseded by the silhouette
# family set; drop the legacy detailed ones (mirrors azooKey policy).
LEGACY_FAMILY_EMOJIS = {
    "👨‍👦",
    "👨‍👦‍👦",
    "👨‍👧",
    "👨‍👧‍👦",
    "👨‍👧‍👧",
    "👨‍👨‍👦",
    "👨‍👨‍👦‍👦",
    "👨‍👨‍👧",
    "👨‍👨‍👧‍👦",
    "👨‍👨‍👧‍👧",
    "👨‍👩‍👦",
    "👨‍👩‍👦‍👦",
    "👨‍👩‍👧",
    "👨‍👩‍👧‍👦",
    "👨‍👩‍👧‍👧",
    "👩‍👦",
    "👩‍👦‍👦",
    "👩‍👧",
    "👩‍👧‍👦",
    "👩‍👧‍👧",
    "👩‍👩‍👦",
    "👩‍👩‍👦‍👦",
    "👩‍👩‍👧",
    "👩‍👩‍👧‍👦",
    "👩‍👩‍👧‍👧",
}

# Multi-person sequences whose skin-tone variants reduce to a base that plain modifier
# removal cannot recover (two independent tones collapse to one neutral base). Maps a
# codepoint pattern (-1 = any skin tone) to the neutral base codepoints. From azooKey.
MIXED_TONE_PATTERNS: list[tuple[list[int], list[int]]] = [
    ([0x1F9D1, -1, ZWJ, 0x2764, VARIATION_SELECTOR_16, ZWJ, 0x1F9D1, -1], [0x1F491]),
    ([0x1F9D1, -1, ZWJ, 0x2764, VARIATION_SELECTOR_16, ZWJ, 0x1F48B, ZWJ, 0x1F9D1, -1], [0x1F48F]),
    ([0x1F468, -1, ZWJ, 0x1F91D, ZWJ, 0x1F468, -1], [0x1F46C]),
    ([0x1F469, -1, ZWJ, 0x1F91D, ZWJ, 0x1F468, -1], [0x1F46B]),
    ([0x1F469, -1, ZWJ, 0x1F91D, ZWJ, 0x1F469, -1], [0x1F46D]),
    ([0x1FAF1, -1, ZWJ, 0x1FAF2, -1], [0x1F91D]),
]


@dataclass
class Emoji:
    base: str
    codepoints: list[int]
    category_id: str
    subgroup: str
    name: str
    version: str
    order: int
    variations: list[str] = field(default_factory=list)
    keywords_en: list[str] = field(default_factory=list)
    keywords_zh: list[str] = field(default_factory=list)
    keywords_tl: list[str] = field(default_factory=list)
    custom: bool = False


def _codepoints_to_str(codepoints: list[int]) -> str:
    return "".join(chr(cp) for cp in codepoints)


def _str_to_codepoints(text: str) -> list[int]:
    return [ord(ch) for ch in text]


def _strip_fe0f(text: str) -> str:
    return text.replace(chr(VARIATION_SELECTOR_16), "")


def version_le(version: str, maximum: str) -> bool:
    # Versions look like "E1.0", "E16.0".
    return float(version[1:]) <= float(maximum[1:])


def parse_emoji_test(path: Path) -> tuple[list[Emoji], dict[str, Emoji], list[str]]:
    # emoji-test.txt is authoritative for membership, order, groups, names, version.
    # Entries carrying a skin-tone modifier are folded into their base's variations,
    # not surfaced as standalone emoji. Returns (ordered bases, lookup, variation strings).
    line_re = re.compile(
        r"^(?P<cps>[0-9A-Fa-f ]+);\s*(?P<status>[\w-]+)\s*#\s*\S+\s+(?P<version>E\d+\.\d+)\s+(?P<name>.+)$"
    )
    emojis: list[Emoji] = []
    lookup: dict[str, Emoji] = {}
    variation_strings: list[str] = []
    current_group = ""
    current_subgroup = ""
    order = 0
    for raw in path.read_text(encoding="utf-8").splitlines():
        if raw.startswith("# group:"):
            current_group = raw.split(":", 1)[1].strip()
            continue
        if raw.startswith("# subgroup:"):
            current_subgroup = raw.split(":", 1)[1].strip()
            continue
        if not raw or raw.startswith("#"):
            continue
        match = line_re.match(raw)
        if not match:
            continue
        if match.group("status") != "fully-qualified":
            continue
        if current_group in EXCLUDED_GROUPS:
            continue
        category = GROUP_TO_CATEGORY.get(current_group)
        if category is None:
            continue
        codepoints = [int(cp, 16) for cp in match.group("cps").split()]
        if any(cp in SKIN_TONE_RANGE for cp in codepoints):
            variation_strings.append(_codepoints_to_str(codepoints))
            continue
        base = _codepoints_to_str(codepoints)
        emoji = Emoji(
            base=base,
            codepoints=codepoints,
            category_id=category[0],
            subgroup=current_subgroup,
            name=match.group("name").strip(),
            version=match.group("version"),
            order=order,
        )
        order += 1
        emojis.append(emoji)
        lookup[_strip_fe0f(base)] = emoji
    return emojis, lookup, variation_strings


def _matches_pattern(codepoints: list[int], pattern: list[int]) -> bool:
    if len(codepoints) != len(pattern):
        return False
    for cp, expected in zip(codepoints, pattern, strict=False):
        if expected == -1:
            if cp not in SKIN_TONE_RANGE:
                return False
        elif cp != expected:
            return False
    return True


def _reduce_to_base(codepoints: list[int]) -> str:
    # Map a skin-tone-bearing sequence to its neutral base string (FE0F-stripped).
    for pattern, base_codepoints in MIXED_TONE_PATTERNS:
        if _matches_pattern(codepoints, pattern):
            return _strip_fe0f(_codepoints_to_str(base_codepoints))
    kept = [cp for cp in codepoints if cp not in SKIN_TONE_RANGE]
    return _strip_fe0f(_codepoints_to_str(kept))


def attach_variations(lookup: dict[str, Emoji], variation_strings: list[str]) -> None:
    seen: set[str] = set()
    for variation in variation_strings:
        if variation in seen:
            continue
        base_key = _reduce_to_base(_str_to_codepoints(variation))
        emoji = lookup.get(base_key)
        if emoji is None:
            continue
        emoji.variations.append(variation)
        seen.add(variation)


def parse_cldr(path: Path) -> dict[str, list[str]]:
    # Returns FE0F-stripped emoji -> keyword list (excludes the type="tts" name lines).
    tree = ET.parse(path)
    keywords: dict[str, list[str]] = {}
    for node in tree.iter("annotation"):
        cp = node.get("cp")
        if cp is None or node.get("type") == "tts" or not node.text:
            continue
        key = _strip_fe0f(cp)
        terms = [term.strip() for term in node.text.split("|") if term.strip()]
        keywords.setdefault(key, []).extend(terms)
    return keywords


def apply_cldr(lookup: dict[str, Emoji], en: dict[str, list[str]], zh: dict[str, list[str]]) -> None:
    for key, emoji in lookup.items():
        emoji.keywords_en = list(dict.fromkeys(en.get(key, [])))
        emoji.keywords_zh = list(dict.fromkeys(zh.get(key, [])))


OVERRIDE_COLUMNS = [
    "emoji",
    "action",
    "category",
    "order",
    "taigi_keywords",
    "zh_Hant_keywords",
    "en_keywords",
    "name",
    "notes",
]


def _split_keywords(cell: str) -> list[str]:
    return [term.strip() for term in cell.split("|") if term.strip()]


def _merge(existing: list[str], cell: str) -> list[str]:
    return list(dict.fromkeys(existing + _split_keywords(cell)))


def apply_overrides(emojis: list[Emoji], lookup: dict[str, Emoji], path: Path) -> list[Emoji]:
    if not path.exists():
        return emojis
    excluded: set[str] = set()
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw.strip() or raw.startswith("#"):
            continue
        cells = raw.split("\t")
        if len(cells) < 2:
            raise ValueError(f"overrides.tsv:{line_no}: need at least emoji + action")
        row = dict(zip(OVERRIDE_COLUMNS, cells + [""] * (len(OVERRIDE_COLUMNS) - len(cells)), strict=False))
        emoji_str = row["emoji"].strip()
        action = row["action"].strip()
        key = _strip_fe0f(emoji_str)
        if not emoji_str:
            raise ValueError(f"overrides.tsv:{line_no}: empty emoji column")
        if action == "exclude":
            if key not in lookup:
                raise ValueError(f"overrides.tsv:{line_no}: exclude target {emoji_str!r} not in upstream")
            excluded.add(key)
        elif action == "patch":
            target = lookup.get(key)
            if target is None:
                raise ValueError(f"overrides.tsv:{line_no}: patch target {emoji_str!r} not in upstream")
            target.keywords_tl = _merge(target.keywords_tl, row["taigi_keywords"])
            target.keywords_zh = _merge(target.keywords_zh, row["zh_Hant_keywords"])
            target.keywords_en = _merge(target.keywords_en, row["en_keywords"])
        elif action == "add":
            category_id = row["category"].strip()
            if category_id not in {cid for cid, _ in GROUP_TO_CATEGORY.values()}:
                raise ValueError(f"overrides.tsv:{line_no}: add needs a valid category, got {category_id!r}")
            name = row["name"].strip()
            if not name:
                raise ValueError(f"overrides.tsv:{line_no}: add needs a non-empty name")
            order = int(row["order"]) if row["order"].strip() else len(emojis)
            new = Emoji(
                base=emoji_str,
                codepoints=_str_to_codepoints(emoji_str),
                category_id=category_id,
                subgroup="",
                name=name,
                version="custom",
                order=order,
                keywords_en=_split_keywords(row["en_keywords"]),
                keywords_zh=_split_keywords(row["zh_Hant_keywords"]),
                keywords_tl=_split_keywords(row["taigi_keywords"]),
                custom=True,
            )
            emojis.append(new)
            lookup[key] = new
        else:
            raise ValueError(f"overrides.tsv:{line_no}: unknown action {action!r} (use patch|add|exclude)")
    return [e for e in emojis if _strip_fe0f(e.base) not in excluded]


def filter_and_validate(emojis: list[Emoji], upstream_keys: set[str]) -> list[Emoji]:
    kept: list[Emoji] = []
    seen_bases: dict[str, str] = {}
    for emoji in emojis:
        if emoji.version != "custom" and not version_le(emoji.version, MAX_EMOJI_VERSION):
            continue
        if _strip_fe0f(emoji.base) in LEGACY_FAMILY_EMOJIS:
            continue
        if not emoji.custom and _strip_fe0f(emoji.base) not in upstream_keys:
            raise ValueError(f"validate: non-custom emoji {emoji.base!r} absent from emoji-test.txt")
        key = _strip_fe0f(emoji.base)
        if key in seen_bases:
            raise ValueError(
                f"validate: duplicate base {emoji.base!r} in {emoji.category_id} and {seen_bases[key]}"
            )
        seen_bases[key] = emoji.category_id
        kept.append(emoji)
    return kept


def _merged_keywords(emoji: Emoji) -> list[str]:
    merged: list[str] = []
    for term in [*emoji.keywords_en, *emoji.keywords_zh, *emoji.keywords_tl]:
        lowered = term.lower()
        if lowered and lowered not in merged:
            merged.append(lowered)
    return merged


def build_json(emojis: list[Emoji]) -> dict:
    ordered_category_ids = [cid for cid, _ in GROUP_TO_CATEGORY.values()]
    titles = dict(GROUP_TO_CATEGORY.values())
    buckets: dict[str, list[Emoji]] = {cid: [] for cid in ordered_category_ids}
    for emoji in emojis:
        buckets[emoji.category_id].append(emoji)

    categories = []
    for index, cid in enumerate(ordered_category_ids):
        bucket = sorted(buckets[cid], key=lambda e: e.order)
        if not bucket:
            continue
        categories.append(
            {
                "id": cid,
                "title": titles[cid],
                "order": index,
                "emoji": [
                    {
                        "base": e.base,
                        "cp": e.codepoints,
                        "name": e.name,
                        "subgroup": e.subgroup,
                        "version": e.version,
                        "variations": e.variations,
                        "keywords": _merged_keywords(e),
                        "keywordsByLocale": {
                            "en": e.keywords_en,
                            "zh_Hant": e.keywords_zh,
                            "taigi": e.keywords_tl,
                        },
                    }
                    for e in bucket
                ],
            }
        )

    count = sum(len(c["emoji"]) for c in categories)
    return {
        "meta": {
            "emojiVersion": MAX_EMOJI_VERSION,
            "cldrVersion": CLDR_VERSION,
            "count": count,
            "generator": "taigi-emojis/scripts/generate.py",
        },
        "categories": categories,
    }


def generate() -> dict:
    emojis, lookup, variation_strings = parse_emoji_test(DATA_DIR / "emoji-test.txt")
    upstream_keys = set(lookup.keys())
    attach_variations(lookup, variation_strings)
    en = parse_cldr(DATA_DIR / "cldr-en.xml")
    en_derived = parse_cldr(DATA_DIR / "cldr-en-derived.xml")
    for key, terms in en_derived.items():
        en.setdefault(key, []).extend(terms)
    zh = parse_cldr(DATA_DIR / "cldr-zh_Hant.xml")
    zh_derived = parse_cldr(DATA_DIR / "cldr-zh_Hant-derived.xml")
    for key, terms in zh_derived.items():
        zh.setdefault(key, []).extend(terms)
    apply_cldr(lookup, en, zh)
    emojis = apply_overrides(emojis, lookup, OVERRIDES_PATH)
    emojis = filter_and_validate(emojis, upstream_keys)
    return build_json(emojis)


def main() -> int:
    document = generate()
    payload = json.dumps(document, ensure_ascii=False, indent=2) + "\n"
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(payload, encoding="utf-8")
    print(f"wrote {document['meta']['count']} emoji to {OUTPUT_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
