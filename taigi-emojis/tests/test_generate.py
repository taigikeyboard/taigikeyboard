"""Executable spec for the emoji generator. Golden cases cover the grouping edge
cases Codex flagged (skin tone, mixed-tone multi-person, hair, direction, keycap,
tag flags, FE0F) plus a drift guard that pins committed dist/emoji.json to a
fresh regenerate."""

import json
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import generate  # noqa: E402

SKIN_TONE_RANGE = range(0x1F3FB, 0x1F3FF + 1)
EXPECTED_CATEGORY_ORDER = [
    "smileys_emotion",
    "people_body",
    "animals_nature",
    "food_drink",
    "travel_places",
    "activities",
    "objects",
    "symbols",
    "flags",
]


@pytest.fixture(scope="module")
def document() -> dict:
    return generate.generate()


@pytest.fixture(scope="module")
def by_base(document: dict) -> dict[str, dict]:
    return {em["base"]: em for cat in document["categories"] for em in cat["emoji"]}


def test_categories_ordered_and_nonempty(document: dict) -> None:
    ids = [cat["id"] for cat in document["categories"]]
    assert ids == EXPECTED_CATEGORY_ORDER
    for cat in document["categories"]:
        assert cat["emoji"], f"{cat['id']} is empty"


def test_count_is_reasonable(document: dict) -> None:
    assert document["meta"]["count"] > 1800
    assert document["meta"]["emojiVersion"] == "E17.0"


def test_first_emoji_is_grinning_face(document: dict) -> None:
    first = document["categories"][0]["emoji"][0]
    assert first["base"] == "😀"
    assert first["cp"] == [0x1F600]
    assert "grinning face" == first["name"]


def test_no_base_carries_a_skin_tone_modifier(by_base: dict) -> None:
    for base, emoji in by_base.items():
        assert not any(cp in SKIN_TONE_RANGE for cp in emoji["cp"]), f"{base} is a tone variant, not a base"


def test_simple_skin_tone_folds_into_variations(by_base: dict) -> None:
    wave = by_base["👋"]
    assert len(wave["variations"]) == 5
    for variant in wave["variations"]:
        assert any(ord(ch) in SKIN_TONE_RANGE for ch in variant)


@pytest.mark.parametrize("base", ["🤝", "🧑‍🤝‍🧑", "👩‍❤️‍💋‍👨"])
def test_mixed_tone_multi_person_folds(by_base: dict, base: str) -> None:
    assert by_base[base]["variations"], f"{base} should have folded tone variants"


def test_hair_component_stays_a_base(by_base: dict) -> None:
    # Red/curly/white/bald-hair people are distinct emoji, not skin-tone variants.
    assert "👨‍🦰" in by_base


def test_direction_variant_present(by_base: dict) -> None:
    assert "🚶‍➡️" in by_base


def test_keycap_and_tag_flag_present(by_base: dict) -> None:
    assert "1️⃣" in by_base
    assert "🏴󠁧󠁢󠁥󠁮󠁧󠁿" in by_base  # England subdivision tag flag


def test_fe0f_base_preserved(by_base: dict) -> None:
    assert "☺️" in by_base  # smiling face keeps its variation selector


def test_legacy_detailed_family_dropped(by_base: dict) -> None:
    assert "👨‍👩‍👦" not in by_base


def test_override_patch_applies_taigi_keyword(by_base: dict) -> None:
    grin = by_base["😀"]
    assert "笑" in grin["keywordsByLocale"]["taigi"]
    assert "笑" in grin["keywords"]


def test_keywords_merged_lowercased_deduped(by_base: dict) -> None:
    grin = by_base["😀"]
    merged = grin["keywords"]
    assert len(merged) == len(set(merged))
    assert all(term == term.lower() for term in merged)


def test_no_duplicate_base_across_output(by_base: dict, document: dict) -> None:
    total = sum(len(cat["emoji"]) for cat in document["categories"])
    assert total == len(by_base)


def test_generation_is_deterministic() -> None:
    assert generate.generate() == generate.generate()


def test_committed_output_matches_regenerate(document: dict) -> None:
    committed = json.loads(generate.OUTPUT_PATH.read_text(encoding="utf-8"))
    assert committed == document, f"{generate.OUTPUT_PATH} is stale — run `make build`"


def _single_base(base: str = "😀") -> tuple[list, dict]:
    emoji = generate.Emoji(
        base=base,
        codepoints=[ord(ch) for ch in base],
        category_id="smileys_emotion",
        subgroup="",
        name="placeholder",
        version="E1.0",
        order=0,
    )
    return [emoji], {generate._strip_fe0f(base): emoji}


def test_override_empty_emoji_rejected(tmp_path) -> None:
    emojis, lookup = _single_base()
    path = tmp_path / "overrides.tsv"
    path.write_text("\tpatch\n", encoding="utf-8")
    with pytest.raises(ValueError, match="empty emoji"):
        generate.apply_overrides(emojis, lookup, path)


def test_override_exclude_unknown_rejected(tmp_path) -> None:
    emojis, lookup = _single_base()
    path = tmp_path / "overrides.tsv"
    path.write_text("🦄\texclude\n", encoding="utf-8")
    with pytest.raises(ValueError, match="exclude target"):
        generate.apply_overrides(emojis, lookup, path)


def test_override_add_requires_name(tmp_path) -> None:
    emojis, lookup = _single_base()
    path = tmp_path / "overrides.tsv"
    path.write_text("🦄\tadd\tsmileys_emotion\t\t\t\t\t\n", encoding="utf-8")
    with pytest.raises(ValueError, match="name"):
        generate.apply_overrides(emojis, lookup, path)
