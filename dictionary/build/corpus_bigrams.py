"""Count cross-word bigrams (and unigrams) from the aligned Taiwanese corpora.

Bigram LM roadmap phase P2 (docs/architecture/bigram-lm-roadmap.md § D1-D2;
method + numbers: docs/reports/2026-09-28-bigram-corpus-spike.md).

Inputs (none is a build input of `build.sh`; this runs by hand):
  corpus/taigi-corpus/data/normalized/<source>.jsonl   opt-in submodule
  corpus/taigi-typing/articles.js                       `mapped` articles only
  output/dictionary.csv                                 token whitelist (the
                                                        records dictionary.bin carries)
  shared/data/romanized_readings.tsv                    reading → hanji for words
                                                        written in romanization
Outputs (committed, consumed by the association.bin v2 writer in P3):
  shared/data/word_bigrams.tsv    prev_hanji prev_tl next_hanji next_tl count <one column per source>
  shared/data/word_unigrams.tsv   hanji tl count <one column per source>

A token is a dictionary word: (hanji, tl_num). Per source the text side and
the romanization side are aligned syllable by syllable; anything that is not
a dictionary word breaks the pair chain. Counts are raw per source (one
column per source, `count` = their sum); any per-source weighting is P3's
decision. Sentence-end punctuation breaks the chain like any other break.

The corpus is counted twice (E1 P2b, docs/architecture/unified-word-frequency-roadmap.md
§6). Pass 1 counts only the tokens that are dictionary words as written; an
out-of-vocabulary word breaks the chain. Pass 2 splits such a word into
dictionary words — fewest pieces, then the lowest summed walker cost under a
model frozen from pass 1 — so `DictionaryRecord.frequency` does not steer the
counts.

Usage (from dictionary/):
  python3 -m build.corpus_bigrams              # all sources
  python3 -m build.corpus_bigrams --sources moe_kautian sinpak_900leku
"""

from __future__ import annotations

import argparse
import csv
import json
import logging
import os
import re
import sys
import unicodedata
from collections import Counter, defaultdict
from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass, field
from pathlib import Path

from build.common import BASE_DIR, BIGRAMS_TSV, MERGED_CSV, SHARED_DATA_DIR, UNIGRAMS_TSV
from build.dictionary_records import load_dictionary_records
from build.walker_lm import WalkerModel, model_from_counts, model_word
from common.abbrev import remove_diacritics
from common.cjk import is_cjk
from common.logging_utils import setup_logging
from common.taigi_bridge import convert_poj_to_tl_strict, to_tone_number_ascii

REPO_DIR = BASE_DIR.parent
CORPUS_DIR = REPO_DIR / "corpus" / "taigi-corpus" / "data" / "normalized"
TAIGI_TYPING_ARTICLES = REPO_DIR / "corpus" / "taigi-typing" / "articles.js"
ROMANIZED_READINGS_TSV = SHARED_DATA_DIR / "romanized_readings.tsv"

Word = tuple[str, str]  # (hanji, tl_num)
# A pair must be seen this often to be written; P3 applies the per-key top-K.
MIN_PAIR_COUNT = 2
# Identical sentences beyond this many per source are boilerplate (news
# bylines, scripture headers) and are not counted again.
SENTENCE_CAP = 3
# Lidstone α of the pass-1 tie-break model. Fixed on its own: tuning the
# walker's α (`build.common.WALKER_ALPHA`, E1 P4) must not regenerate the corpus.
SUBSEGMENT_ALPHA = 10.0

SENTENCE_END = "。！？!?"
PUNCTUATION = SENTENCE_END + "，,、；;：:「」『』（）()《》〈〉…—“”\"'‘’‧·．."
ROMAN_LETTER = re.compile(r"[A-Za-zÀ-ɏḀ-ỿⁿ]")
NON_KEY_CHARS = re.compile(r"[^a-z0-9]")
HYPHENS = re.compile(r"-+")
# Numeric POJ writes the nasal as a capital N before the tone digit (chiaN2);
# taigi-converter only knows `nn`.
POJ_NASAL_N = re.compile(r"N(?=[0-9]?(?:$|[-\s]))")
HORIZONTAL_WHITESPACE = re.compile(r"[ \t　]+")


# --------------------------------------------------------------------- tokens
def key_of(numeric_tl: str) -> str:
    """Numeric ASCII TL word → dictionary `tl_num` key (letters + digits only).

    taigi-converter writes every tone digit explicitly, so this equals
    `common.romanization.to_numeric_tone(word, ascii_only=True)` (pinned by a test).
    """
    return NON_KEY_CHARS.sub("", numeric_tl.lower())


def syllables_of(word: str) -> list[str]:
    return [syllable for syllable in HYPHENS.split(word) if syllable]


def letters_only(text: str) -> str:
    """Case- and tone-insensitive shape of a romanized syllable, for run matching."""
    text = remove_diacritics(text.replace("ⁿ", "n").replace("ᴺ", "n")).lower()
    return "".join(c for c in text if c.isascii() and c.isalpha())


def normalize_sentence(text: str) -> str:
    return HORIZONTAL_WHITESPACE.sub(" ", unicodedata.normalize("NFC", text)).strip()


def hanji_with_khinsiann(chars: list[str], word: str) -> str:
    """Dictionary hanzi carries `--` where the TL has it (講--的): mirror that."""
    if "--" not in word:
        return "".join(chars)
    out: list[str] = []
    taken = 0
    for part in re.split(r"(--)", word):
        if part == "--":
            out.append("--")
        else:
            count = len(syllables_of(part))
            out.append("".join(chars[taken : taken + count]))
            taken += count
    return "".join(out)


# ------------------------------------------------------------------ alignment
@dataclass(frozen=True)
class Token:
    """One aligned word, or a chain break.

    kind: 'word' (all hanji), 'latin' (all romanized), 'mixed', 'digit' or
    'break' (clause punctuation, or sentence punctuation on the text side).
    """

    kind: str
    word: str | None = None  # romanization as written
    hanji: str | None = None


def tl_tokens(line: str) -> list[Token]:
    """Romanization side → words with khin-siann attached, plus break markers."""
    out: list[Token] = []
    for raw in line.split():
        stripped = raw.lstrip(PUNCTUATION)
        core = stripped.rstrip(PUNCTUATION)
        if len(stripped) < len(raw):
            out.append(Token("break"))
        if core:
            if core.startswith("--") and out and out[-1].kind == "latin":
                out[-1] = Token("latin", out[-1].word + core)
            elif ROMAN_LETTER.search(core):
                out.append(Token("latin", core))
            else:
                out.append(Token("digit"))
        if len(core) < len(stripped):
            out.append(Token("break"))
    return out


def hanlo_units(line: str) -> list[tuple[str, str]]:
    """Text side → ('c', hanji) | ('r', romanized syllable) | ('d', digits) | ('e', sentence end).

    A romanized run keeps its hyphens and is then split into syllables, so a
    word may mix romanized and Han syllables (`pháiⁿ人`, `to3-tng2來`).
    """
    units: list[tuple[str, str]] = []
    run, kind = "", ""

    def flush() -> None:
        nonlocal run, kind
        if run and kind == "r":
            units.extend(("r", syllable) for syllable in syllables_of(run))
        elif run:
            units.append((kind, run))
        run, kind = "", ""

    for ch in line:
        if is_cjk(ch):
            flush()
            units.append(("c", ch))
        elif ch in SENTENCE_END:
            flush()
            units.append(("e", ch))
        elif ch == "-" and kind == "r":
            run += ch
        elif ch.isspace() or unicodedata.category(ch)[0] == "P":
            flush()
        elif ch.isdigit() and kind != "r":
            kind = "d"
            run += ch
        else:
            if kind == "d":
                flush()
            kind = "r"
            run += ch
    flush()
    return units


class AlignError(ValueError):
    """The two sides of a unit do not line up; the unit is dropped."""


def align_unit(text: str, romanization: str) -> list[Token]:
    """Pair each romanized word with its hanji (or romanized / mixed form).

    Sentence ends come from the text side (`。！？`), which every source writes,
    and break the chain like the romanization's own clause punctuation.
    """
    units = hanlo_units(text)
    position = 0
    out: list[Token] = []

    def take_sentence_ends() -> None:
        nonlocal position
        while position < len(units) and units[position][0] == "e":
            out.append(Token("break"))
            position += 1

    for token in tl_tokens(romanization):
        take_sentence_ends()
        if token.kind == "break":
            if out and out[-1].kind != "break":
                out.append(token)
            continue
        if token.kind == "digit":
            if position < len(units) and units[position][0] == "d":
                position += 1
            out.append(token)
            continue
        syllables = syllables_of(token.word)
        taken = units[position : position + len(syllables)]
        if len(taken) < len(syllables) or any(kind in "de" for kind, _ in taken):
            raise AlignError("char-count")
        for (kind, unit), syllable in zip(taken, syllables, strict=True):
            if kind == "r" and letters_only(unit) != letters_only(syllable):
                raise AlignError("latin-mismatch")
        position += len(syllables)
        kinds = {kind for kind, _ in taken}
        if kinds == {"c"}:
            out.append(Token("word", token.word, hanji_with_khinsiann([unit for _, unit in taken], token.word)))
        elif kinds == {"r"}:
            out.append(Token("latin", token.word))
        else:
            out.append(Token("mixed", token.word, "".join(unit for _, unit in taken)))
    take_sentence_ends()
    if position != len(units):
        raise AlignError("hanlo-leftover")
    return out


def split_sentences(text: str) -> list[str]:
    parts = re.split(r"(?<=[" + re.escape(SENTENCE_END) + r"])", text)
    return [part for part in (piece.strip() for piece in parts) if part]


# ----------------------------------------------------------------- dictionary
@dataclass(frozen=True)
class Lexicon:
    tl_by_word: dict[Word, str]  # display TL (diacritics) per dictionary word
    hanji_by_romanized_reading: dict[str, str]  # tl_num → hanji, for words written in romanization


def load_lexicon(dictionary_csv: Path = MERGED_CSV, readings_tsv: Path = ROMANIZED_READINGS_TSV) -> Lexicon:
    """The words dictionary.bin carries (same filter as the binary writers), plus the reading table."""
    tl_by_word: dict[Word, str] = {}
    for record in load_dictionary_records(dictionary_csv):
        if record.hanzi is None or not record.tl_num:
            continue
        tl_by_word.setdefault(model_word(record), record.tl)
    with readings_tsv.open(encoding="utf-8", newline="") as f:
        readings = {row["tl_num"]: row["hanji"] for row in csv.DictReader(f, delimiter="\t")}
    unknown = [f"{key}→{hanji}" for key, hanji in readings.items() if (hanji, key) not in tl_by_word]
    if unknown:
        raise ValueError(f"{readings_tsv.name}: not dictionary words: {', '.join(unknown)}")
    return Lexicon(tl_by_word, readings)


def subsegment(hanji: str, syllable_keys: list[str], lexicon: Lexicon, model: WalkerModel) -> list[Word] | None:
    """Split an out-of-vocabulary word into dictionary words: fewest pieces
    (greedy longest-match gave `新聞報 → 導`), then the lowest sum of the
    pieces' quantised milli-nat costs under the pass-1 model. On an exact tie
    the first split found stays: the longest last piece, its prefix decided
    the same way."""
    chars = list(hanji)
    length = len(chars)
    if length != len(syllable_keys) or length < 2:
        return None
    # best[end] = (pieces, summed cost, start of the last piece)
    best: list[tuple[int, int, int] | None] = [None] * (length + 1)
    best[0] = (0, 0, -1)
    for end in range(1, length + 1):
        for start in range(end):
            if best[start] is None:
                continue
            word = ("".join(chars[start:end]), "".join(syllable_keys[start:end]))
            if word not in lexicon.tl_by_word:
                continue
            candidate = (best[start][0] + 1, best[start][1] + model.cost(word), start)
            if best[end] is None or candidate[:2] < best[end][:2]:
                best[end] = candidate
    if best[length] is None:
        return None
    pieces: list[Word] = []
    end = length
    while end > 0:
        start = best[end][2]
        pieces.append(("".join(chars[start:end]), "".join(syllable_keys[start:end])))
        end = start
    return pieces[::-1]


# -------------------------------------------------------------------- sources
@dataclass(frozen=True)
class Unit:
    text: str
    romanization: str
    system: str  # "tl" | "poj"


def _jsonl(name: str) -> Iterator[dict]:
    path = CORPUS_DIR / f"{name}.jsonl"
    if not path.exists():
        raise FileNotFoundError(
            f"{path} missing. The corpus is an opt-in submodule: run "
            "`git submodule update --init --checkout corpus/taigi-corpus` from the repository root."
        )
    with path.open(encoding="utf-8") as f:
        for line in f:
            yield json.loads(line)


def paired_lines(text: str, romanization: str, origin: str) -> Iterator[tuple[str, str]]:
    """Line N of the text ↔ line N of the romanization; a count mismatch is a corpus bug."""
    text_lines, reading_lines = text.split("\n"), romanization.split("\n")
    if len(text_lines) != len(reading_lines):
        raise ValueError(
            f"{origin}: {len(text_lines)} text lines vs {len(reading_lines)} romanization lines; "
            "rebuild the source in taigi-corpus before counting"
        )
    return zip(text_lines, reading_lines, strict=True)


def units_jsonl(name: str, system: str, tag: str | None = None) -> Callable[[], Iterator[Unit]]:
    """Line-aligned `text` / `parallel_poj`, each line split into sentences when
    both sides agree on the count, else aligned whole."""

    def units() -> Iterator[Unit]:
        for doc in _jsonl(name):
            if tag and tag not in doc["metadata"]["tags"]:
                continue
            for text, reading in paired_lines(doc["text"], doc["metadata"]["parallel_poj"], doc["id"]):
                text_sentences, reading_sentences = split_sentences(text), split_sentences(reading)
                if len(text_sentences) == len(reading_sentences):
                    for sentence, sentence_reading in zip(text_sentences, reading_sentences, strict=True):
                        yield Unit(sentence, sentence_reading, system)
                else:
                    yield Unit(text, reading, system)

    return units


def units_moe_kautian() -> Iterator[Unit]:
    example = re.compile(r"^例:(.*?)（(.*?)）")
    for doc in _jsonl("moe_kautian"):
        for line in doc["text"].split("\n"):
            match = example.match(line)
            if match:
                yield Unit(match.group(1), match.group(2), "tl")


_JS_TOKEN = re.compile(
    r"""(?P<ws>\s+)|(?P<comment>//[^\n]*)|(?P<backtick>`[^`]*`)|(?P<string>"(?:[^"\\]|\\.)*")
      |(?P<word>[A-Za-z_][A-Za-z0-9_]*|-?\d+(?:\.\d+)?)|(?P<punct>[\[\]{}:,])|(?P<end>;)""",
    re.VERBOSE,
)


def js_array_to_json(source: str) -> list:
    """Parse `const name = [ ... ];` — the object-literal subset the taigi-typing data files use
    (unquoted keys, backtick strings, trailing commas, `//` comments); anything after the
    statement's `;` is ignored, anything else unsupported raises."""
    body = source[source.index("=") + 1 :]
    out: list[str] = []
    pos = 0
    while pos < len(body):
        match = _JS_TOKEN.match(body, pos)
        if not match:
            raise ValueError(f"js_array_to_json: unsupported syntax at offset {pos}: {body[pos : pos + 40]!r}")
        pos = match.end()
        kind, text = match.lastgroup, match.group()
        if kind == "end":
            break
        if kind in ("ws", "comment"):
            continue
        if kind == "backtick":
            if "${" in text:
                raise ValueError("js_array_to_json: template interpolation is not supported")
            out.append(json.dumps(text[1:-1], ensure_ascii=False))
        elif kind == "word" and not text[0].isdigit() and text[0] != "-" and text not in ("true", "false", "null"):
            out.append(json.dumps(text))
        elif kind == "punct" and text in "]}" and out and out[-1] == ",":
            out[-1] = text
        else:
            out.append(text)
    return json.loads("".join(out))


def typing_article_lines() -> Iterator[tuple[int, int, str, str]]:
    """(article id, line index, text, TL) per line of a `mapped` taigi-typing article."""
    if not TAIGI_TYPING_ARTICLES.exists():
        raise FileNotFoundError(
            f"{TAIGI_TYPING_ARTICLES} missing: run `git submodule update --init corpus/taigi-typing`."
        )
    for article in js_array_to_json(TAIGI_TYPING_ARTICLES.read_text(encoding="utf-8")):
        if article.get("type") != "mapped":
            continue
        origin = f"articles.js#{article['id']}"
        lines = paired_lines(article["hanji"].strip(), article["tailo"].strip(), origin)
        for index, (text, tl) in enumerate(lines):
            yield article["id"], index, text, tl


def units_taigi_typing() -> Iterator[Unit]:
    for _, _, text, tl in typing_article_lines():
        yield Unit(text, tl, "tl")


SOURCES: dict[str, Callable[[], Iterator[Unit]]] = {
    "icorpus_hanji": units_jsonl("icorpus_hanji", "tl"),
    "moe_kautian": units_moe_kautian,
    "sinpak_900leku": units_jsonl("sinpak_900leku", "tl"),
    "kok4hau7": units_jsonl("kok4hau7", "tl"),
    "taigi_bible_nt": units_jsonl("taigi_bible_nt", "poj"),
    "kipsupin_2009": units_jsonl("kipsupin_2009", "poj"),
    "nmtl_dadwt": units_jsonl("nmtl_dadwt", "poj"),
    "khinhoan_pojbh": units_jsonl("khinhoan_pojbh", "poj", tag="parallel-status:aligned"),
    "taigi_typing": units_taigi_typing,
}


# ------------------------------------------------------------------- counting
@dataclass
class Counts:
    bigrams: dict[tuple[Word, Word], Counter] = field(default_factory=lambda: defaultdict(Counter))
    unigrams: dict[Word, Counter] = field(default_factory=lambda: defaultdict(Counter))
    stats: dict[str, Counter] = field(default_factory=lambda: defaultdict(Counter))


def classify(
    tokens: list[Token], numeric_words: list[str], lexicon: Lexicon, stat: Counter, model: WalkerModel | None
) -> list[Word | None]:
    """Aligned tokens → dictionary words; `None` breaks the chain.

    `model` is the pass-1 tie-break model; `None` (pass 1) leaves every
    out-of-vocabulary word unsplit.
    """
    items: list[Word | None] = []
    remaining = iter(numeric_words)
    for token in tokens:
        if token.kind in ("break", "digit"):
            items.append(None)
            continue
        numeric_word = next(remaining)
        key = key_of(numeric_word)
        if token.kind == "latin":
            hanji = lexicon.hanji_by_romanized_reading.get(key)
            stat["tok:romanized-mapped" if hanji else "tok:romanized-oov"] += 1
            items.append((hanji, key) if hanji else None)
            continue
        if token.kind == "mixed":
            stat["tok:mixed-oov"] += 1
            items.append(None)
            continue
        word = (token.hanji, key)
        if word in lexicon.tl_by_word:
            stat["tok:in-vocab"] += 1
            items.append(word)
            continue
        pieces = None
        if model is not None and "--" not in token.word:
            keys = [key_of(syllable) for syllable in syllables_of(numeric_word)]
            pieces = subsegment(token.hanji, keys, lexicon, model)
        if pieces:
            stat["tok:subsegmented"] += 1
            stat["tok:in-vocab"] += len(pieces)
            items.extend(pieces)
        else:
            stat["tok:oov"] += 1
            items.append(None)
    return items


def convert_readings(readings: list[str], system: str) -> list[str]:
    """Romanized words → numeric ASCII TL through taigi-converter, one call per unit."""
    if not readings:
        return []
    if system == "tl":
        tl = " ".join(readings)
    else:
        tl = convert_poj_to_tl_strict(" ".join(POJ_NASAL_N.sub("nn", reading) for reading in readings))
    numeric_words = to_tone_number_ascii(tl).split(" ")
    if len(numeric_words) != len(readings):
        raise RuntimeError(f"taigi-converter changed the word count for {readings!r}")
    return numeric_words


def count_source(name: str, units: Iterable[Unit], lexicon: Lexicon, counts: Counts, model: WalkerModel | None) -> None:
    stat = counts.stats[name]
    seen: Counter = Counter()
    for unit in units:
        stat["units"] += 1
        fingerprint = (normalize_sentence(unit.text), normalize_sentence(unit.romanization))
        seen[fingerprint] += 1
        if seen[fingerprint] > SENTENCE_CAP:
            stat["units:capped"] += 1
            continue
        try:
            tokens = align_unit(unit.text, unit.romanization)
        except AlignError as err:
            stat[f"fail:{err}"] += 1
            continue
        stat["aligned"] += 1
        readings = [token.word for token in tokens if token.kind in ("word", "latin", "mixed")]
        previous: Word | None = None
        for item in classify(tokens, convert_readings(readings, unit.system), lexicon, stat, model):
            if item is None:
                previous = None
                continue
            counts.unigrams[item][name] += 1
            if previous is not None:
                counts.bigrams[(previous, item)][name] += 1
            previous = item


def count_corpus(source_names: Iterable[str], lexicon: Lexicon, model: WalkerModel | None) -> Counts:
    counts = Counts()
    for name in source_names:
        count_source(name, SOURCES[name](), lexicon, counts, model)
    return counts


def tiebreak_model(pass_one: Counts, lexicon: Lexicon) -> WalkerModel:
    """Pass-1 unigrams → the frozen subsegment tie-break model (V = the lexicon's words).

    Splits compared by cost have the same piece count, so N and V cancel;
    only α and the counts decide.
    """
    return model_from_counts(pass_one.unigrams, len(lexicon.tl_by_word), SUBSEGMENT_ALPHA)


# --------------------------------------------------------------------- output
def write_outputs(
    counts: Counts,
    lexicon: Lexicon,
    bigrams_path: Path = BIGRAMS_TSV,
    unigrams_path: Path = UNIGRAMS_TSV,
    source_names: Iterable[str] = SOURCES,
) -> tuple[int, int]:
    """Sorted, byte-stable TSVs: total count, then one raw-count column per source."""
    source_columns = sorted(source_names)

    def per_source(sources: Counter) -> list[int]:
        return [sources.get(name, 0) for name in source_columns]

    bigram_rows = [
        [
            previous[0],
            lexicon.tl_by_word[previous],
            following[0],
            lexicon.tl_by_word[following],
            sum(sources.values()),
            *per_source(sources),
        ]
        for (previous, following), sources in counts.bigrams.items()
        if sum(sources.values()) >= MIN_PAIR_COUNT
    ]
    bigram_rows.sort(key=lambda row: (row[0], row[1], -row[4], row[2], row[3]))
    unigram_rows = [
        [word[0], lexicon.tl_by_word[word], sum(sources.values()), *per_source(sources)]
        for word, sources in counts.unigrams.items()
    ]
    unigram_rows.sort(key=lambda row: (row[0], row[1], -row[2]))
    # Both files are staged first and swapped in together, so a failure never
    # leaves bigrams and unigrams from different runs side by side.
    staged = [
        _stage_tsv(
            bigrams_path, ["prev_hanji", "prev_tl", "next_hanji", "next_tl", "count", *source_columns], bigram_rows
        ),
        _stage_tsv(unigrams_path, ["hanji", "tl", "count", *source_columns], unigram_rows),
    ]
    for temp_path, final_path in staged:
        os.replace(temp_path, final_path)
    return len(bigram_rows), len(unigram_rows)


def _stage_tsv(path: Path, header: list[str], rows: Iterable[list]) -> tuple[Path, Path]:
    temp_path = path.with_suffix(path.suffix + ".tmp")
    with temp_path.open("w", encoding="utf-8", newline="\n") as f:
        f.write("\t".join(header) + "\n")
        for row in rows:
            f.write("\t".join(str(cell) for cell in row) + "\n")
    return temp_path, path


# ----------------------------------------------------------------------- main
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--sources", nargs="+", choices=sorted(SOURCES), default=sorted(SOURCES))
    args = parser.parse_args(argv)
    log: logging.Logger = setup_logging("corpus_bigrams")
    lexicon = load_lexicon()
    log.info(
        "Lexicon: %d (hanzi, tl) words, %d romanized readings",
        len(lexicon.tl_by_word),
        len(lexicon.hanji_by_romanized_reading),
    )
    model = tiebreak_model(count_corpus(args.sources, lexicon, None), lexicon)
    log.info("Pass 1 tie-break model: N=%d V=%d alpha=%r", model.tokens, model.vocabulary, model.alpha)
    counts = count_corpus(args.sources, lexicon, model)
    for name in args.sources:
        log.info("%s: %s", name, dict(sorted(counts.stats[name].items())))
    bigram_count, unigram_count = write_outputs(counts, lexicon)
    log.info("Wrote %d bigram rows → %s", bigram_count, BIGRAMS_TSV)
    log.info("Wrote %d unigram rows → %s", unigram_count, UNIGRAMS_TSV)
    return 0


if __name__ == "__main__":
    sys.exit(main())
