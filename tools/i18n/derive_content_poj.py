#!/usr/bin/env python3
# Derive the POJ rendering of every in-app CONTENT string from its authored Tâi-lô (tailo) value and write
# it back into content/*.json as the `poj` value. This is the content/ sibling of derive_poj.py (which owns
# the i18n/*.json namespaces): POJ is a deterministic transliteration of TL with no semantics of its own
# (ts->ch, oo->o͘, nn->ⁿ, ua->oa, ing->eng, ...), so it is DERIVED here rather than hand-authored. Re-run
# after any tailo correction to keep poj in lockstep. Both poj sources go through the ONE canonical
# taigi-converter bridge (dictionary/common/taigi_bridge) the dictionary pipeline uses.
#
# Why a SEPARATE tool and not a generalized derive_poj.py: the i18n derive is a regex rewrite cross-checked
# against an oracle built from the FLAT `keys[k].values[lang]` schema. content/*.json is a NESTED tree whose
# localizable objects are identified by carrying a `hanji` key (feature.title / feature.summary /
# paragraph.text / paragraph.attachment.text). The two share only the strict converter and the
# derive-all-in-memory -> validate -> atomic-write lifecycle, so they stay as two bounded schema adapters.
#
# Like derive_poj.py this is an AUTHORING tool, NOT part of any build/codegen step (the bridge needs Node).
# The bridge runs in STRICT mode here, so a Node/IPC/converter failure aborts the whole run and writes
# nothing rather than silently storing TL as POJ.
#
#   make content-derive-poj                              # write poj into content/*.json
#   python3 tools/i18n/derive_content_poj.py             # same, run directly
#   python3 tools/i18n/derive_content_poj.py --check     # verify committed poj is fresh vs tailo; exit 1 on drift

import argparse
import contextlib
import copy
import json
import os
import re
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))  # for i18n_lib (this dir)
sys.path.insert(0, str(REPO_ROOT / "dictionary" / "common"))  # for taigi_bridge

from i18n_lib import _no_duplicate_keys  # noqa: E402  (one duplicate-key authority for the package)
from taigi_bridge import convert_tl_to_poj_strict  # noqa: E402  (sys.path tweak must precede the import)

CONTENT_FILES = ["content/tab1-features.json", "content/tab1-faq.json"]

# The only keys a localizable LocalizedContentText object may carry (mirrors the iOS/Android model). poj is
# inserted right after tailo so the rendered file groups Taiwanese-script renderings (hanji, tailo, poj)
# before the other languages (en, ja) — matching the i18n/*.json value order.
ALLOWED_LANG_KEYS = {"hanji", "tailo", "poj", "en", "ja"}
LANG_ORDER = ["hanji", "tailo", "poj", "en", "ja"]

# Metalinguistic-literal protection. A few content strings TEACH romanization and embed literal spelling /
# keystroke examples that the TL->POJ converter would wrongly transliterate: the TL spelling `oo` shown next
# to its POJ form `o͘`, or a literal numeric-tone keystroke form like `gau5-tsa2`. Converting them destroys
# the lesson (`「oo」` would render `「o͘」`, erasing the TL-vs-POJ contrast). These spans are kept verbatim in
# the derived POJ. Keyed by the object's stable content path so a same-looking token elsewhere still converts.
#
# Each protected literal must be UNAMBIGUOUS as a plain substring of the tailo — it is matched without word
# boundaries, so a bare `oo` would also wrongly protect the `oo` inside a real word like `tsoo-ha̍p` (組合).
# The spelling examples are therefore protected in their QUOTED form (`「oo」` / `'oo'`); the keystroke forms
# carry tone DIGITS, which never appear inside a real Tâi-lô word, so they are safe bare. Empirically derived
# from the tailo->poj audit; keep MINIMAL — add only on a proven corruption, never to hand-tune a reading
# (correct readings belong in tailo, re-derived in lockstep).
POJ_PROTECT = {
    "features[inputModes].paragraphs[1].text": ["「oo」"],
    "features[romanInput].paragraphs[3].text": ["'oo'"],
    "features[romanInput].paragraphs[5].text": ["'gau5-tsa2'", "'gau5tsa2'"],
    "features[accuracyTips].paragraphs[0].text": ["gau5tsa2"],
    "features[accuracyTips].paragraphs[1].text": ["phang1"],
}


class DeriveError(RuntimeError):
    """A content file failed structural validation — abort the whole run, write nothing."""


def _reject_nan(token: str) -> None:
    raise ValueError(f"non-JSON constant {token!r} in content file")


def _load(text: str) -> dict:
    # Reject duplicate keys (one authority, shared with the i18n tools) and NaN/Infinity (json accepts them
    # by default; content is hand-authored data, never expect non-finite numbers).
    return json.loads(text, object_pairs_hook=_no_duplicate_keys, parse_constant=_reject_nan)


def canonical_json(doc: dict) -> str:
    # The one canonical serialization the content files are stored in (verified byte-exact to the committed
    # sources). Both the freshness check and the written output go through this, so the format has one home.
    return json.dumps(doc, ensure_ascii=False, indent=2)


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise DeriveError(message)


def _localizable_items(doc: dict) -> list[tuple[str, dict]]:
    # Schema-aware traversal: collect (stable_path, object) for every localizable object by its KNOWN path
    # in the content tree. The path keys metalinguistic-literal protection and the review sheet. Every index
    # into the tree is guarded so a malformed file raises a descriptive DeriveError, never a raw KeyError /
    # TypeError. A stable path must be UNIQUE (duplicate ids would alias one path onto several objects and
    # mis-apply protection), so feature/FAQ ids are checked for uniqueness.
    _require(isinstance(doc, dict), "content root is not a JSON object")
    items = doc.get("features")
    root = "features"
    if items is None:
        items, root = doc.get("faqs"), "faqs"
    _require(isinstance(items, list), "content file has no 'features' or 'faqs' list")
    out: list[tuple[str, dict]] = []
    seen_ids: set[str] = set()
    for entry in items:
        _require(isinstance(entry, dict), f"{root} entry is not an object")
        eid = entry.get("id")
        _require(isinstance(eid, str), f"{root} entry has no string 'id'")
        _require(eid not in seen_ids, f"{root}: duplicate id {eid!r}")
        seen_ids.add(eid)
        _require(isinstance(entry.get("title"), dict), f"{root}[{eid}]: 'title' is not an object")
        out.append((f"{root}[{eid}].title", entry["title"]))
        if "summary" in entry:
            _require(isinstance(entry["summary"], dict), f"{root}[{eid}]: 'summary' is not an object")
            out.append((f"{root}[{eid}].summary", entry["summary"]))
        paragraphs = entry.get("paragraphs")
        _require(isinstance(paragraphs, list), f"{root}[{eid}]: 'paragraphs' is not a list")
        for index, paragraph in enumerate(paragraphs):
            _require(isinstance(paragraph, dict), f"{root}[{eid}].paragraphs[{index}] is not an object")
            _require(isinstance(paragraph.get("text"), dict),
                     f"{root}[{eid}].paragraphs[{index}]: 'text' is not an object")
            out.append((f"{root}[{eid}].paragraphs[{index}].text", paragraph["text"]))
            attachment = paragraph.get("attachment")
            if isinstance(attachment, dict) and "text" in attachment:
                _require(isinstance(attachment["text"], dict),
                         f"{root}[{eid}].paragraphs[{index}].attachment: 'text' is not an object")
                out.append((f"{root}[{eid}].paragraphs[{index}].attachment.text", attachment["text"]))
    return out


def _all_hanji_objects(node: object, found: list[dict]) -> None:
    # Reverse assertion support: recursively collect EVERY object carrying a `hanji` key, so a future
    # `{hanji: ..., extra: ...}` object added off the known paths is caught instead of silently skipped.
    if isinstance(node, dict):
        if "hanji" in node:
            found.append(node)
        for value in node.values():
            _all_hanji_objects(value, found)
    elif isinstance(node, list):
        for value in node:
            _all_hanji_objects(value, found)


def _validate_localizable(obj: dict, path_hint: str) -> None:
    if not isinstance(obj.get("hanji"), str):
        raise DeriveError(f"{path_hint}: localizable object missing string 'hanji'")
    extra = set(obj) - ALLOWED_LANG_KEYS
    if extra:
        raise DeriveError(f"{path_hint}: unexpected keys {sorted(extra)} in localizable object")
    for key, value in obj.items():
        if not isinstance(value, str):
            raise DeriveError(f"{path_hint}: value for {key!r} is {type(value).__name__}, expected str")


def _convert_protected(tailo: str, protect: list[str], base_convert) -> str:
    # Convert tailo -> poj, but keep each protected literal verbatim. Protected literals are quote/space
    # delimited spelling/keystroke examples, so splitting on them leaves clean token boundaries for the
    # converter. Longest-first match so a protected `gau5-tsa2` is not split by a shorter `gau5tsa2`.
    if not protect:
        return base_convert(tailo)
    pattern = re.compile("|".join(re.escape(literal) for literal in sorted(protect, key=len, reverse=True)))
    out: list[str] = []
    last = 0
    for match in pattern.finditer(tailo):
        if match.start() > last:
            out.append(base_convert(tailo[last:match.start()]))
        out.append(match.group(0))  # protected literal: kept exactly
        last = match.end()
    if last < len(tailo):
        out.append(base_convert(tailo[last:]))
    return "".join(out)


def _derived_object(path: str, obj: dict, base_convert) -> dict:
    # Rebuild ONE localizable object with poj == convert(tailo) inserted right after tailo, every other field
    # byte-identical, in canonical LANG_ORDER. Objects without a tailo (tailo not yet authored) are returned
    # unchanged — derive only what can be derived. Metalinguistic literals on this path stay verbatim.
    protect = POJ_PROTECT.get(path, [])
    if "tailo" not in obj:
        # A path carrying protection but no tailo is a stale POJ_PROTECT entry — catch it here too, not only
        # when the tailo exists, so removing a tailo can never silently disable protection (false-green).
        if protect:
            raise DeriveError(f"{path}: POJ_PROTECT set but object has no tailo to protect")
        return copy.deepcopy(obj)
    missing = [literal for literal in protect if literal not in obj["tailo"]]
    if missing:
        # A protected literal that no longer occurs in the tailo is a stale entry (e.g. left after a tailo
        # edit) — fail loudly so protection can never silently point at nothing.
        raise DeriveError(f"{path}: POJ_PROTECT literal(s) not found in tailo: {missing}")
    # Spread overwrites any stale poj; the LANG_ORDER filter then drops it back into canonical position.
    values = {**obj, "poj": _convert_protected(obj["tailo"], protect, base_convert)}
    return {key: values[key] for key in LANG_ORDER if key in values}


def _derive_file(path: Path) -> tuple[Path, str, str, set[str]]:
    # Returns (path, original, new_text, known_paths) — original is returned so callers compare in memory
    # instead of re-reading the file. Pure read — writes nothing. Raises DeriveError / RuntimeError /
    # ValueError on any malformed source or strict-bridge failure.
    original = path.read_text(encoding="utf-8")
    source = _load(original)
    if original != canonical_json(source):
        # The structured load/dump must be byte-exact to the source (verified for the committed files), so a
        # clean derive diff only ever adds poj lines. A mismatch means the file was reformatted by hand —
        # refuse rather than reflow the whole file in a poj-derive commit.
        raise DeriveError(
            f"{path.name}: file is not in canonical `json.dumps(indent=2, ensure_ascii=False)` form; "
            f"re-save it canonically before deriving poj"
        )

    known = _localizable_items(source)
    known_ids = {id(obj) for _, obj in known}
    all_hanji: list[dict] = []
    _all_hanji_objects(source, all_hanji)
    if known_ids != {id(obj) for obj in all_hanji}:
        raise DeriveError(
            f"{path.name}: found {len(all_hanji)} objects with a 'hanji' key but {len(known)} on the known "
            f"localizable paths — an object carrying 'hanji' is off the expected schema"
        )
    known_paths = {item_path for item_path, _ in known}
    for item_path, obj in known:
        _validate_localizable(obj, item_path)

    # Convert each distinct tailo FRAGMENT once (the Node bridge is a subprocess round-trip). Per-run cache,
    # not module-level, so the converter stays mockable per test. Fragment-level (not whole-string) caching
    # because protected derivations convert prose gaps around the kept literals.
    poj_cache: dict[str, str] = {}

    def base_convert(fragment: str) -> str:
        if fragment not in poj_cache:
            poj_cache[fragment] = convert_tl_to_poj_strict(fragment)
        return poj_cache[fragment]

    expected = copy.deepcopy(source)
    for item_path, obj in _localizable_items(expected):
        rebuilt = _derived_object(item_path, obj, base_convert)
        obj.clear()
        obj.update(rebuilt)

    new_text = canonical_json(expected)
    # Serialization round-trip: the written bytes must re-parse to exactly the structure we built, so a
    # dumps/loads asymmetry (encoding, key reorder, non-finite) can never reach disk.
    if _load(new_text) != expected:
        raise DeriveError(f"{path.name}: serialized poj derivation did not round-trip cleanly")
    return path, original, new_text, known_paths


def _atomic_write(path: Path, text: str) -> None:
    # Write to canonical content/*.json only. These are the REAL files; the app trees symlink to them, so a
    # tempfile+os.replace here must target the regular file, never follow into an app symlink.
    if path.is_symlink():
        raise DeriveError(f"{path}: refusing to write through a symlink (write the canonical content/ file)")
    original_mode = os.stat(path).st_mode  # preserve the source file's permissions across the replace
    fd, tmp_name = tempfile.mkstemp(dir=str(path.parent), prefix=path.name, suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.chmod(tmp_name, original_mode)  # mkstemp creates 0600; don't tighten the published file's mode
        os.replace(tmp_name, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp_name)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description="Derive content/*.json poj values from tailo via taigi-converter.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify committed poj is the fresh strict derivation of tailo; write nothing, exit 1 on drift",
    )
    args = parser.parse_args()

    # Derive + validate EVERY file in memory before any write, so the tree is never left half-updated. Aborts
    # on a strict-bridge failure (RuntimeError, incl. DeriveError) or a malformed/duplicate-key source
    # (ValueError from the shared loader / json).
    try:
        results = [_derive_file(REPO_ROOT / rel) for rel in CONTENT_FILES]
    except (RuntimeError, ValueError) as exc:
        print(f"content-poj-derive: {exc}", file=sys.stderr)
        return 1

    # A protected path that no longer exists is almost always a stale entry left after a content edit —
    # fail loudly so protection can never silently point at nothing.
    all_paths = set().union(*(known_paths for _, _, _, known_paths in results))
    unknown_protect = sorted(set(POJ_PROTECT) - all_paths)
    if unknown_protect:
        print(f"content-poj-derive: POJ_PROTECT references unknown path(s): {unknown_protect}", file=sys.stderr)
        return 1

    if args.check:
        stale = [str(path.relative_to(REPO_ROOT)) for path, original, new_text, _ in results
                 if original != new_text]
        if stale:
            print(f"content-poj-derive: stale poj (re-run `make content-derive-poj`): {', '.join(stale)}",
                  file=sys.stderr)
            return 1
        print("content-poj-derive: all poj values are fresh")
        return 0

    changed = 0
    for path, original, new_text, _ in results:
        if original != new_text:
            _atomic_write(path, new_text)
            changed += 1
            print(f"content-poj-derive: wrote {path.relative_to(REPO_ROOT)}")
    print(f"content-poj-derive: {changed} file(s) updated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
