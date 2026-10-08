"""E1 walker gold set — pick pointers, resolve them against the corpus, simulate the walker cost offline.

Plan: docs/architecture/unified-word-frequency-roadmap.md §5.

The committed gold file (engine/composing/tests/data/walker_gold.tsv) holds pointers
and judgements only, never corpus text (corpus/README.md licensing). `resolve` reads
the excerpts from the corpus/taigi-typing submodule and writes the full rows to
engine/target/walker_gold/, which engine/composing/tests/walker_gold.rs types through
the production engine.

Usage (from dictionary/):
  PYTHONPATH=. python3 -m tools.walker_gold select     # rewrite the gold file (needs corpus/taigi-corpus too)
  PYTHONPATH=. python3 -m tools.walker_gold resolve    # pointers → resolved.tsv + word_inputs.tsv + edge_keys.tsv
  cargo test … --test prod walker_gold -- --ignored     # engine metrics + edge_picks.tsv (see walker_gold.rs)
  PYTHONPATH=. python3 -m tools.walker_gold d3         # runtime edge pick vs corpus winner (plan D3)
  PYTHONPATH=. python3 -m tools.walker_gold simulate   # pre-P3 vs A2 cost over the resolved items
  PYTHONPATH=. python3 -m tools.walker_gold exposure   # pre-P3 vs A2 over every 2–3 syllable dictionary key
"""

from __future__ import annotations

import argparse
import csv
import math
import random
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, replace
from functools import cache
from pathlib import Path

from build.common import BASE_DIR, MERGED_CSV, WALKER_ALPHA
from build.corpus_bigrams import (
    SOURCES,
    TAIGI_TYPING_ARTICLES,
    AlignError,
    align_unit,
    convert_readings,
    js_array_to_json,
    key_of,
    normalize_sentence,
    typing_article_lines,
)
from build.dictionary_records import load_dictionary_records
from build.walker_lm import HELD_OUT_SOURCE, MILLI_NATS, load_model
from common.notone import remove_tone

REPO_DIR = BASE_DIR.parent
GOLD_TSV = REPO_DIR / "engine" / "composing" / "tests" / "data" / "walker_gold.tsv"
OUTPUT_DIR = REPO_DIR / "engine" / "target" / "walker_gold"
RESOLVED_TSV = OUTPUT_DIR / "resolved.tsv"
WORD_INPUTS_TSV = OUTPUT_DIR / "word_inputs.tsv"
EDGE_KEYS_TSV = OUTPUT_DIR / "edge_keys.tsv"
EDGE_PICKS_TSV = OUTPUT_DIR / "edge_picks.tsv"
EXAMPLE_SENTENCES = TAIGI_TYPING_ARTICLES.parent / "exampleSentences.js"

SEED = 20261008
# Typing articles split by article so no calib line shares an article with a final line.
CALIB_ARTICLES = {10, 31, 32, 33, 35}
FINAL_ARTICLES = {1, 21, 22, 23, 34}
DEV_PHRASES = 120
DEV_PER_SINGLE_WORD_CATEGORY = 15
DICT_QUOTA = {"unseen": 40, "rare": 20, "variant": 20}
WINDOWS_PER_TYPING_LINE = 2
MIN_SYLLABLES, MAX_SYLLABLES, MAX_WORDS, MAX_EDGE_SYLLABLES = 2, 8, 4, 4
COMMON_COUNT = 100
# The sources a fresh install enables (desktop `DictionarySourceToggles::DEFAULT`; iOS and
# Android defaults agree). walker_gold.rs builds the same filter from named toggles.
DEFAULT_SOURCES = frozenset({"kautian", "taigitv", "kungge", "stti", "khpoo", "dev", "lkk"})
# Fixed regression inputs (plan §5), stored as dictionary items.
REGRESSION_ITEMS = (
    "教授/kàu-siū",
    "台語/tâi-gí",
    "予我/hōo--guá",
    "台灣/tâi-uân",
    "囡仔人/gín-á-lâng+食飯/tsia̍h-pn̄g+袂使/bē-sái",
)
# Words are joined with `+`, which no hanji or TL contains; a TL may contain a space.
WORD_SEPARATOR = "+"
INPUT_VARIANTS = (
    "tl_full",
    "tl_partial",
    "tl_toneless",
    "tl_hyphen",
    "poj_full",
    "poj_toneless",
    "tps_full",
    "tps_toneless",
)
NUMERIC_SYLLABLE = re.compile(r"[a-z]+[0-9]?")
# A fully toned key the input variants can be derived from. 55 dictionary rows carry a
# non-ASCII `tl_num` (`sere2tn̄g6` for 洗盪), which NUMERIC_SYLLABLE would mis-split.
SUPPORTED_KEY = re.compile(r"(?:[a-z]+[0-9]?)+")
DISPLAY_SYLLABLE_SEPARATOR = re.compile(r"[-\s]+")
GOLD_HEADER = ["id", "split", "category", "ref", "extra_accepted"]


# ------------------------------------------------------------------ dictionary
@dataclass(frozen=True)
class Row:
    hanzi: str
    tl: str
    frequency: int
    tl_num: str
    tl_notone: str
    poj_num: str
    poj_notone: str
    tps_num: str
    tps_notone: str
    syllable_count: int  # `DictionaryRecord.syllable_count`, the value the engine prices with
    is_visible_by_default: bool

    @property
    def plain_hanji(self) -> str:
        return self.hanzi.replace("-", "")

    @property
    def display_syllables(self) -> int:
        """Syllables as the walker's displayed roman shows them (walker_gold.rs `syllable_counts`)."""
        return len([s for s in DISPLAY_SYLLABLE_SEPARATOR.split(self.tl) if s])


class Dictionary:
    """The `dictionary.bin` records plus the walker model over them (`build.walker_lm`)."""

    def __init__(self) -> None:
        records = load_dictionary_records(MERGED_CSV)
        self.walker_model = load_model(records)
        self.rows: list[Row] = []
        for record in records:
            if record.hanzi is None or not record.tl_num:
                continue
            sources = record.source_dict()
            self.rows.append(
                Row(
                    record.hanzi,
                    record.tl,
                    record.frequency or 0,
                    record.tl_num.lower(),
                    record.tl_notone or "",
                    (record.poj_num or "").lower(),
                    record.poj_notone or "",
                    record.tps_num or "",
                    record.tps_notone or "",
                    record.syllable_count,
                    not sources["is_variant"]
                    and not sources["khiin"]
                    and any(sources[name] for name in DEFAULT_SOURCES),
                )
            )
        # First row wins, as in `corpus_bigrams.load_lexicon`.
        self.by_word: dict[tuple[str, str], Row] = {}
        for row in self.rows:
            self.by_word.setdefault((row.hanzi, row.tl_num), row)
        # Lattice edges and homophones: only what a default install can show.
        self.by_tl_num: dict[str, list[Row]] = defaultdict(list)
        self.all_by_tl_num: dict[str, list[Row]] = defaultdict(list)
        for row in self.rows:
            self.all_by_tl_num[row.tl_num].append(row)
            if row.is_visible_by_default:
                self.by_tl_num[row.tl_num].append(row)

    def count(self, row: Row) -> int:
        # Row.tl_num is already lowercased, so (hanzi, tl_num) is the model word.
        return self.walker_model.counts.get((row.hanzi, row.tl_num), 0)

    def category(self, row: Row) -> str:
        """One gold category per single word; a multi-word excerpt is a `phrase`."""
        if "--" in row.tl:
            return "neutral"
        if len({r.hanzi for r in self.by_tl_num[row.tl_num]}) > 1:
            return "variant"
        count = self.count(row)
        return "common" if count >= COMMON_COUNT else "rare" if count else "unseen"


# ------------------------------------------------------------------- gold rows
@dataclass(frozen=True)
class Item:
    id: str
    split: str
    category: str
    ref: str
    extra_accepted: str = ""


def read_gold(path: Path = GOLD_TSV) -> list[Item]:
    with path.open(encoding="utf-8", newline="") as f:
        return [Item(**row) for row in csv.DictReader(f, delimiter="\t")]


def write_tsv(path: Path, header: list[str], rows: list[list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as f:
        f.write("\t".join(header) + "\n")
        for row in rows:
            f.write("\t".join(row) + "\n")


# --------------------------------------------------------------------- corpus
@dataclass(frozen=True)
class Unit:
    ref: str
    hanji: str
    tl: str
    article: int | None  # typing article id; None for a 教典 example sentence


@cache
def corpus_units() -> dict[str, Unit]:
    """Every 教典 example sentence and `mapped` typing-article line, by gold ref."""
    groups = js_array_to_json(EXAMPLE_SENTENCES.read_text(encoding="utf-8"))
    sentences = [sentence for group in groups for sentence in group]
    units = [Unit(f"kautian:{i}", s["hanji"], s["tailo"], None) for i, s in enumerate(sentences)]
    units += [Unit(f"typing:{a}:{i}", text, tl, a) for a, i, text, tl in typing_article_lines()]
    return {unit.ref: unit for unit in units}


def unit_words(unit: Unit, dictionary: Dictionary) -> list[Row | None]:
    """Aligned tokens → the dictionary word as written; None for anything a default install cannot show.

    No OOV subsegmenting and no romanized-word mapping: a gold excerpt is the corpus's own
    words, in its own script.
    """
    tokens = align_unit(unit.hanji, unit.tl)
    readings = [t.word for t in tokens if t.kind in ("word", "latin", "mixed")]
    numeric = iter(convert_readings(readings, "tl"))
    out: list[Row | None] = []
    for token in tokens:
        if token.kind in ("break", "digit"):
            out.append(None)
            continue
        key = key_of(next(numeric))
        row = dictionary.by_word.get((token.hanji, key)) if token.kind == "word" else None
        out.append(row if row is not None and row.is_visible_by_default else None)
    return out


def windows(words: list[Row | None]) -> list[tuple[int, int]]:
    """Every (start, end) run of 1..MAX_WORDS dictionary words with MIN..MAX_SYLLABLES syllables."""
    out = []
    for start in range(len(words)):
        syllables = 0
        for end in range(start + 1, min(start + MAX_WORDS, len(words)) + 1):
            if words[end - 1] is None:
                break
            syllables += words[end - 1].syllable_count
            if syllables > MAX_SYLLABLES:
                break
            if syllables >= MIN_SYLLABLES:
                out.append((start, end))
    return out


# --------------------------------------------------------------------- select
def other_source_sentences() -> set[str]:
    """Normalized text of every corpus sentence outside the held-out source (cross-source dedupe)."""
    return {
        normalize_sentence(unit.text) for name, units in SOURCES.items() if name != HELD_OUT_SOURCE for unit in units()
    }


def select(args: argparse.Namespace) -> int:
    rng = random.Random(SEED)
    dictionary = Dictionary()
    items: list[Item] = []

    def add(split: str, category: str, ref: str) -> None:
        items.append(Item(f"{split}-{len(items):04d}", split, category, ref))

    def corpus_items(units: list[Unit], split: str, per_unit: int, quota: dict[str, int] | None) -> None:
        taken: dict[str, int] = defaultdict(int)
        for unit in units:
            if quota is not None and all(taken[c] >= n for c, n in quota.items()):
                return
            try:
                words = unit_words(unit, dictionary)
            except AlignError:
                continue
            spans = windows(words)
            rng.shuffle(spans)
            used: list[tuple[int, int]] = []
            for start, end in spans:
                if len(used) == per_unit:
                    break
                if any(start < e and s < end for s, e in used):
                    continue
                category = "phrase" if end - start > 1 else dictionary.category(words[start])
                if quota is not None and taken[category] >= quota.get(category, 0):
                    continue
                taken[category] += 1
                used.append((start, end))
                add(split, category, f"{unit.ref}#{start}-{end}")

    units = list(corpus_units().values())
    examples = [unit for unit in units if unit.article is None]
    rng.shuffle(examples)
    single = DEV_PER_SINGLE_WORD_CATEGORY
    corpus_items(
        examples,
        "dev",
        1,
        {
            "phrase": DEV_PHRASES,
            "common": single,
            "rare": single,
            "variant": single,
            "neutral": single,
        },
    )

    seen_elsewhere = other_source_sentences()
    for split, articles in (("calib", CALIB_ARTICLES), ("final", FINAL_ARTICLES)):
        lines = [u for u in units if u.article in articles and normalize_sentence(u.hanji) not in seen_elsewhere]
        corpus_items(lines, split, WINDOWS_PER_TYPING_LINE, None)

    for words in REGRESSION_ITEMS:
        add("dev", "regression", f"dict:{words}")
    candidates = [
        r
        for r in dictionary.rows
        if r.is_visible_by_default and 2 <= r.syllable_count <= 3 and SUPPORTED_KEY.fullmatch(r.tl_num)
    ]
    rng.shuffle(candidates)
    want = dict(DICT_QUOTA)
    for row in candidates:
        category = dictionary.category(row)
        if want.get(category):
            want[category] -= 1
            add("dev", category, f"dict:{row.hanzi}/{row.tl}")

    write_tsv(
        GOLD_TSV,
        GOLD_HEADER,
        [[i.id, i.split, i.category, i.ref, i.extra_accepted] for i in items],
    )
    print(f"wrote {len(items)} items → {GOLD_TSV}")
    return 0


# -------------------------------------------------------------------- resolve
@dataclass(frozen=True)
class Resolved:
    item: Item
    rows: list[Row]  # one dictionary row per gold word

    def syllables(self) -> list[str]:
        return [s for row in self.rows for s in NUMERIC_SYLLABLE.findall(row.tl_num)]

    def inputs(self) -> dict[str, str]:
        def partial(tl_num: str) -> str:
            first, *rest = NUMERIC_SYLLABLE.findall(tl_num)
            return first + "".join(remove_tone(s) for s in rest)

        return {
            "tl_full": "".join(r.tl_num for r in self.rows),
            "tl_partial": "".join(partial(r.tl_num) for r in self.rows),
            "tl_toneless": "".join(r.tl_notone for r in self.rows),
            "tl_hyphen": "-".join(self.syllables()),
            "poj_full": "".join(r.poj_num for r in self.rows),
            "poj_toneless": "".join(r.poj_notone for r in self.rows),
            "tps_full": "".join(r.tps_num for r in self.rows),
            "tps_toneless": "".join(r.tps_notone for r in self.rows),
        }


def resolve_item(item: Item, dictionary: Dictionary) -> Resolved | None:
    kind, _, rest = item.ref.partition(":")
    if kind == "dict":
        pairs = [word.split("/", 1) for word in rest.split(WORD_SEPARATOR)]
        by_display = {(r.hanzi, r.tl.lower()): r for r in reversed(dictionary.rows)}
        return Resolved(item, [by_display[(hanji, tl.lower())] for hanji, tl in pairs])
    ref, _, span = item.ref.partition("#")
    start, end = (int(x) for x in span.split("-"))
    words = unit_words(corpus_units()[ref], dictionary)[start:end]
    if any(word is None for word in words):
        return None
    return Resolved(item, words)


def resolve_all(dictionary: Dictionary) -> tuple[list[Resolved], list[tuple[Item, str]]]:
    """The resolvable items, plus the skipped ones with the reason.

    `select` built every item from this corpus, dictionary and taigi-converter, so an item
    that no longer resolves means the environment drifted (most often a submodule that is not
    checked out): that exits instead of quietly measuring a smaller gold set.
    """
    resolved, skipped, drifted = [], [], []
    for item in read_gold():
        result = resolve_item(item, dictionary)
        if result is None:
            drifted.append(item.id)
        elif unsupported := [r.tl_num for r in result.rows if not SUPPORTED_KEY.fullmatch(r.tl_num)]:
            skipped.append((item, f"unsupported tl_num {unsupported}"))
        else:
            resolved.append(result)
    if drifted:
        raise SystemExit(
            f"{len(drifted)} gold items no longer resolve: {drifted}. Check "
            "`git submodule update --init corpus/taigi-typing taigi-converter` and `make dict`."
        )
    return resolved, skipped


def homophones(row: Row, dictionary: Dictionary) -> str:
    """Every hanji a default install has for the word's fully toned key (the gold word included)."""
    return "|".join(sorted({row.plain_hanji} | {r.plain_hanji for r in dictionary.by_tl_num[row.tl_num]}))


def resolve(args: argparse.Namespace) -> int:
    dictionary = Dictionary()
    resolved, skipped = resolve_all(dictionary)
    header = [
        "id",
        "split",
        "category",
        "hanji",
        "tl",
        "homophones",
        "extra_accepted",
        *INPUT_VARIANTS,
    ]
    rows = []
    for r in resolved:
        inputs = r.inputs()
        rows.append(
            [
                r.item.id,
                r.item.split,
                r.item.category,
                WORD_SEPARATOR.join(row.plain_hanji for row in r.rows),
                WORD_SEPARATOR.join(row.tl for row in r.rows),
                WORD_SEPARATOR.join(homophones(row, dictionary) for row in r.rows),
                r.item.extra_accepted,
                *(inputs[v] for v in INPUT_VARIANTS),
            ]
        )
    write_tsv(RESOLVED_TSV, header, rows)
    print(f"resolved {len(rows)} items → {RESOLVED_TSV}; skipped {len(skipped)}")
    # Each gold word typed alone: walker_gold.rs ranks it in the candidate list (E1 P5b).
    word_rows = []
    for r in resolved:
        for index, row in enumerate(r.rows):
            inputs = Resolved(r.item, [row]).inputs()
            word_rows.append(
                [r.item.id, r.item.split, r.item.category, str(index), row.plain_hanji, *(inputs[v] for v in INPUT_VARIANTS)]
            )
    write_tsv(WORD_INPUTS_TSV, ["id", "split", "category", "word", "hanji", *INPUT_VARIANTS], word_rows)
    print(f"{len(word_rows)} gold words → {WORD_INPUTS_TSV}")
    for item, reason in skipped:
        print(f"  skipped {item.id}: {reason}")
    # Every fully toned key with competing hanji: walker_gold.rs asks the engine which one it picks.
    keys = sorted(k for k, rs in dictionary.all_by_tl_num.items() if len({r.plain_hanji for r in rs}) > 1)
    write_tsv(
        EDGE_KEYS_TSV,
        ["key", "homophones"],
        [[k, "|".join(sorted({r.plain_hanji for r in dictionary.all_by_tl_num[k]}))] for k in keys],
    )
    print(f"{len(keys)} multi-homophone keys → {EDGE_KEYS_TSV}")
    return 0


# ------------------------------------------------------------------ simulator
def read_edge_picks(bitmask: str = "default") -> dict[str, str]:
    """key → the hanji the engine picks for that edge (walker_gold.rs `walker_edge_picks`)."""
    if not EDGE_PICKS_TSV.exists():
        raise SystemExit(f"{EDGE_PICKS_TSV} missing: run `resolve`, then the walker_gold.rs harness.")
    with EDGE_PICKS_TSV.open(encoding="utf-8", newline="") as f:
        return {row["key"]: row["pick"] for row in csv.DictReader(f, delimiter="\t") if row["sources"] == bitmask}


# The denominator the engine priced `frequency` against until E1 P3 retired it
# (`CORPUS_TOTAL_FREQ` in engine/composing/src/lattice/cost.rs, last value); the hermetic
# engine fixtures freeze the same value (engine/test-support/src/tkdb.rs).
PRE_P3_CORPUS_TOTAL_FREQ = 13_056_588.0


class Simulator:
    """The walker over a fully toned syllable sequence: dictionary edges only (no OOV, no user data).

    `pre-p3` = the edge cost before E1 P3 (homophone max frequency over the frozen corpus total);
    `a2` = plan §3 / D2 / D3, the engine since P3 (min `walker_cost` over the homophones,
    `build.walker_lm` at the given α). Both price the length terms on the picked word,
    as `continuous.rs` does; the pick is the engine's own (`read_edge_picks`).
    """

    def __init__(self, dictionary: Dictionary, picks: dict[str, str]) -> None:
        self.dictionary = dictionary
        self.picks = picks

    def pick(self, key: str, rows: list[Row]) -> Row:
        hanji = self.picks.get(key)
        return next((r for r in rows if r.plain_hanji == hanji), rows[0])

    def edge_cost(self, rows: list[Row], pick: Row, model: str, alpha: float) -> float:
        if model == "pre-p3":
            base = math.log(PRE_P3_CORPUS_TOTAL_FREQ / (1 + max(r.frequency for r in rows)))
        else:
            # The quantised `walker_cost` dictionary.bin v4 stores, at this α.
            walker_model = replace(self.dictionary.walker_model, alpha=alpha)
            base = min(walker_model.cost((r.hanzi, r.tl_num)) for r in rows) / MILLI_NATS
        return base / len(pick.tl_notone) ** 0.2 * pick.syllable_count**0.2

    def best_path(self, syllables: list[str], model: str, alpha: float) -> list[Row] | None:
        n = len(syllables)
        best: list[tuple[float, int, Row | None] | None] = [None] * (n + 1)
        best[0] = (0.0, -1, None)
        for end in range(1, n + 1):
            for start in range(max(0, end - MAX_EDGE_SYLLABLES), end):
                key = "".join(syllables[start:end])
                rows = self.dictionary.by_tl_num.get(key)
                if best[start] is None or not rows:
                    continue
                pick = self.pick(key, rows)
                cost = best[start][0] + self.edge_cost(rows, pick, model, alpha)
                if best[end] is None or cost < best[end][0]:
                    best[end] = (cost, start, pick)
        if best[n] is None:
            return None
        path, end = [], n
        while end > 0:
            path.append(best[end][2])
            end = best[end][1]
        return path[::-1]


def path_text(path: list[Row] | None) -> str:
    return " ".join(r.plain_hanji for r in path) if path else "∅"


def path_shape(path: list[Row] | None) -> tuple[str, list[int]]:
    """(hanji, displayed syllables per word) — what walker_gold.rs records for the engine's slot 0."""
    return ("".join(r.plain_hanji for r in path), [r.display_syllables for r in path]) if path else ("", [])


def simulate(args: argparse.Namespace) -> int:
    dictionary = Dictionary()
    simulator = Simulator(dictionary, read_edge_picks())
    resolved = [r for r in resolve_all(dictionary)[0] if r.item.split in args.splits]
    models = [("pre-p3", 0.0), *((f"a2@{a}", a) for a in args.alphas)]
    compared = models[1][0]
    # stratum → model → [exact, segmented]; `n` per stratum alongside.
    totals: dict[str, dict[str, list[int]]] = defaultdict(lambda: defaultdict(lambda: [0, 0]))
    sizes: dict[str, int] = defaultdict(int)
    changes = []
    for r in resolved:
        gold = [row.plain_hanji for row in r.rows]
        strata = (f"{r.item.split}/all", f"{r.item.split}/{r.item.category}")
        for stratum in strata:
            sizes[stratum] += 1
        results = {name: simulator.best_path(r.syllables(), name.split("@")[0], alpha) for name, alpha in models}
        for name, path in results.items():
            is_exact = path is not None and "".join(p.plain_hanji for p in path) == "".join(gold)
            # Same words (fully toned keys) in the same order, whatever the orthography.
            is_segmented = path is not None and [p.tl_num for p in path] == [g.tl_num for g in r.rows]
            for stratum in strata:
                totals[stratum][name][0] += is_exact
                totals[stratum][name][1] += is_segmented
        if r.item.split != "final" and path_text(results["pre-p3"]) != path_text(results[compared]):
            changes.append(
                (
                    r.item.id,
                    r.item.category,
                    " ".join(gold),
                    path_text(results["pre-p3"]),
                    path_text(results[compared]),
                )
            )
    print("stratum\tn\t" + "\t".join(f"{name} exact/seg" for name, _ in models))
    for stratum in sorted(totals):
        n = sizes[stratum]
        print(
            f"{stratum}\t{n}\t"
            + "\t".join(f"{totals[stratum][m][0] / n:.0%}/{totals[stratum][m][1] / n:.0%}" for m, _ in models)
        )
    if args.engine_slot0:
        report_engine_agreement(args.engine_slot0, resolved, simulator)
    print(f"\n{len(changes)} items whose slot 0 changes (pre-p3 → {compared}; final split never listed):")
    for change in changes:
        print("\t".join(change))
    return 0


def report_engine_agreement(path: Path, resolved: list[Resolved], simulator: Simulator) -> None:
    """How often the simulator's A2 slot 0 at the shipped α (hanji + displayed word lengths) equals the
    engine's on tl_full."""
    with path.open(encoding="utf-8", newline="") as f:
        engine = {
            row["id"]: (
                row["slot0"],
                [int(n) for n in row["word_syllables"].split(",") if n],
            )
            for row in csv.DictReader(f, delimiter="\t")
            if row["variant"] == "tl_full"
        }
    agree, disagreements = 0, []
    for r in resolved:
        simulated = path_shape(simulator.best_path(r.syllables(), "a2", WALKER_ALPHA))
        if simulated == engine.get(r.item.id):
            agree += 1
        else:
            disagreements.append(f"{r.item.id}\tengine={engine.get(r.item.id)}\tsimulator={simulated}")
    print(
        f"\nsimulator vs engine (a2@{WALKER_ALPHA}, tl_full, hanji + word lengths): {agree}/{len(resolved)} agree"
    )
    print("\n".join(disagreements[:20]))


def exposure(args: argparse.Namespace) -> int:
    """Every fully toned 2–3 syllable key: does slot 0 keep one of the key's own words as one edge?"""
    dictionary = Dictionary()
    simulator = Simulator(dictionary, read_edge_picks())
    keys = sorted(k for k in dictionary.by_tl_num if 2 <= len(NUMERIC_SYLLABLE.findall(k)) <= 3)
    losers = {"pre-p3": 0, "a2": 0}
    seen_losers = {"pre-p3": 0, "a2": 0}
    flips = defaultdict(list)
    for key in keys:
        syllables = NUMERIC_SYLLABLE.findall(key)
        is_seen = any(dictionary.count(r) for r in dictionary.by_tl_num[key])
        paths = {model: simulator.best_path(syllables, model, args.alpha) for model in losers}
        keeps_word = {model: path is not None and len(path) == 1 for model, path in paths.items()}
        for model, kept in keeps_word.items():
            losers[model] += not kept
            seen_losers[model] += not kept and is_seen
        if keeps_word["pre-p3"] != keeps_word["a2"]:
            word, split = ("pre-p3", "a2") if keeps_word["pre-p3"] else ("a2", "pre-p3")
            direction = "word→split" if keeps_word["pre-p3"] else "split→word"
            flips[direction].append(f"{key}\t{path_text(paths[word])}\t{path_text(paths[split])}")
    print(f"keys: {len(keys)}  alpha={args.alpha}")
    for model, lost in losers.items():
        print(f"{model}: whole-key word loses slot 0 on {lost} keys ({seen_losers[model]} with a corpus count)")
    for direction, rows in flips.items():
        print(f"\n{direction}: {len(rows)}")
        random.Random(SEED).shuffle(rows)
        print("\n".join(rows[: args.sample]))
    return 0


def hanji_counts(rows: list[Row], dictionary: Dictionary) -> dict[str, int]:
    """Corpus count per displayed hanji of one key. Rows that share a model word (`tshut-lâi` /
    `tshut--lâi`) share one count, so each model word is added once."""
    counts: dict[str, int] = defaultdict(int)
    for hanzi, tl_num in {(r.hanzi, r.tl_num) for r in rows}:
        counts[hanzi.replace("-", "")] += dictionary.walker_model.counts.get((hanzi, tl_num), 0)
    return counts


def d3(args: argparse.Namespace) -> int:
    """Plan D3: how often the engine's edge pick is not the word the corpus writes most for that key."""
    dictionary = Dictionary()
    for sources in ("default", "all"):
        picks = read_edge_picks(sources)
        differs = rare_pick = 0
        examples = []
        # The winner is chosen among the rows the engine could pick under these sources.
        visible = dictionary.by_tl_num if sources == "default" else dictionary.all_by_tl_num
        for key, pick_hanji in picks.items():
            rows = visible.get(key)
            if not rows:
                continue
            counts = hanji_counts(rows, dictionary)
            winner = max(sorted(counts), key=lambda h: counts[h])
            if not pick_hanji or pick_hanji == winner or counts[winner] == 0:
                continue
            differs += 1
            if counts[pick_hanji] == 0 and counts[winner] >= 10:
                rare_pick += 1
                examples.append(
                    (
                        counts[winner],
                        f"{key}\t{pick_hanji} ({counts[pick_hanji]})\t{winner} ({counts[winner]})",
                    )
                )
        print(
            f"{sources}: {len(picks)} keys picked; pick ≠ corpus winner on {differs}; pick unseen while the winner has ≥ 10 on {rare_pick}"
        )
        print("\n".join(text for _, text in sorted(examples, reverse=True)[: args.sample]))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("select").set_defaults(run=select)
    sub.add_parser("resolve").set_defaults(run=resolve)
    simulate_parser = sub.add_parser("simulate")
    # `final` is read once, at the end of P4 (plan §5) — pass it explicitly then.
    simulate_parser.add_argument(
        "--splits",
        nargs="+",
        default=["dev", "calib"],
        choices=["dev", "calib", "final"],
    )
    simulate_parser.add_argument("--alphas", nargs="+", type=float, default=[0.5, 2.0, 10.0, 50.0])
    simulate_parser.add_argument(
        "--engine-slot0",
        type=Path,
        help="GOLD_SLOT0_OUT of walker_gold.rs: simulator/engine agreement",
    )
    simulate_parser.set_defaults(run=simulate)
    exposure_parser = sub.add_parser("exposure")
    exposure_parser.add_argument("--alpha", type=float, default=0.5)
    exposure_parser.add_argument("--sample", type=int, default=30)
    exposure_parser.set_defaults(run=exposure)
    d3_parser = sub.add_parser("d3")
    d3_parser.add_argument("--sample", type=int, default=15)
    d3_parser.set_defaults(run=d3)
    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
