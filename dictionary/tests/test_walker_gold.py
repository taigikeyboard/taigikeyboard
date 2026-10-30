"""tools/walker_gold.py — input variants, excerpt windows and the offline walker simulator.

Run from `dictionary/`:
    PYTHONPATH=. python3 -m pytest tests/test_walker_gold.py -q
"""

from __future__ import annotations

import sys
import unittest
from collections import defaultdict
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from build.walker_lm import BigramCounts, WalkerModel
from tools import walker_gold
from tools.walker_gold import Item, Resolved, Row, Simulator, windows


def row(hanzi: str, tl: str, tl_num: str, frequency: int, tl_notone: str, syllable_count: int) -> Row:
    return Row(hanzi, tl, frequency, tl_num, tl_notone, tl_num, tl_notone, "", "", syllable_count, True)


KAU_SIU = row("教授", "kàu-siū", "kau3siu7", 10, "kausiu", 2)
KAU = row("到", "kàu", "kau3", 17_333, "kau", 1)
SIU = row("受", "siū", "siu7", 8_975, "siu", 1)


class InputVariantTests(unittest.TestCase):
    def test_inputs_cover_every_variant_from_the_dictionary_keys(self) -> None:
        resolved = Resolved(Item("dev-0000", "dev", "phrase", "dict:"), [KAU_SIU, KAU])
        inputs = resolved.inputs()
        self.assertEqual(inputs["tl_full"], "kau3siu7kau3")
        # Digit on each word's first syllable only.
        self.assertEqual(inputs["tl_partial"], "kau3siukau3")
        self.assertEqual(inputs["tl_toneless"], "kausiukau")
        self.assertEqual(inputs["tl_hyphen"], "kau3-siu7-kau3")


class WindowTests(unittest.TestCase):
    def test_windows_stop_at_breaks_and_respect_the_syllable_range(self) -> None:
        words = [KAU, KAU_SIU, None, KAU]
        # A lone one-syllable word is below MIN_SYLLABLES; nothing crosses the break.
        self.assertEqual(windows(words), [(0, 2), (1, 2)])


class SimulatorTests(unittest.TestCase):
    def simulator(self, counts: dict[str, int]) -> Simulator:
        by_tl_num = defaultdict(list)
        for r in (KAU_SIU, KAU, SIU):
            by_tl_num[r.tl_num].append(r)
        word_counts = {(r.hanzi, r.tl_num): counts[r.hanzi] for r in (KAU_SIU, KAU, SIU) if r.hanzi in counts}
        dictionary = SimpleNamespace(
            by_tl_num=by_tl_num,
            walker_model=WalkerModel(word_counts, tokens=3_627_713, vocabulary=167_907, alpha=1.0),
        )
        return Simulator(dictionary, picks={})

    def test_pre_p3_model_composes_the_single_characters(self) -> None:
        # trace: pre-P3 cost 到 5.3177 + 受 5.8460 = 11.1636 < 教授 11.2279
        path = self.simulator({}).best_path(["kau3", "siu7"], "pre-p3", 0.0)
        self.assertEqual([r.hanzi for r in path], ["到", "受"])

    def test_a2_model_keeps_the_word_on_corpus_counts(self) -> None:
        # trace: a2@0.5, denominator 3,627,713 + 0.5 × 167,907; cost = walker_cost / 1000 / len^0.2 × syll^0.2
        # 教授 −ln(246.5/D) / 6^0.2 × 2^0.2 = 7.72; 到 4.24 + 受 5.22 = 9.46 > 7.72
        simulator = self.simulator({"教授": 246, "到": 18_902, "受": 5_574})
        path = simulator.best_path(["kau3", "siu7"], "a2", 0.5)
        self.assertEqual([r.hanzi for r in path], ["教授"])

    def test_context_walk_without_bigrams_is_the_a2_walk(self) -> None:
        no_bigrams = BigramCounts({})

        def penalty(previous: Row, pick: Row, base: float) -> float:
            return base + no_bigrams.penalty(previous.model_word, pick.model_word, base)

        for counts in ({}, {"教授": 246, "到": 18_902, "受": 5_574}):
            simulator = self.simulator(counts)
            for syllables in (["kau3", "siu7"], ["kau3", "kau3siu7"], ["siu7", "kau3", "siu7"]):
                with self.subTest(counts=counts, syllables=syllables):
                    self.assertEqual(
                        simulator.best_path(syllables, "a2", 1.0, penalty),
                        simulator.best_path(syllables, "a2", 1.0),
                    )

    def test_penalty_after_the_previous_pick_keeps_the_word(self) -> None:
        # trace: α 1, D = 3,627,713 + 167,907 = 3,795,620. 教授 unseen −ln(1/D) = 15.149 / 6^0.2 × 2^0.2
        # = 12.161; 到 ln(D/18,903) = 5.302 / 3^0.2 = 4.256; 受 ln(D/5,575) = 6.523 / 3^0.2 = 5.237
        # → unigram split 9.493 < 12.161. 到 has one other continuation (C 1,000, N 1): 受 after 到 pays
        # −ln(0.75 / 1,000) = 7.195 → (6.523 + 7.195) / 3^0.2 = 11.013 → split 15.269 > 12.161.
        simulator = self.simulator({"到": 18_902, "受": 5_574})
        self.assertEqual([r.hanzi for r in simulator.best_path(["kau3", "siu7"], "a2", 1.0)], ["到", "受"])
        bigrams = BigramCounts({("到", "kau3"): {("教", "kau3"): 1_000}})
        path = simulator.best_path(
            ["kau3", "siu7"],
            "a2",
            1.0,
            lambda previous, pick, base: base + bigrams.penalty(previous.model_word, pick.model_word, base),
        )
        self.assertEqual([r.hanzi for r in path], ["教授"])


class HanjiCountsTests(unittest.TestCase):
    def test_spellings_that_share_a_model_word_count_it_once(self) -> None:
        # 猴山仔 has two spellings of one model word (count 12); 猴猻仔 is its own word (17).
        # Adding per row would give 猴山仔 24 and a wrong winner.
        kau_san = row("猴山仔", "kâu-san-á", "kau5san1a2", 0, "kausana", 3)
        kau_san_neutral = row("猴山仔", "kâu-san--á", "kau5san1a2", 0, "kausana", 3)
        kau_sun = row("猴猻仔", "kâu-sun-á", "kau5san1a2", 0, "kausana", 3)
        model = WalkerModel(
            {("猴山仔", "kau5san1a2"): 12, ("猴猻仔", "kau5san1a2"): 17}, tokens=29, vocabulary=2, alpha=1.0
        )
        counts = walker_gold.hanji_counts([kau_san, kau_san_neutral, kau_sun], SimpleNamespace(walker_model=model))
        self.assertEqual(dict(counts), {"猴山仔": 12, "猴猻仔": 17})


class ResolveAllTests(unittest.TestCase):
    ITEMS = (Item("dev-0000", "dev", "rare", "dict:"), Item("dev-0001", "dev", "unseen", "dict:"))

    def resolve_all(self, results: dict[str, Resolved | None]):
        with (
            mock.patch.object(walker_gold, "read_gold", lambda: self.ITEMS),
            mock.patch.object(walker_gold, "resolve_item", lambda item, _: results[item.id]),
        ):
            return walker_gold.resolve_all(SimpleNamespace())

    def test_an_item_that_no_longer_resolves_exits(self) -> None:
        # The converter or a corpus submodule missing turns every corpus item into None.
        ok = Resolved(self.ITEMS[0], [KAU])
        with self.assertRaises(SystemExit) as raised:
            self.resolve_all({"dev-0000": ok, "dev-0001": None})
        self.assertIn("dev-0001", str(raised.exception))

    def test_an_unsupported_key_is_skipped_with_its_reason(self) -> None:
        bad = row("洗盪", "seré-tň̄g", "sere2tn̄g6", 0, "seretng", 2)
        resolved, skipped = self.resolve_all(
            {"dev-0000": Resolved(self.ITEMS[0], [KAU]), "dev-0001": Resolved(self.ITEMS[1], [bad])}
        )
        self.assertEqual([r.item.id for r in resolved], ["dev-0000"])
        self.assertEqual([(i.id, reason) for i, reason in skipped], [("dev-0001", "unsupported tl_num ['sere2tn̄g6']")])


if __name__ == "__main__":
    unittest.main()
