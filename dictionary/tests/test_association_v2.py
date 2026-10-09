# -*- coding: utf-8 -*-
"""association.bin v2 word namespace — build/associations.py + the writer.

Bigram LM roadmap P3 (docs/architecture/bigram-lm-roadmap.md § D3). Fixtures
are tiny hand-written dictionary / bigram / variants files; the real-data
round trip is `create_association_bin --verify` in build.sh.

Run from `dictionary/`:
    PYTHONPATH=. python3 -m pytest tests/test_association_v2.py -q
"""

from __future__ import annotations

import csv
import logging
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from build import create_association_bin as writer  # noqa: E402
from build.associations import (  # noqa: E402
    WORD_TOP_K,
    compute_associations,
    compute_word_associations,
    load_variant_folds,
    weighted_count,
    word_key,
)
from build.dictionary_records import REQUIRED_COLUMNS  # noqa: E402
from common.source_bits import DICT_BIN_COLUMNS  # noqa: E402
from common.variants import read_variant_rows  # noqa: E402

DICT_COLUMNS = [
    *REQUIRED_COLUMNS, "kautian_main", "kautian_accent_mask", "kautian_name", "kautian_alt_reading",
]
SOURCES = ["moe_kautian", "taigi_bible_nt"]
TSV_HEADER = ["prev_hanji", "prev_tl", "next_hanji", "next_tl", "count", *SOURCES]


def write_dictionary(path: Path, rows: list[tuple[str, str, str]]) -> None:
    """rows = (hanzi, tl, source column that is True)."""
    with path.open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=DICT_COLUMNS)
        w.writeheader()
        for hanzi, tl, source in rows:
            # Flags are literal True/False as merge_csv writes them (an empty
            # cell reads as NaN, which `bool()` turns into True).
            row = {col: "False" for col in DICT_BIN_COLUMNS}
            row.update({
                "hanzi": hanzi, "tl": tl, "frequency": 10, source: "True",
                "kautian_main": str(source == "kautian"), "kautian_accent_mask": 0, "kautian_name": "False",
                "kautian_alt_reading": "False",
            })
            w.writerow(row)


def write_bigrams(path: Path, rows: list[tuple[str, str, str, str, int, int]]) -> None:
    """rows = (prev_hanji, prev_tl, next_hanji, next_tl, moe count, bible count)."""
    with path.open("w", newline="", encoding="utf-8") as f:
        f.write("\t".join(TSV_HEADER) + "\n")
        for prev_hanji, prev_tl, next_hanji, next_tl, moe, bible in rows:
            f.write("\t".join(map(str, (prev_hanji, prev_tl, next_hanji, next_tl, moe + bible, moe, bible))) + "\n")


def write_variants(path: Path, rows: list[tuple[str, str, str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as f:
        f.write("hanzi,variant,tl\n")
        for hanzi, variant, tl in rows:
            f.write(f"{hanzi},{variant},{tl}\n")


class WordAssociationTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        tmp = Path(self._tmp.name)
        self.dictionary = tmp / "dictionary.csv"
        self.bigrams = tmp / "word_bigrams.tsv"
        self.variants = tmp / "variants.csv"
        write_dictionary(self.dictionary, [
            ("一", "tsi̍t", "kautian"),
            ("个", "ê", "kautian"),
            ("個", "ê", "taihoa"),
            ("佮", "kah", "kautian"),
            ("甲", "kah", "taihoa"),
            ("不好", "m̄-hó", "taihoa"),      # variant whose 教典 form 毋好 is not a row
            ("人", "lâng", "kautian"),
            ("耶穌", "iâ-soo", "taihoa"),
        ])
        write_variants(self.variants, [("佮", "甲", "kah/kap"), ("毋好", "不好", "m̄-hó")])
        logging.disable(logging.CRITICAL)

    def tearDown(self) -> None:
        logging.disable(logging.NOTSET)
        self._tmp.cleanup()

    def compute(self, rows):
        write_bigrams(self.bigrams, rows)
        return compute_word_associations(self.bigrams, self.dictionary, self.variants)

    def test_load_variant_folds_splits_readings_and_adds_classifier_rule(self) -> None:
        folds = load_variant_folds(self.variants)
        self.assertEqual(folds[("甲", "kah")], "佮")
        self.assertEqual(folds[("甲", "kap")], "佮")
        self.assertEqual(folds[("個", "ê")], "个")

    def test_short_or_blank_variant_rows_are_skipped(self) -> None:
        self.variants.write_text("hanzi,variant,tl\n佮,甲,kah\n毋\n,青,tshenn\n欲,卜,\n", encoding="utf-8")
        self.assertEqual(list(read_variant_rows(self.variants)), [("佮", "甲", "kah")])

    def test_ambiguous_variant_is_not_folded(self) -> None:
        write_variants(self.variants, [("生", "青", "tshenn"), ("腥", "青", "tshenn")])
        self.assertNotIn(("青", "tshenn"), load_variant_folds(self.variants))

    def test_prev_and_next_fold_and_merge_counts(self) -> None:
        grouped = self.compute([
            ("一", "tsi̍t", "個", "ê", 3, 0),
            ("一", "tsi̍t", "个", "ê", 2, 0),
            ("甲", "kah", "人", "lâng", 4, 0),
            ("佮", "kah", "人", "lâng", 1, 0),
        ])
        self.assertNotIn(word_key("甲", "kah"), grouped)
        (entry,) = grouped[word_key("一", "tsi̍t")]
        self.assertEqual((entry.next_word, entry.next_tl, entry.count), ("个", "ê", 5))
        (entry,) = grouped[word_key("佮", "kah")]
        self.assertEqual((entry.next_word, entry.count), ("人", 5))

    def test_fold_target_missing_from_dictionary_keeps_variant(self) -> None:
        grouped = self.compute([("人", "lâng", "不好", "m̄-hó", 2, 0)])
        (entry,) = grouped[word_key("人", "lâng")]
        self.assertEqual(entry.next_word, "不好")

    def test_bible_weight_half_up_then_min_count(self) -> None:
        self.assertEqual(weighted_count({"moe_kautian": 0, "taigi_bible_nt": 3}), 2)  # 1.5 → 2
        self.assertEqual(weighted_count({"moe_kautian": 0, "taigi_bible_nt": 5}), 3)  # 2.5 → 3
        grouped = self.compute([
            ("人", "lâng", "耶穌", "iâ-soo", 0, 2),   # 1.0 → below WORD_MIN_COUNT
            ("人", "lâng", "个", "ê", 1, 2),          # 2.0
        ])
        self.assertEqual([e.next_word for e in grouped[word_key("人", "lâng")]], ["个"])

    def test_bitmask_is_the_next_words_dictionary_flags(self) -> None:
        grouped = self.compute([("人", "lâng", "個", "ê", 2, 0)])
        (entry,) = grouped[word_key("人", "lâng")]
        self.assertEqual(entry.next_word, "个", "folded before the flag lookup")
        self.assertEqual(entry.source_dict()["kautian"], 1)
        self.assertEqual(entry.source_dict()["taihoa"], 0)

    def test_next_word_outside_dictionary_is_dropped(self) -> None:
        grouped = self.compute([("人", "lâng", "毋好", "m̄-hó", 9, 0)])
        self.assertEqual(grouped, {})

    def test_top_k_and_deterministic_tiebreak(self) -> None:
        rows = [("人", "lâng", h, tl, 2, 0) for h, tl in
                [("一", "tsi̍t"), ("个", "ê"), ("佮", "kah"), ("耶穌", "iâ-soo")]]
        with mock.patch("build.associations.WORD_TOP_K", 3):
            grouped = self.compute(rows)
        by_bytes = sorted(["一", "个", "佮", "耶穌"], key=lambda s: s.encode("utf-8"))[:3]
        self.assertEqual([e.next_word for e in grouped[word_key("人", "lâng")]], by_bytes)
        self.assertGreaterEqual(WORD_TOP_K, 3)


class CharacterNamespaceTests(unittest.TestCase):
    """v1 character pairs: unchanged for BMP hanji; Extension F/G hanji now count."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.dictionary = Path(self._tmp.name) / "dictionary.csv"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_bmp_word_yields_the_same_character_pair(self) -> None:
        write_dictionary(self.dictionary, [("好人", "hó-lâng", "kautian")])
        (entry,) = compute_associations(self.dictionary)["好"]
        self.assertEqual((entry.next_word, entry.next_tl, entry.count), ("人", "lâng", 10))

    def test_extension_g_hanji_is_a_character_key(self) -> None:
        # U+308FB is an Extension G character the dictionary carries; the old
        # per-file range stopped at Extension E and dropped this pair.
        write_dictionary(self.dictionary, [("𰣻人", "hó-lâng", "kautian")])
        self.assertEqual([e.next_word for e in compute_associations(self.dictionary)["𰣻"]], ["人"])


class WriterTests(unittest.TestCase):
    """Both namespaces in one file; keys byte-sorted; `--verify` round-trips."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        tmp = Path(self._tmp.name)
        dictionary, bigrams, variants, output = (
            tmp / "dictionary.csv", tmp / "word_bigrams.tsv", tmp / "variants.csv", tmp / "association.bin",
        )
        write_dictionary(dictionary, [("好", "hó", "kautian"), ("好人", "hó-lâng", "kautian"), ("人", "lâng", "kautian")])
        write_bigrams(bigrams, [("好", "hó", "人", "lâng", 3, 0)])
        write_variants(variants, [])
        self.output = output
        self.patches = [
            mock.patch.object(writer, "CSV_FILE", dictionary),
            mock.patch.object(writer, "BIGRAMS_TSV", bigrams),
            mock.patch.object(writer, "OUTPUT_FILE", output),
            mock.patch("build.associations.VARIANTS_CSV", variants),
            mock.patch.object(writer, "build_id", lambda: 7),
        ]
        for p in self.patches:
            p.start()
        logging.disable(logging.CRITICAL)

    def tearDown(self) -> None:
        logging.disable(logging.NOTSET)
        for p in self.patches:
            p.stop()
        self._tmp.cleanup()

    def read_keys(self) -> list[str]:
        data = self.output.read_bytes()
        version, key_count = struct.unpack_from("<II", data, 4)
        self.assertEqual(version, 2)
        keys = []
        for i in range(key_count):
            offset = struct.unpack_from("<I", data, 20 + i * 4)[0]
            length = data[offset]
            keys.append(data[offset + 1: offset + 1 + length].decode("utf-8"))
        return keys

    def test_character_and_word_keys_share_one_byte_sorted_section(self) -> None:
        logger = logging.getLogger("test")
        grouped = writer.build(logger)
        keys = self.read_keys()
        self.assertEqual(keys, ["好", "好\x01hó"])
        self.assertEqual(keys, sorted(keys, key=lambda k: k.encode("utf-8")))
        self.assertEqual([e.next_word for e in grouped["好"]], ["人"], "v1 character pair from 好人")
        writer.verify(logger, grouped)  # exits non-zero on any mismatch


if __name__ == "__main__":
    unittest.main()
