"""Walker cost of every `dictionary.bin` record — E1 plan §3 / D1
(`docs/architecture/unified-word-frequency-roadmap.md`).

One unigram model over the segmented corpus counts in
`shared/data/word_unigrams.tsv`, with the `taigi_typing` column held out (it
feeds the walker gold set's calib / final splits):

    p(w)           = (c(w) + α) / (N + α·V)          Lidstone add-α
    walker_cost(w) = round_half_even(−ln p(w) × 1000)  milli-nats, u16

- A model word is `(hanzi, tl_num.lower())`, the token `corpus_bigrams`
  counts. The counter cannot tell `tshut-lâi` from `tshut--lâi`, so records
  sharing that key share one count and one cost.
- V = distinct model words over the records; N = Σ counts.
- A word with no count takes the same smoothed value — there is no fallback
  to `DictionaryRecord.frequency`, which keeps feeding the candidate-list sort.

`create_dictionary_bin` calls `record_costs` and writes the result into the
v4 `walker_cost` field; the engine prices on it from P3 on. `corpus_bigrams`
builds its subsegment tie-break model through the same `model_from_counts`.
"""

from __future__ import annotations

import csv
import math
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from build.common import UNIGRAMS_TSV, WALKER_ALPHA
from build.dictionary_records import DictionaryRecord

ALPHA = WALKER_ALPHA  # `build.common`: build_id hashes it
# Typing-practice articles: the gold set's calib / final excerpts.
HELD_OUT_SOURCE = "taigi_typing"
MILLI_NATS = 1000
MAX_WALKER_COST = 0xFFFF

ModelWord = tuple[str, str]


def model_word(record: DictionaryRecord) -> ModelWord:
    """The corpus counter's token for this record (`corpus_bigrams.load_lexicon`)."""
    return (record.hanzi or "", (record.tl_num or record.tl).lower())


@dataclass(frozen=True)
class WalkerModel:
    counts: dict[ModelWord, int]
    tokens: int  # N
    vocabulary: int  # V
    alpha: float

    def cost(self, word: ModelWord) -> int:
        return self.cost_of_count(self.counts.get(word, 0))

    def cost_of_count(self, count: int) -> int:
        # Capped at 1 (cost 0) like the engine's `WalkerModel::cost_of_count`:
        # an effective user count may exceed a tiny fixture corpus.
        probability = min((count + self.alpha) / (self.tokens + self.alpha * self.vocabulary), 1.0)
        milli_nats = round(-math.log(probability) * MILLI_NATS)  # round() is half-to-even
        if milli_nats > MAX_WALKER_COST:
            raise ValueError(f"walker cost {milli_nats} (count {count}) exceeds u16")
        return milli_nats

    @property
    def unseen_cost(self) -> int:
        return self.cost_of_count(0)


def model_from_counts(
    per_source: Mapping[ModelWord, Mapping[str, int]],
    vocabulary: int,
    alpha: float,
) -> WalkerModel:
    """Model over per-source counts, the `HELD_OUT_SOURCE` column left out."""
    if not (math.isfinite(alpha) and alpha > 0):
        raise ValueError(f"alpha must be finite and positive, got {alpha}")
    counts = {word: sum(sources.values()) - sources.get(HELD_OUT_SOURCE, 0) for word, sources in per_source.items()}
    return WalkerModel(counts, sum(counts.values()), vocabulary, alpha)


def load_model(
    records: list[DictionaryRecord],
    unigrams_tsv: Path = UNIGRAMS_TSV,
    alpha: float = ALPHA,
) -> WalkerModel:
    """Counts per model word; fails on a malformed or unmatched TSV row."""
    word_by_row = {(r.hanzi, r.tl): model_word(r) for r in records if r.hanzi}
    per_source: dict[ModelWord, dict[str, int]] = {}
    with unigrams_tsv.open(encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        sources = [c for c in reader.fieldnames or [] if c not in ("hanji", "tl", "count")]
        if HELD_OUT_SOURCE not in sources:
            raise ValueError(f"{unigrams_tsv.name}: no `{HELD_OUT_SOURCE}` column")
        for line, row in enumerate(reader, start=2):
            row_counts = {s: int(row[s]) for s in sources}
            total = int(row["count"])
            if min(row_counts.values()) < 0 or total != sum(row_counts.values()):
                raise ValueError(f"{unigrams_tsv.name}:{line}: count {total} != Σ sources {list(row_counts.values())}")
            word = word_by_row.get((row["hanji"], row["tl"]))
            if word is None:
                raise ValueError(f"{unigrams_tsv.name}:{line}: {row['hanji']}/{row['tl']} is not a dictionary record")
            if word in per_source:
                raise ValueError(f"{unigrams_tsv.name}:{line}: second row for model word {word}")
            per_source[word] = row_counts
    return model_from_counts(per_source, len({model_word(r) for r in records}), alpha)


def record_costs(records: list[DictionaryRecord], model: WalkerModel) -> list[int]:
    """`walker_cost` per record, in rowid order."""
    return [model.cost(model_word(r)) for r in records]


def stats_text(records: list[DictionaryRecord], model: WalkerModel, costs: list[int]) -> str:
    """`output/walker_lm_stats.txt`: the model's parameters and the cost distribution."""
    ordered = sorted(costs)

    def percentile(q: float) -> int:
        return ordered[min(len(ordered) - 1, int(q * len(ordered)))]

    counted = sum(1 for r in records if model.counts.get(model_word(r), 0) > 0)
    lines = [
        f"alpha={model.alpha!r}",
        f"held_out={HELD_OUT_SOURCE}",
        "model_word=(hanzi, tl_num.lower())",
        f"tokens_N={model.tokens}",
        f"vocabulary_V={model.vocabulary}",
        f"records={len(records)}",
        f"records_with_count={counted}",
        f"unseen_cost={model.unseen_cost}",
        f"cost_min={ordered[0]}",
        f"cost_p10={percentile(0.10)}",
        f"cost_p50={percentile(0.50)}",
        f"cost_p90={percentile(0.90)}",
        f"cost_max={ordered[-1]}",
    ]
    return "\n".join(lines) + "\n"
