# -*- coding: utf-8 -*-
"""Sparse-CSV defensive tests for build/associations.py.

Pinned by Codex PR #200 r3173989541 (P1) + r3173989546 (P2):

The SQLite-era pipeline:
- Frequency: tolerated NaN cells (stored as SQL NULL, encoded as 0)
  but ALSO let NaN poison the count accumulator across duplicate keys
  (`5 + NaN → NaN → NULL → 0`, silently zeroing real data). The
  CSV-direct rewrite intentionally drops the poisoning behavior —
  NaN is coerced to 0 per-row so a single bad cell can't zero out an
  otherwise-valid accumulator. Net difference is invisible on current
  dictionary.csv (no NaN cells); see PR #200 thread for the call.
- Source flags: treated only literal 1 / "1" / True as a set bit.
  The rewrite must preserve this so partially populated CSVs don't
  silently flip absent source flags to 1 via Python `bool(NaN)`.

Production CSVs don't carry NaN cells today, so the parity gate
(tools/compare_baseline.py verify) does not exercise these paths.
These unit tests are the only guard.

Stdlib unittest so this runs anywhere Python is — no new requirements.
Run from `dictionary/`:
    PYTHONPATH=. python3 -m unittest discover tests/
"""

from __future__ import annotations

import csv
import sys
import tempfile
import unittest
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from build.associations import compute_associations  # noqa: E402
from common.source_bits import ASSOC_SOURCE_COLUMNS  # noqa: E402


def _write_csv(path: Path, rows: list[dict]) -> None:
    columns = ["hanzi", "tl", "frequency", *ASSOC_SOURCE_COLUMNS]
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=columns)
        writer.writeheader()
        for row in rows:
            writer.writerow({col: row.get(col, "") for col in columns})


class SparseCsvDefensesTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self._tmp.name)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_nan_frequency_encodes_as_zero(self) -> None:
        """Blank `frequency` cell must produce count=0, not raise ValueError.

        End-to-end SQLite parity (Codex PR #200 r3173989541): old
        generate_association.py let NaN flow through (NaN is Python-truthy
        so `... or 1` did not normalise it), SQLite stored it as NULL, and
        create_association_bin's `count = row["count"] or 0` encoded it as 0.
        """
        csv_path = self.tmp_path / "dict.csv"
        _write_csv(csv_path, [
            # Two-char hanzi, one syllable per char, frequency cell empty.
            {"hanzi": "早安", "tl": "tsa-an", "frequency": "", "kautian": "True"},
        ])
        grouped = compute_associations(csv_path)
        self.assertIn("早", grouped)
        entries = grouped["早"]
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0].count, 0)

    def test_zero_frequency_normalises_to_one(self) -> None:
        """A row with explicit `frequency=0` must bump to 1 (parity with old `or 1`).

        `0 or 1` IS Python-falsy → 1, so the old short-circuit fired here
        (unlike the NaN case above).
        """
        csv_path = self.tmp_path / "dict.csv"
        _write_csv(csv_path, [
            {"hanzi": "早安", "tl": "tsa-an", "frequency": "0", "kautian": "True"},
        ])
        grouped = compute_associations(csv_path)
        self.assertEqual(grouped["早"][0].count, 1)

    def test_missing_source_flag_is_unset(self) -> None:
        """An empty source cell must NOT promote that bit to 1."""
        csv_path = self.tmp_path / "dict.csv"
        _write_csv(csv_path, [
            # `kautian` cell empty (NaN after pandas read), `taigitv` set.
            {"hanzi": "早安", "tl": "tsa-an", "frequency": "100",
             "kautian": "", "taigitv": "True"},
        ])
        grouped = compute_associations(csv_path)
        sources = grouped["早"][0].source_dict()
        self.assertEqual(sources["kautian"], 0, "NaN source cell must stay unset")
        self.assertEqual(sources["taigitv"], 1, "Literal True source cell must be set")

    def test_string_one_counts_as_set(self) -> None:
        """Original generate_association.py accepted `'1'` as truthy — preserve."""
        csv_path = self.tmp_path / "dict.csv"
        _write_csv(csv_path, [
            {"hanzi": "早安", "tl": "tsa-an", "frequency": "100", "kautian": "1"},
        ])
        grouped = compute_associations(csv_path)
        self.assertEqual(grouped["早"][0].source_dict()["kautian"], 1)

    def test_string_zero_does_not_count(self) -> None:
        """`'0'` must NOT be promoted to 1 (matches original narrow check)."""
        csv_path = self.tmp_path / "dict.csv"
        _write_csv(csv_path, [
            {"hanzi": "早安", "tl": "tsa-an", "frequency": "100", "kautian": "0"},
        ])
        grouped = compute_associations(csv_path)
        self.assertEqual(grouped["早"][0].source_dict()["kautian"], 0)


if __name__ == "__main__":
    unittest.main()
