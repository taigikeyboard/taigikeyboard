#!/usr/bin/env python3
# Generate continuous-input dogfood test data from the dictionary.
#
# Prints an ASCII table of test groups for MANUAL on-device testing. Every
# group is RANDOMLY generated from real dictionary words (different each run)
# and shown in ALL THREE modes (TL / POJ / TPS) plus a 漢字 column. Each row
# targets ONE edge-case aspect plain random concatenation would miss (lone
# initials, explicit tone, ir-after-sibilant, stop/nasal codas, oo/er/ee finals,
# 連字 / 輕聲 separators, longest-match prefix, continuous segmentation, …).
# Aspect is documented per PLAN entry; rows are unlabelled in output, PLAN order
# fixed, so a row's aspect is recoverable on request.
#
# The 漢字 column lists the words the input should surface, freq-sorted, padded
# with more same-input candidates up to the widest cell in the run — a LOOSE
# expectation (dictionary same-key words by frequency), NOT the engine's actual
# ranked output (no engine is run). The human tester eyeballs the device.
# Just run it: `python3 gen_dogfood.py`.

import csv
import random
import re
import unicodedata
from pathlib import Path

# dictionary/tools/gen_dogfood.py -> repo root is two parents up.
REPO_ROOT = Path(__file__).resolve().parents[2]
CSV_PATH = REPO_ROOT / "dictionary" / "output" / "dictionary.csv"

WORDS_PER_PHRASE = 3  # words concatenated for the continuous-segmentation rows
VOWELS = set("aeiou")
NASAL_SYLLABLES = {"m", "ng", "mh", "ngh"}

TONED = ("tl_num", "poj_num", "tps_num")
TONELESS = ("tl_notone", "poj_notone", "tps_notone")

# Snapshot of engine `phonetics/src/tables.rs` (TL_INITIALS / TL_FINALS). Used to
# decide whether a TPS coda fold is phonotactically valid — i.e. which of the §32
# stop-gate / §33 nasal-gate / §35 de-fold paths a boundary exercises. Keep in
# sync with tables.rs if that list ever changes.
TL_INITIALS = ["tsh", "ts", "ph", "th", "kh", "ng", "p", "m", "b", "t", "n", "l", "k", "g", "s", "j", "h"]
TL_FINALS = frozenset(
    "a ah ap at ak ann annh am an ang e eh enn ennh i ih ip it ik inn innh im in ing o oh oo ooh "
    "op ok om ong onn onnh u uh ut un ai aih ainn ainnh au auh aunn aunnh ia iah iap iat iak iam ian "
    "iang iann iannh io ioh iok iong ionn iu iuh iut iunn iunnh ua uah uat uak uan uann uannh ue ueh "
    "uenn uennh ui uih uinn uinnh iau iauh iaunn iaunnh uai uaih uainn uainnh m mh ng ngh ioo iooh "
    "iai iaih er erh erk erm ere ereh eng ir irh irp irt irk irm irn irng iri irinn ie or orh ior "
    "iorh uang oi oih ee eeh".split()
)
# TPS dual-form coda keys: stops ㄅㄉㄍㄏ + nasals ㄇㄋㄫ. Excludes affricates ts/tsh
# and aspirated ph/th/kh — those have no coda form, so no boundary fold.
STOP_CODA_INITIALS = {"p", "t", "k", "h"}
NASAL_CODA_INITIALS = {"m", "n", "ng"}


def frequency(row):
    try:
        return int(row.get("frequency", "0") or 0)
    except ValueError:
        return 0


def load_rows():
    """Rows with a non-empty reading in all three modes, skipping variants and
    frequency-0 entries."""
    rows = []
    with open(CSV_PATH, encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if any(not row.get(col, "").strip() for col in TONED):
                continue
            if row.get("is_variant", "").strip().lower() == "true":
                continue
            if frequency(row) < 1:
                continue
            rows.append(row)
    return rows


def is_consonant_initial(row):
    t = row["tl_notone"].strip()
    return bool(t) and t[0] not in VOWELS and t[0].isalpha() and any(v in t for v in VOWELS)


def has_shorter_valid_prefix(toneless, all_toneless):
    return any(toneless[:k] in all_toneless for k in range(1, len(toneless)))


def strip_tl_tone(syllable):
    """TL syllable minus tone diacritics, lowercased (NFD strip combining marks)."""
    decomposed = unicodedata.normalize("NFD", syllable)
    return "".join(c for c in decomposed if not unicodedata.combining(c)).strip().lower()


def tl_initial(syllable):
    """The syllable's leading initial (longest match), '' for zero-onset."""
    for initial in sorted(TL_INITIALS, key=len, reverse=True):
        if syllable.startswith(initial):
            return initial
    return ""


def classify_tps_boundary(tl):
    """Which TPS coda-fold path a word's first open-vowel|dual-form-initial boundary
    exercises: 'stop' (§32 #392 gate) / 'nasal' (§33 #394 gate) / 'defold' (§35
    #396) / None. The per-keystroke fold is valid (→ de-fold path) iff the merged
    `<rime><coda>` is a real TL final; otherwise the gate keeps the initial."""
    syllables = [s for s in re.split(r"-+", tl) if s]
    for first, second in zip(syllables, syllables[1:]):
        s1, s2 = strip_tl_tone(first), strip_tl_tone(second)
        if not s1 or not s2 or s1[-1] not in VOWELS:
            continue
        initial = tl_initial(s2)
        if initial in NASAL_CODA_INITIALS:
            kind = "nasal"
        elif initial in STOP_CODA_INITIALS:
            kind = "stop"
        else:
            continue
        rime = s1[len(tl_initial(s1)):]
        return "defold" if (rime + initial) in TL_FINALS else kind
    return None


def build_buckets(pool):
    """Pre-filter the pool into edge-case sub-pools (computed once)."""
    mono = [r for r in pool if len(r["hanzi"].strip()) == 1]
    all_toneless = {r["tl_notone"].strip() for r in pool}
    tps_boundary = {"stop": [], "nasal": [], "defold": []}
    for row in pool:
        category = classify_tps_boundary(row["tl"])
        if category:
            tps_boundary[category].append(row)
    return {
        "all": pool,
        # single-initial wants single-char words so the surfaced candidates are
        # "high-freq single chars".
        "mono_consonant_initial": [r for r in mono if is_consonant_initial(r)],
        "mono": mono,
        "nasal_syllable": [r for r in pool if r["tl_notone"].strip() in NASAL_SYLLABLES],
        # explicit-tone: mark-bearing tones only (1=no mark, 4=stop-coda aspect,
        # 9=tone-9 aspect) so "only this tone" actually holds.
        "mono_toned": [r for r in mono if r["tl_num"].strip()[-1:] in set("235678")],
        "tone9": [r for r in pool if r["tl_num"].strip().endswith("9")],
        "ir_sibilant": [r for r in pool if re.search(r"(ts|tsh|s|j)ir", r["tl_num"])],
        "stop_coda": [r for r in mono if re.search(r"[ptkh][48]$", r["tl_num"].strip())],
        "nasal_vowel": [r for r in mono if "nn" in r["tl_notone"]],
        "oo": [r for r in mono if "oo" in r["tl_notone"]],  # o͘ / TPS ㆦ
        "er_ee": [r for r in mono if "er" in r["tl_notone"] or "ee" in r["tl_notone"]],  # §3.2.6
        "hyphen": [r for r in pool if "-" in r["tl"] and "--" not in r["tl"]],  # 連字 separator
        "khinsiann": [r for r in pool if "--" in r["tl"]],  # 輕聲 `--` separator
        "prefix_collision": [
            r for r in mono
            if len(r["tl_notone"].strip()) >= 2
            and has_shorter_valid_prefix(r["tl_notone"].strip(), all_toneless)
        ],
        "tps_stop_gate": tps_boundary["stop"],    # §32 #392
        "tps_nasal_gate": tps_boundary["nasal"],  # §33 #394
        "tps_defold": tps_boundary["defold"],     # §35 #396
        "multi": [r for r in pool if len(r["hanzi"].strip()) >= 3],
    }


def hanzi(row):
    return row["hanzi"].strip()


def readings(row, columns):
    return [row[col].strip() for col in columns]


def leading_initial(reading):
    """Leading consonant run before the first vowel / tone digit (= the initial
    a user would type for an incomplete syllable)."""
    chars = []
    for char in reading:
        if char.lower() in VOWELS or char.isdigit() or char == "-":
            break
        chars.append(char)
    return "".join(chars)


def freq_sorted_hanzi(rows, exclude):
    """Distinct Hanji of `rows`, frequency-descending, minus `exclude`."""
    out, seen = [], set()
    for row in sorted(rows, key=frequency, reverse=True):
        h = hanzi(row)
        if h and h != exclude and h not in seen:
            seen.add(h)
            out.append(h)
    return out


def same_reading(pool, column, value, exclude):
    return freq_sorted_hanzi([r for r in pool if r[column].strip() == value], exclude)


# --- row builders: each returns [漢字, TL, POJ, TPS, candidates] ---------------
# candidates = extra same-input Hanji (freq-sorted) used to pad the 漢字 cell.

def sample_aspect(bucket_key, columns=TONED, key_column="tl_num"):
    """Factory for the common shape: pick a random word from `bucket_key`, show
    its readings, fill candidates with same-`key_column` words. Used by every
    aspect whose test is just "type this syllable/word, expect its homophones"."""
    def builder(buckets):
        row = random.choice(buckets[bucket_key])
        cands = same_reading(buckets["all"], key_column, row[key_column].strip(), hanzi(row))
        return [hanzi(row), *readings(row, columns), cands]

    return builder


def build_single_initial(buckets):  # #441/#443 lone initial -> high-freq singles
    row = random.choice(buckets["mono_consonant_initial"])
    tl_initial = leading_initial(row["tl_num"])
    siblings = [r for r in buckets["mono_consonant_initial"] if leading_initial(r["tl_num"]) == tl_initial]
    return [
        hanzi(row),
        tl_initial,
        leading_initial(row["poj_num"]),
        row["tps_num"].strip()[:1],
        freq_sorted_hanzi(siblings, hanzi(row)),
    ]


def build_tone9(buckets):  # #434 tone-9, TPS types digit 9 -> ˆ
    row = random.choice(buckets["tone9"])
    cands = same_reading(buckets["all"], "tl_num", row["tl_num"].strip(), hanzi(row))
    return [hanzi(row), row["tl_num"].strip(), row["poj_num"].strip(), row["tps_notone"].strip() + "9", cands]


def build_long_phrase(buckets):  # multi-syllable phrase segmentation (a width-setter)
    row = random.choice(buckets["multi"])
    return [hanzi(row), *readings(row, TONED), []]


def make_continuous(separator, words=2):  # #391 space=soft-sep (TPS) vs literal boundary
    def builder(buckets):
        picks = [random.choice(buckets["all"]) for _ in range(words)]
        joined = [separator.join(p[col].strip() for p in picks) for col in TONED]
        return ["、".join(hanzi(p) for p in picks), *joined, []]

    return builder


# One row per aspect (count = 1 each).
PLAN = [
    build_single_initial,                                  # #441/#443 lone initial
    sample_aspect("nasal_syllable", TONELESS, "tl_notone"),  # m/ng syllable
    sample_aspect("mono_toned"),                           # #367/#433 explicit tone
    sample_aspect("mono", TONELESS, "tl_notone"),          # no tone -> all tones
    build_tone9,                                           # #434 tone-9
    sample_aspect("ir_sibilant"),                          # #435 ir after sibilant
    sample_aspect("stop_coda"),                            # 入聲 stop coda ㆴㆵㆻㆷ
    sample_aspect("nasal_vowel"),                          # nasalized -nn ㆩ/ㆧ
    sample_aspect("oo"),                                   # §1 oo / o͘ vowel ㆦ
    sample_aspect("er_ee"),                                # §3.2.6 er / ee finals
    sample_aspect("prefix_collision"),                     # §18 #371 longest-match prefix
    sample_aspect("hyphen"),                               # §22 #380 連字 separator
    sample_aspect("khinsiann"),                            # §22 #379/#380 輕聲 `--`
    make_continuous("", WORDS_PER_PHRASE),                 # continuous no-space segmentation
    make_continuous(" "),                                  # #391 space soft-sep vs boundary
    sample_aspect("tps_stop_gate", TONELESS, "tps_notone"),   # §32 #392 stop-coda gate
    sample_aspect("tps_nasal_gate", TONELESS, "tps_notone"),  # §33 #394 nasal-coda gate
    sample_aspect("tps_defold", TONELESS, "tps_notone"),      # §35 #396 de-fold enumerate
    build_long_phrase,                                     # multi-syllable phrase
    sample_aspect("mono", TONELESS, "tl_notone"),          # #395/#397 literal-roman
]


def display_width(text):
    """Terminal column width: full/wide CJK + Bopomofo = 2, combining tone
    marks (e.g. U+0307) = 0, everything else = 1 — so the columns line up
    despite Hanji / Bopomofo glyphs."""
    width = 0
    for char in text:
        if unicodedata.combining(char):
            continue
        width += 2 if unicodedata.east_asian_width(char) in ("W", "F") else 1
    return width


def fill_hanzi(primary, candidates, target_width):
    """Append `、`-joined candidates to `primary` up to (not over) target_width."""
    cell = primary
    for candidate in candidates:
        trial = cell + "、" + candidate
        if display_width(trial) > target_width:
            break
        cell = trial
    return cell


def pad(text, width):
    return text + " " * (width - display_width(text))


def render_table(headers, rows):
    columns = list(zip(headers, *rows))
    widths = [max(display_width(cell) for cell in column) for column in columns]
    separator = "+" + "+".join("-" * (width + 2) for width in widths) + "+"

    def format_row(cells):
        return "| " + " | ".join(pad(cell, width) for cell, width in zip(cells, widths)) + " |"

    out = [separator, format_row(headers), separator]
    out += [format_row(row) for row in rows]
    out.append(separator)
    return "\n".join(out)


def main():
    buckets = build_buckets(load_rows())
    built = [builder(buckets) for builder in PLAN]
    target = max(display_width(row[0]) for row in built)  # widest 漢字 (continuous rows)
    rows = [[fill_hanzi(primary, cands, target), tl, poj, tps] for primary, tl, poj, tps, cands in built]
    print(render_table(["漢字", "TL", "POJ", "TPS"], rows))


if __name__ == "__main__":
    main()
