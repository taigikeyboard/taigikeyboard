#!/usr/bin/env python3
# Derive the POJ rendering of every i18n string from its authored Tâi-lô (tailo) value and write it back
# into i18n/*.json as the `poj` value. POJ is a deterministic transliteration of TL with no semantics of
# its own (ts→ch, tsh→chh, oo→o͘, nn→ⁿ, ua→oa, ue→oe, ing→eng, ...), so it is DERIVED here rather than
# hand-authored; re-run after any tailo correction to keep poj in sync (the i18n plan D4 "wire
# convert_tl_to_poj" step, realised as derive-and-store). Conversion goes through the canonical
# taigi-converter bridge (dictionary/common/taigi_bridge) — the same path the dictionary pipeline uses,
# so the project has ONE POJ authority. Brand / acronym / {placeholder} / punctuation tokens pass through
# unchanged: the converter only rewrites tokens it recognizes as valid TL syllables (empirically verified
# against all i18n strings — iTaigi / CSV / iOS / {imported} et al. are preserved verbatim).
#
# This is an AUTHORING tool, NOT part of `make i18n`: the codegen stays pure-Python and the
# `make i18n-check` freshness gate never shells out to Node. The bridge is used in STRICT mode here, so a
# Node/IPC/converter failure aborts the whole run rather than silently storing TL as POJ.
#
#   make i18n-derive-poj                          # canonical: write poj into i18n/*.json, then `make i18n`
#   python3 tools/i18n/derive_poj.py              # same, run directly
#   python3 tools/i18n/derive_poj.py --check      # verify committed poj is fresh vs tailo; exit 1 on drift

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))  # for i18n_lib (this dir)
sys.path.insert(0, str(REPO_ROOT / "dictionary" / "common"))  # for taigi_bridge

from i18n_lib import _no_duplicate_keys  # noqa: E402  (one duplicate-key authority for the package)
from taigi_bridge import convert_tl_to_poj_strict  # noqa: E402  (sys.path tweak must precede the import)

# Match `"tailo": "<value>"`, optionally trailed by an existing `, "poj": "<old>"` so a re-run REPLACES
# the derived poj instead of inserting a duplicate. A JSON string body is any run of non-quote/non-escape
# chars or an escape sequence; the closing quote anchors the match so a shorter tailo cannot match inside
# a longer one (e.g. `"tailo": "tâi-gí"` does not match within `"tailo": "tâi-gí-tâi"`). Correctness is
# NOT trusted to the regex alone — every result is re-parsed and structurally validated before any write.
_JSON_STR_BODY = r'(?:[^"\\]|\\.)*'
_TAILO_POJ_RE = re.compile(
    rf'(?P<tailo>"tailo":\s*"(?P<tl>{_JSON_STR_BODY})")(?:,\s*"poj":\s*"{_JSON_STR_BODY}")?'
)


class DeriveError(RuntimeError):
    """A namespace failed structural validation — abort the whole run, write nothing."""


def _load(text: str) -> dict:
    # Reuse the package's one duplicate-key authority (raises ValueError, which main() catches).
    return json.loads(text, object_pairs_hook=_no_duplicate_keys)


def _json_escape_body(text: str) -> str:
    # Escape a plain string for embedding inside JSON double-quotes, then drop the surrounding quotes.
    return json.dumps(text, ensure_ascii=False)[1:-1]


def _expected_values(values: dict, to_poj) -> dict:
    # The post-derive `values` for one entry: every other field byte-identical, poj == to_poj(tailo)
    # inserted/replaced immediately after tailo. Entries without a tailo are left untouched.
    if "tailo" not in values:
        return dict(values)
    expected = {}
    for lang, text in values.items():
        if lang == "poj":
            continue  # re-inserted right after tailo below, so a stale poj cannot survive out of place
        expected[lang] = text
        if lang == "tailo":
            expected["poj"] = to_poj(text)
    return expected


def _derive_namespace(path: Path) -> tuple[str, str]:
    # Returns (relative_path, new_text). Raises DeriveError unless the regex rewrite reproduces EXACTLY
    # the expected structure: same keys, every field byte-identical except a poj == strict(tailo) added
    # after each tailo. Pure read — no file is written here.
    original = path.read_text(encoding="utf-8")
    source = _load(original)
    # Convert each distinct tailo once per namespace — the oracle pass below and the regex pass both need
    # the poj, and the Node bridge is a subprocess round-trip. A per-run cache (not a module-level one)
    # keeps the converter mockable per test. The oracle vs regex cross-check still independently validates
    # the poj's PLACEMENT in the text; only the converted VALUE is shared.
    poj_cache: dict[str, str] = {}

    def to_poj(tailo: str) -> str:
        if tailo not in poj_cache:
            poj_cache[tailo] = convert_tl_to_poj_strict(tailo)
        return poj_cache[tailo]

    # The ONLY allowed change is a poj == to_poj(tailo) added after each entry's tailo. Build the expected
    # FULL document (not just keys) and compare against the regex result, so a `"tailo"` the regex might
    # touch anywhere OTHER than an entry's values (top-level field, future structure) is caught too.
    expected = {**source, "keys": {
        key: {**entry, "values": _expected_values(entry["values"], to_poj)}
        for key, entry in source["keys"].items()
    }}

    def replace(match: re.Match) -> str:
        tailo = json.loads(f'"{match.group("tl")}"')
        return f'{match.group("tailo")}, "poj": "{_json_escape_body(to_poj(tailo))}"'

    new_text = _TAILO_POJ_RE.sub(replace, original)
    if _load(new_text) != expected:
        raise DeriveError(
            f"{path.name}: regex rewrite did not reproduce the expected derive-and-store result "
            f"(check for a poj key not adjacent to tailo, or a tailo field outside an entry's values)"
        )
    return str(path.relative_to(REPO_ROOT)), new_text


def _all_namespaces() -> list[Path]:
    return sorted((REPO_ROOT / "i18n").glob("*.json"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Derive i18n poj values from tailo via taigi-converter.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify committed poj is the fresh strict derivation of tailo; write nothing, exit 1 on drift",
    )
    args = parser.parse_args()

    # Derive + validate EVERY namespace in memory first, before any file is touched, so the source tree is
    # never left half-updated. Aborts on a strict-bridge failure (RuntimeError, incl. DeriveError) or a
    # malformed/duplicate-key source (ValueError from the shared _no_duplicate_keys hook / json).
    try:
        results = [_derive_namespace(path) for path in _all_namespaces()]
    except (RuntimeError, ValueError) as exc:
        print(f"poj-derive: {exc}", file=sys.stderr)
        return 1

    if args.check:
        stale = [rel for rel, new_text in results if (REPO_ROOT / rel).read_text(encoding="utf-8") != new_text]
        if stale:
            print(f"poj-derive: stale poj (re-run `make i18n-derive-poj`): {', '.join(stale)}", file=sys.stderr)
            return 1
        print("poj-derive: all poj values are fresh")
        return 0

    changed = 0
    for rel, new_text in results:
        target = REPO_ROOT / rel
        if target.read_text(encoding="utf-8") != new_text:
            target.write_text(new_text, encoding="utf-8")
            changed += 1
            print(f"poj-derive: wrote {rel}")
    print(f"poj-derive: {changed} file(s) updated — run `make i18n` to regenerate artifacts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
