# i18n codegen core: parse the canonical i18n/*.json sources and produce every generated artifact.
#
# `build_outputs(repo_root)` returns {repo-relative path -> file content}. generate.py writes them;
# check.py compares them byte-for-byte against the committed tree. Keeping a single pure function as
# the source of truth means the freshness gate cannot drift from the generator.

import json
import re
from pathlib import Path

# --- Schema constants -------------------------------------------------------

# Base language: every key MUST define this (structural — validate_namespace). en/ja are real OS locales;
# tailo/poj are hand-authored in lockstep (P3b/P3c). Beyond the base, the production CLI additionally
# requires every PRODUCTION_LANGUAGES value (validate_production_completeness) — which covers the tailo/poj
# pair too, both being production languages; only a NON-production language's absence is a deliberate fallback.
BASE_LANGUAGE = "hanji"

# Display languages emitted to the Kotlin TL/POJ map (no OS locale -> GeneratedMap path). poj is the POJ
# rendering of the same reading as tailo; both are hand-authored as a pair (kept in step by
# validate_production_completeness — both are production languages, so neither can be missing alone).
GENERATED_MAP_LANGUAGES = ("tailo", "poj")

# Authored, user-selectable production languages: every key MUST define ALL of these (non-empty), so a
# picker option never renders a silent Hanji fallback for a missing translation. tailo/poj joined this
# roster when they were promoted out of debug-only preview (R5-2 / R6-2), so they are now enforced like
# every other production language — including the tailo/poj pair, which needs no separate lockstep gate.
# The lint config disables Android `MissingTranslation`, so this completeness check is the generator's job (R4-3).
# CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — must mirror the picker roster
# in android/.../i18n/DisplayLanguage.kt `productionLanguages` and ios/.../Strings/DisplayLanguage.swift
# `productionLanguages` (hanji/en/ja/tailo/poj), SAME ORDER. Drift would either gate an unshipped language
# or let a shipped one render incomplete.
PRODUCTION_LANGUAGES = ("hanji", "en", "ja", "tailo", "poj")

VALID_PLATFORMS = {"ios", "android", "macos", "windows"}
VALID_SURFACES = {"host", "extension"}
VALID_VALUE_LANGUAGES = {"hanji", "tailo", "poj", "ja", "en"}

# Output locations (repo-relative).
ANDROID_PKG = "android/app/src/main/java/com/siansiansu/taigikeyboard"
ANDROID_RES_ROOT = "android/app/src/main/res"
GEN_PKG_DIR = f"{ANDROID_PKG}/i18n/generated"

# iOS output locations. Both the String Catalog and the generated Swift live where the build
# already picks them up: the catalog is in the host+extension Copy-Bundle-Resources phase, and
# Strings/ is an Xcode synchronized group (auto-includes new files, no pbxproj edit).
IOS_XCSTRINGS = "ios/Localizable.xcstrings"
IOS_STRINGS_DIR = "ios/Sources/TaigiKeyboard/Strings"
IOS_GEN_DIR = f"{IOS_STRINGS_DIR}/Generated"

# Hanji has no Android resource-qualifier dir. This BCP-47 tag deliberately maps to no authored
# Android locale so a Context built from it falls back to the default values/ set. iOS does NOT use
# this tag: App Store Connect rejects the resulting nan-Hant-TW.lproj, so Hanji is a generated map.
BCP47_HANJI = "nan-Hant-TW"

# iOS packages only App-Store-recognized native localizations. Hanji/TL/POJ are product display
# languages, not OS bundle locales, and resolve through GeneratedTaigiStrings.swift instead.
IOS_NATIVE_LANGUAGES = {"en": "en", "ja": "ja"}
XCSTRINGS_SOURCE_LANGUAGE = "en"

# macOS output locations. The input method is a SwiftPM package assembled by scripts/bundle-app.sh
# and installed to ~/Library/Input Methods, so the App Store constraint that splits iOS into
# catalog + map does not apply: ALL FIVE production languages are generated maps and the package
# needs no resource bundle. Behavior (Hanji fallback, `.system` never resolving) stays identical
# across the three platforms — only the storage mechanism differs (intentional divergence).
MACOS_STRINGS_DIR = "macos/Sources/TaigiInputMethodCore/Strings"
MACOS_GEN_DIR = f"{MACOS_STRINGS_DIR}/Generated"

# Windows output location. The input method is a Rust workspace (`windows/`), so its strings are a
# generated Rust module compiled straight into `taigi-windows-core`. Like macOS, ALL FIVE production
# languages are generated maps — there is no resource bundle. The hand-written `strings/mod.rs`
# next to it owns `DisplayLanguage`, `StringResolver` and the positional formatter the generated
# functions call; only this file is ever regenerated.
WINDOWS_STRINGS_DIR = "windows/crates/taigi-windows-core/src/strings"
WINDOWS_GEN_FILE = f"{WINDOWS_STRINGS_DIR}/generated.rs"

# The macOS bundle's OWN localized name — THE reference for this mechanism; everywhere else points here.
#
# macOS resolves an input source's displayed name (Text Input Sources' `kTISPropertyLocalizedName`,
# which is what System Settings -> Keyboard -> Input Sources and the menu-bar input menu show) and
# Finder's app name by matching the SYSTEM language against the `.lproj` directories the bundle
# carries. With none, every system language falls back to the literal `CFBundleName` — which is why
# the input source read "TaigiKeyboard" on a Traditional-Chinese Mac. `scripts/bundle-app.sh` copies
# these into `Contents/Resources/`.
#
# That axis is the system language, NOT the app's own display-language picker (`DisplayLanguageStore`,
# which keeps owning every string drawn inside the app). So only languages a Mac can actually be set
# to get a directory: Tâi-lô and Pe̍h-ōe-jī are product display languages with no OS locale, and Hanji
# maps to `zh-Hant` because that is what a Taiwanese Mac runs. Deliberately NOT derived from
# `IOS_NATIVE_LANGUAGES`: that roster answers to what App Store Connect accepts, which is a different
# constraint that is allowed to diverge from what a Mac's system language can be.
MACOS_APP_DIR = "macos/App"
MACOS_BUNDLE_LOCALIZATIONS = {"en": "en", "ja": "ja", "hanji": "zh-Hant"}

# The i18n key the bundle name is read from. Deliberately NOT expressed as a `macos` platform scope:
# scoping it would also emit it into the generated Swift string map, where nothing would ever look it
# up. It is the product's name, so it is the Home tab's title and the bundle's name by construction.
MACOS_BUNDLE_NAME_KEY = ("home", "appHeaderTitle")

# Both keys carry the same value in every InfoPlist.strings. `CFBundleDisplayName` is what Apple
# documents for a localized app name; Apple documents no order of preference between the two for a
# Text Input Source, and MOE's own 教育部臺灣台語輸入法 writes both — so writing both avoids depending
# on an undocumented implementation detail.
MACOS_BUNDLE_NAME_PLIST_KEYS = ("CFBundleDisplayName", "CFBundleName")

# Value-language key -> Swift `DisplayLanguage` case. The generated maps index by enum case, not by
# tag, and two names differ from their tag (`ja`/`en`) — so this is the ONE place that mapping lives.
# MIRROR: must equal the cases in ios/.../Strings/DisplayLanguage.swift and
# macos/.../Strings/DisplayLanguage.swift.
SWIFT_LANGUAGE_CASES = {"hanji": "hanji", "tailo": "tailo", "poj": "poj", "ja": "japanese", "en": "english"}

# `.system` is a selection policy with no authored strings, so it is never a map key — but every
# generated `lookup` must still answer for it (with nil), which is why it is named separately here.
SWIFT_SYSTEM_CASE = "system"

# Which languages each Swift target keeps in a generated map, as (value-language, enum case) pairs.
# iOS maps only the three with no OS locale; macOS maps the whole production roster (no catalog).
# Derived from PRODUCTION_LANGUAGES so promoting a sixth language cannot silently skip the macOS map —
# it would otherwise resolve to the Hanji fallback at runtime with every gate still green.
IOS_MAP_LANGUAGES = tuple((lang, SWIFT_LANGUAGE_CASES[lang]) for lang in ("hanji", "tailo", "poj"))
MACOS_MAP_LANGUAGES = tuple((lang, SWIFT_LANGUAGE_CASES[lang]) for lang in PRODUCTION_LANGUAGES)

# DisplayLanguage enum case for English, per language — the only count-inflecting display language, so
# the plural-aware typed accessor branches on it alone (`displayLanguage == DisplayLanguage.ENGLISH` /
# `language == .english`). MIRROR: must equal the enum case in every matching DisplayLanguage source —
# android/.../i18n/DisplayLanguage.kt, ios/.../Strings/DisplayLanguage.swift, and
# macos/.../Strings/DisplayLanguage.swift; drift breaks the branch.
ANDROID_ENGLISH_ENUM = "ENGLISH"
SWIFT_ENGLISH_ENUM = SWIFT_LANGUAGE_CASES["en"]

# Languages resolved via native Android resource dirs (plan D2). hanji -> default values/ (always
# emitted); en/ja -> language-qualified dirs, emitted only once a key actually carries that value.
NATIVE_RESOURCE_DIRS = {
    "hanji": "values",
    "en": "values-en",
    "ja": "values-ja",
}

GENERATED_HEADER = "GENERATED by tools/i18n/generate.py — DO NOT EDIT. Run `make i18n`."

PLACEHOLDER_RE = re.compile(r"\{[^}]*\}")
DUPLICATE_KEY_ERROR = "duplicate key"

# Identifier shape shared by namespace keys and placeholder names (both become Kotlin symbols).
IDENTIFIER_RE = re.compile(r"[a-z][a-zA-Z0-9]*")

# A half-width comma in a Hanji value, EXCEPT one sitting between two digits.
#
# Taiwanese written in 漢字 punctuates full-width. Every other i18n namespace already did;
# desktop.json (then macos.json) was the one that drifted (USER 2026-08-24: 「hanji 必須使用全形逗號」). A numeric
# separator (`上限 30,000 項`) is exempt — that is how a number is written, not how a sentence is
# punctuated. Comma only: the half-width `; : ? ( )` still in the sources are product copy, and
# copy is the USER's call.
#
# Applied only AFTER plural is ruled out of the base language: `{n, plural, ...}` carries ASCII
# commas as ICU grammar, so scanning the raw string any earlier reports the wrong error. What is
# left to scan is literal text plus `{name}` placeholders, whose names are comma-free identifiers.
# Revisit if `parse_message` ever grows another comma-bearing argument form (`select`, number
# skeletons) — the scan reads the raw string, not the parsed nodes.
#
# Two alternations, not one comma with two negative lookarounds: the exemption needs a digit on
# BOTH sides, so the rule must fire when EITHER side is a non-digit. A single
# `(?<![0-9]),(?![0-9])` matches only when both sides are non-digits, which lets `上限 30,項`
# through (Codex post-impl).
HANJI_HALFWIDTH_COMMA_RE = re.compile(r"(?<![0-9]),|,(?![0-9])")

# Placeholder type vocabulary: schema type -> per-target facets. The table is the single, complete
# extension point — adding a type propagates to both the typed accessor signature and the emitted
# format spec on every platform. Facets per target:
#   param — accessor signature type (Kotlin / Swift).
#   conv  — printf conversion char. Kotlin `%d` (Java int); Swift `%lld` because Swift Int is 64-bit
#           and Apple's `%d` is 32-bit, so the unsuffixed spec would truncate large counts (Codex Q8.1).
#   cast  — (Swift only) expression wrapping the arg before String(format:) to match its conv char.
#           Kotlin needs no cast (`%d` takes Int directly).
# Keyed by LANGUAGE, not by platform: `%lld` + an `Int64` cast is a property of Foundation's
# `String(format:)`, so iOS and macOS share the `swift` facet rather than declaring one each.
PLACEHOLDER_TYPES = {
    "int": {
        "kotlin": {"param": "Int", "conv": "d"},
        "swift": {"param": "Int", "conv": "lld", "cast": "Int64"},
        # Rust has no printf: the template carries `{N}` positional slots that the hand-written
        # `format_positional` in strings/mod.rs substitutes with `Display` renderings, so the facet
        # declares only the accessor parameter type.
        "rust": {"param": "i64"},
    },
    # For text the product does not author — a store's own error description — so the
    # punctuation around it can be written per language instead of concatenated in code.
    "string": {
        "kotlin": {"param": "String", "conv": "s"},
        "swift": {"param": "String", "conv": "@", "cast": None},
        "rust": {"param": "&str"},
    },
}

# Value-language key -> Rust `DisplayLanguage` variant, and the variant that maps nothing.
# MIRROR: must equal the variants in windows/crates/taigi-windows-core/src/strings/mod.rs.
RUST_LANGUAGE_VARIANTS = {"hanji": "Hanji", "tailo": "Tailo", "poj": "Poj", "ja": "Japanese", "en": "English"}
RUST_SYSTEM_VARIANT = "System"
RUST_MAP_LANGUAGES = tuple((lang, RUST_LANGUAGE_VARIANTS[lang]) for lang in PRODUCTION_LANGUAGES)

# Rust strict + reserved keywords — a generated accessor is a Rust function name and a placeholder
# name a parameter name; reject any that cannot be one. `r#` raw identifiers are deliberately not
# emitted: an accessor that needs one reads worse than a renamed key.
RUST_KEYWORDS = frozenset(
    {
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
        "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "abstract", "become", "box", "do", "final", "gen", "macro", "override",
        "priv", "try", "typeof", "unsized", "virtual", "yield",
    }
)

# Swift hard keywords — a generated accessor name maps to a Swift symbol, so reject any that
# cannot be one. The accessor is namespace-prefixed (`commonCancel`), so a collision is unlikely,
# but the codegen validates it loudly rather than emit code that fails to compile.
SWIFT_KEYWORDS = frozenset(
    {
        "associatedtype", "class", "deinit", "enum", "extension", "fileprivate", "func", "import",
        "init", "inout", "internal", "let", "open", "operator", "private", "protocol", "public",
        "rethrows", "static", "struct", "subscript", "typealias", "var", "break", "case", "continue",
        "default", "defer", "do", "else", "fallthrough", "for", "guard", "if", "in", "repeat",
        "return", "switch", "where", "while", "as", "catch", "false", "is", "nil", "super", "self",
        "throw", "throws", "true", "try",
    }
)

# Kotlin hard keywords — a placeholder name maps directly to a function parameter, so reject any
# name that cannot be a Kotlin identifier.
KOTLIN_KEYWORDS = frozenset(
    {
        "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
        "interface", "is", "null", "object", "package", "return", "super", "this", "throw",
        "true", "try", "typealias", "typeof", "val", "var", "when", "while",
    }
)


# --- Message parsing (placeholders + ICU plural subset) ---------------------
#
# A message is parsed into a small AST so a flat `{name}` and an inline ICU plural
# `{name, plural, one {# entry} other {# entries}}` share one code path. The supported grammar is a
# STRICT subset of ICU MessageFormat (the ARB/TMS-standard authoring syntax):
#   - `{name}`                                   simple argument
#   - `{name, plural, one {...} other {...}}`    plural; `one` arm optional, `other` arm required
#   - `#` inside a plural arm                     the enclosing plural's count
# Deliberately NOT supported (rejected loudly, not silently mis-parsed): nested plural/select,
# `offset:`, `=N` exact selectors, the CLDR `zero`/`two`/`few`/`many` categories, nested `{...}`
# inside an arm, and `'...'` quoting. Plural is authored only in translations whose language inflects
# nouns by count — only English among the 5 display languages — so the runtime selector is `n == 1`
# (CLDR `en`); the codegen rejects a plural authored for any other language (see _emit_*_formats).

PLURAL_CATEGORIES = ("one", "other")  # the CLDR subset this codegen renders


def parse_message(text: str) -> list:
    # Parse a message into AST nodes: ("lit", str) | ("arg", name) | ("plural", name, {category: [nodes]}).
    nodes, _ = _parse_segment(text, 0, len(text), enclosing_plural=None)
    return nodes


def _parse_segment(text: str, i: int, end: int, enclosing_plural):
    # Parse a run of literals / `{...}` placeholders / `#` (only meaningful inside a plural arm).
    nodes = []
    buffer = []

    def flush():
        if buffer:
            nodes.append(("lit", "".join(buffer)))
            buffer.clear()

    while i < end:
        char = text[i]
        if char == "'" and i + 1 < end and text[i + 1] in "{}#'":
            # ICU quoting (`'{'`, `''`, `'#'`) is not supported — it would silently change meaning
            # (a quoted `#` is a literal hash, not the count). A lone apostrophe in prose (`user's`,
            # `Shun'ichi`) is fine: only an apostrophe that begins quoting (next char is ICU syntax)
            # is rejected.
            raise ValueError(f"ICU quoting ('...') is not supported in message: {text!r}")
        if char == "#" and enclosing_plural is not None:
            flush()
            nodes.append(("arg", enclosing_plural))  # `#` == the enclosing plural's count
            i += 1
        elif char == "{":
            flush()
            node, i = _parse_brace(text, i, end)
            nodes.append(node)
        elif char == "}":
            raise ValueError(f"unbalanced '}}' in message: {text!r}")
        else:
            buffer.append(char)
            i += 1
    flush()
    return nodes, i


def _parse_brace(text: str, i: int, end: int):
    # text[i] == '{'. Returns ("arg", name) for `{name}` or ("plural", name, arms) for a plural.
    i += 1
    name, i = _read_until(text, i, end, ",}")
    name = name.strip()
    if i >= end:
        raise ValueError(f"unterminated '{{' in message: {text!r}")
    if not IDENTIFIER_RE.fullmatch(name):
        raise ValueError(f"placeholder name {name!r} must be a lowerCamelCase identifier")
    if text[i] == "}":
        return ("arg", name), i + 1
    # text[i] == ',' → a plural argument: `{name, plural, <category> {<arm>} ...}`.
    i += 1  # skip the ',' after the name
    keyword, i = _read_until(text, i, end, ",}")
    if keyword.strip() != "plural":
        raise ValueError(f"only 'plural' arguments are supported, got {keyword.strip()!r} in {text!r}")
    if i >= end or text[i] != ",":
        raise ValueError(f"plural argument {name!r} is missing its categories in {text!r}")
    i += 1  # skip the ',' after 'plural'
    arms = {}
    while True:
        category, i = _read_until(text, i, end, "{}")
        category = category.strip()
        if i < end and text[i] == "}":
            if category:
                raise ValueError(f"plural category {category!r} is missing its arm {{...}} in {text!r}")
            break  # the plural argument's closing brace
        if i >= end:
            raise ValueError(f"unterminated plural argument {name!r} in {text!r}")
        # text[i] == '{' → an arm begins; `category` is the selector for it.
        if category not in PLURAL_CATEGORIES:
            raise ValueError(f"unsupported plural category {category!r} (only {list(PLURAL_CATEGORIES)}) in {text!r}")
        if category in arms:
            raise ValueError(f"duplicate plural category {category!r} in {text!r}")
        arm_nodes, i = _parse_arm(text, i, end, name)
        arms[category] = arm_nodes
    i += 1  # skip the plural argument's closing '}'
    if "other" not in arms:
        raise ValueError(f"plural argument {name!r} must define the 'other' category in {text!r}")
    return ("plural", name, arms), i


def _parse_arm(text: str, i: int, end: int, plural_name: str):
    # text[i] == '{'. An arm body holds only literals and `#`; nested placeholders/braces are rejected.
    i += 1
    body = []
    while i < end and text[i] != "}":
        if text[i] == "{":
            raise ValueError(f"nested placeholder inside a plural arm is not supported in {text!r}")
        body.append(text[i])
        i += 1
    if i >= end:
        raise ValueError(f"unterminated plural arm in {text!r}")
    # arm_text is brace-free by construction (nested `{` rejected above, `}` ends the body), so the
    # recursive parse only exercises the literal / `#` / quoting branches — never another `{...}`.
    arm_text = "".join(body)
    arm_nodes, _ = _parse_segment(arm_text, 0, len(arm_text), enclosing_plural=plural_name)
    return arm_nodes, i + 1  # +1 skips the arm's closing '}'


def _read_until(text: str, i: int, end: int, stop_chars: str):
    # Reads up to (not including) the first stop char; returns (token, index_at_stop).
    start = i
    while i < end and text[i] not in stop_chars:
        i += 1
    return text[start:i], i


def _names_in_order(nodes: list, accumulator: list) -> list:
    for node in nodes:
        if node[0] in ("arg", "plural"):
            if node[1] not in accumulator:
                accumulator.append(node[1])
            if node[0] == "plural":
                for arm in node[2].values():
                    _names_in_order(arm, accumulator)
    return accumulator


def _has_plural(nodes: list) -> bool:
    return any(node[0] == "plural" for node in nodes)


def _placeholder_names_in_order(text: str) -> list:
    # Names of the placeholders, de-duplicated, in order of first appearance. The base-language text
    # defines the canonical positional order shared by every language (Fork 6: substitute by name).
    return _names_in_order(parse_message(text), [])


def _lower_atom(node, order: list, declared: dict, target: str) -> str:
    # Lower one non-plural node to its positional-template fragment. Shared by the native-resource path
    # (_lower_nodes) and the runtime plural path (_plural_template_parts) so the %N$conv mapping has
    # one definition. `arg` covers both `{name}` and a plural arm's `#` (which parses to an arg node).
    if target == "rust":
        # `format_positional` reads `{N}` (0-based). No `%` to protect and no conversion char to
        # pick. Literal braces need no escaping: the message grammar has no way to author one (an
        # unnamed or unbalanced `{` is rejected by `parse_message`), so a `{` in a lowered template
        # is always a slot.
        if node[0] == "lit":
            return node[1]
        return f"{{{order.index(node[1])}}}"
    if node[0] == "lit":
        return node[1].replace("%", "%%")  # literal % must survive String.format untouched
    index = order.index(node[1]) + 1
    conversion = PLACEHOLDER_TYPES[declared[node[1]]][target]["conv"]
    return f"%{index}${conversion}"


def _lower_nodes(nodes: list, order: list, declared: dict, target: str, category: str) -> str:
    # Render AST nodes to a positional format template (%1$d / %1$lld) under one plural `category`.
    out = []
    for node in nodes:
        if node[0] == "plural":
            arm = node[2][category] if category in node[2] else node[2]["other"]
            out.append(_lower_nodes(arm, order, declared, target, category))
        else:
            out.append(_lower_atom(node, order, declared, target))
    return "".join(out)


def _to_positional(text: str, order: list, declared: dict, target: str) -> str:
    # Convert authored placeholders to positional args (%1$d, %2$d, ...) for `target` (Android %d vs
    # iOS %lld). `order` (base-text appearance order) keeps a reordered translation mapping each name to
    # the right argument. Run BEFORE escaping so the inserted specs are escaped exactly once.
    # Native resources render the `other` arm — the fallback the plural-aware typed accessors override.
    if not order:
        return text  # plain key: leave literal % untouched (it is never String.format'd)
    return _lower_nodes(parse_message(text), order, declared, target, "other")


def _finalize_value(raw: str, escape, entry: dict, target: str) -> str:
    # Convert any placeholders to positional specs for `target`, THEN escape for the output (XML /
    # Kotlin / Swift / JSON-identity). Escaping last is load-bearing: `kotlin_escape` must turn the
    # inserted `%1$d`'s `$` into `\$` so the Kotlin source string is not read as a template; xml_escape
    # and the JSON-identity escape leave it untouched. Order + declared types come from the base-language
    # text, so every language shares one mapping.
    base_text = entry["values"][BASE_LANGUAGE]
    return escape(_to_positional(raw, _placeholder_names_in_order(base_text), entry.get("placeholders", {}), target))


# --- Parsing (with duplicate-key detection) ---------------------------------


def _no_duplicate_keys(pairs):
    # json.load silently keeps the last duplicate; a duplicate key in a source file is almost
    # always an editing mistake that would silently drop a string, so reject it loudly.
    seen = {}
    for key, value in pairs:
        if key in seen:
            raise ValueError(f"{DUPLICATE_KEY_ERROR}: {key!r}")
        seen[key] = value
    return seen


def load_namespace(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        data = json.loads(text, object_pairs_hook=_no_duplicate_keys)
    except ValueError as exc:
        raise ValueError(f"{path}: {exc}") from exc
    return data


# --- Validation -------------------------------------------------------------


def validate_namespace(namespace: str, data: dict, path: Path) -> None:
    if data.get("namespace") != namespace:
        raise ValueError(f"{path}: top-level 'namespace' must equal {namespace!r}")
    keys = data.get("keys")
    if not isinstance(keys, dict):
        raise ValueError(f"{path}: missing 'keys' object")
    for key, entry in keys.items():
        if not IDENTIFIER_RE.fullmatch(key):
            raise ValueError(f"{path}: key {key!r} must be lowerCamelCase")
        scope = entry.get("scope", {})
        platforms = set(scope.get("platforms", []))
        surfaces = set(scope.get("surfaces", []))
        if not platforms or not platforms <= VALID_PLATFORMS:
            raise ValueError(f"{path}:{key}: scope.platforms must be a non-empty subset of {sorted(VALID_PLATFORMS)}")
        if not surfaces or not surfaces <= VALID_SURFACES:
            raise ValueError(f"{path}:{key}: scope.surfaces must be a non-empty subset of {sorted(VALID_SURFACES)}")
        values = entry.get("values", {})
        if BASE_LANGUAGE not in values or not values[BASE_LANGUAGE]:
            raise ValueError(f"{path}:{key}: missing required base-language value '{BASE_LANGUAGE}'")
        unknown = set(values) - VALID_VALUE_LANGUAGES
        if unknown:
            raise ValueError(f"{path}:{key}: unknown value languages {sorted(unknown)}")
        # Placeholder discipline. Canonical names = those used in the base (Hanji) text; positional
        # index is assigned by first appearance there (see _to_positional). A format key MUST declare
        # `placeholders` (name -> type) so the generated typed accessor has an explicit signature.
        try:
            base_nodes = parse_message(values[BASE_LANGUAGE])
        except ValueError as exc:
            raise ValueError(f"{path}:{key}: base '{BASE_LANGUAGE}': {exc}") from exc
        # The base/source language defines the canonical argument order. Hanji (the base) does not
        # inflect nouns by count, so plural belongs only in translations — reject it in the base.
        if _has_plural(base_nodes):
            raise ValueError(f"{path}:{key}: base language '{BASE_LANGUAGE}' must not use plural (author plural only in translations)")
        if HANJI_HALFWIDTH_COMMA_RE.search(values[BASE_LANGUAGE]):
            raise ValueError(
                f"{path}:{key}: '{BASE_LANGUAGE}' must punctuate with the full-width comma '\uff0c', "
                f"not ',' (a comma between digits is exempt): {values[BASE_LANGUAGE]!r}"
            )
        base_placeholder_names = set(_names_in_order(base_nodes, []))
        declared = entry.get("placeholders")
        if base_placeholder_names:
            if not declared:
                raise ValueError(f"{path}:{key}: base text uses placeholders {sorted(base_placeholder_names)} but none declared")
            if set(declared) != base_placeholder_names:
                raise ValueError(f"{path}:{key}: declared placeholders {sorted(declared)} != base text {sorted(base_placeholder_names)}")
            for name, ptype in declared.items():
                # A placeholder name becomes a function parameter on BOTH platforms (Kotlin in
                # StringResolverFormats.kt, Swift in StringResolverFormats.swift), so reject a name
                # that is a keyword on either — e.g. `default` is legal in Kotlin but emits an illegal
                # Swift `default: Int` parameter (Codex post-impl).
                if (
                    not IDENTIFIER_RE.fullmatch(name)
                    or name in KOTLIN_KEYWORDS
                    or name in SWIFT_KEYWORDS
                    or name in RUST_KEYWORDS
                ):
                    raise ValueError(f"{path}:{key}: placeholder name {name!r} must be a lowerCamelCase non-keyword identifier")
                if ptype not in PLACEHOLDER_TYPES:
                    raise ValueError(f"{path}:{key}: placeholder {name!r} type {ptype!r} not in {sorted(PLACEHOLDER_TYPES)}")
        elif declared:
            raise ValueError(f"{path}:{key}: declared placeholders {sorted(declared)} but base text has none")
        for lang, text in values.items():
            # Reject empty authored values — a blank string would render blank, not fall back to base.
            if not text:
                raise ValueError(f"{path}:{key}: empty value for '{lang}'")
            try:
                lang_names = set(_placeholder_names_in_order(text))
            except ValueError as exc:
                raise ValueError(f"{path}:{key}: '{lang}': {exc}") from exc
            # Every authored language must use the same placeholder names as base (order may differ —
            # substitution is by name, not position, so a reordered translation is allowed).
            if lang_names != base_placeholder_names:
                raise ValueError(f"{path}:{key}: placeholder names in '{lang}' differ from base")


# --- Name transforms --------------------------------------------------------


def _camel_to_snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def res_name(namespace: str, key: str) -> str:
    return f"i18n_{namespace}_{key}"


def string_key_const(namespace: str, key: str) -> str:
    return f"{namespace.upper()}_{_camel_to_snake(key).upper()}"


def l10n_accessor(namespace: str, key: str) -> str:
    return f"{namespace}{key[0].upper()}{key[1:]}"


# --- Escaping ---------------------------------------------------------------


def xml_escape(value: str) -> str:
    # `>` is legal as raw text in XML element content (Android strings.xml never escapes it),
    # so only `&` and `<` need entity-escaping; backslash/quotes/newline use Android's backslash form.
    # Backslash is escaped FIRST (matching kotlin_escape/swift_escape) so a literal `\` in a source
    # value reaches the Android resource parser as `\\`, and the backslashes introduced by the
    # quote/newline escaping below are not doubled.
    out = (
        value.replace("\\", "\\\\")
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace('"', "\\\"")
        .replace("'", "\\'")
        .replace("\n", "\\n")
    )
    if out[:1] in ("@", "?"):
        out = "\\" + out
    return out


def kotlin_escape(value: str) -> str:
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("$", "\\$")
        .replace("\n", "\\n")
    )


def swift_escape(value: str) -> str:
    # Swift string-literal escaping. Backslash first so a literal `\(` becomes `\\(` (a backslash +
    # paren, not a string interpolation). Swift has no `$` template syntax, so `%1$lld` is left as-is.
    #
    # ALSO the escaping an `InfoPlist.strings` value takes (`_emit_info_plist_strings`) — a `.strings`
    # value is a C string literal, which is what a Swift one is here. Tune this for a Swift reason and
    # the bundle names change with it.
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
    )


def rust_escape(value: str) -> str:
    # Rust string-literal escaping. Backslash first for the same reason as swift_escape. Braces are
    # left alone: the runtime formatter is hand-written (`format_positional`), not `format!`, so a
    # `{` is only ever the `{N}` slot `_lower_atom` emitted.
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
    )


def rust_variant(namespace: str, key: str) -> str:
    # `commonCancel` -> `CommonCancel`: the accessor with its first letter raised is a valid Rust
    # enum variant, and keeps the key's own casing so `iTaigiDict` stays readable (`CommonITaigiDict`).
    accessor = l10n_accessor(namespace, key)
    return accessor[0].upper() + accessor[1:]


def rust_accessor(namespace: str, key: str) -> str:
    # `desktopUpdateCurrentVersionLabel` -> `desktop_update_current_version_label`.
    return _camel_to_snake(l10n_accessor(namespace, key))


def _json_identity(value: str) -> str:
    # The xcstrings emitter serializes via json.dumps, which performs all JSON escaping; values are
    # passed through unchanged so placeholder conversion is the only transform applied beforehand.
    return value


# --- Emitters ---------------------------------------------------------------


def _validate_global(entries) -> None:
    # Cross-namespace checks the per-file validator cannot see: the synthetic resource name and the
    # typed accessor are both global symbols, so two namespaces must never collapse to the same one,
    # and no accessor may be a reserved keyword on either platform (it becomes a Kotlin val / Swift case).
    seen_res = {}
    seen_accessor = {}
    seen_rust_accessor = {}
    for namespace, key, _entry in entries:
        origin = f"{namespace}:{key}"
        name = res_name(namespace, key)
        if name in seen_res:
            raise ValueError(f"duplicate resource name {name!r}: {seen_res[name]} and {origin}")
        seen_res[name] = origin
        accessor = l10n_accessor(namespace, key)
        if accessor in seen_accessor:
            raise ValueError(f"duplicate accessor {accessor!r}: {seen_accessor[accessor]} and {origin}")
        seen_accessor[accessor] = origin
        if accessor in KOTLIN_KEYWORDS or accessor in SWIFT_KEYWORDS:
            raise ValueError(f"accessor {accessor!r} ({origin}) is a reserved Kotlin/Swift keyword")
        # The Rust accessor is a DIFFERENT transform (snake_case), so it is checked after its own
        # lowering: two distinct camelCase accessors can collapse to one snake_case name
        # (`probeA1` / `probeA_1` both -> `probe_a_1`), and a snake_case name can be a Rust keyword
        # the camelCase form was not.
        rust_fn = rust_accessor(namespace, key)
        if rust_fn in seen_rust_accessor:
            raise ValueError(
                f"duplicate Rust accessor {rust_fn!r}: {seen_rust_accessor[rust_fn]} and {origin} lower to the same snake_case name"
            )
        seen_rust_accessor[rust_fn] = origin
        if rust_fn in RUST_KEYWORDS or rust_variant(namespace, key) in RUST_KEYWORDS:
            raise ValueError(f"accessor {accessor!r} ({origin}) lowers to a reserved Rust identifier")


def validate_production_completeness(entries) -> None:
    # Every production (user-selectable) language MUST be authored for every key, so a picker option
    # never renders a silent Hanji fallback. This is a property of the FULL real source set, enforced at
    # the CLI boundary (generate.py / check.py / Gradle checkI18nGenerated) — NOT inside the generic
    # build_outputs machinery, whose unit tests intentionally use partial fixtures to exercise other
    # paths (scope filtering, partial-language emission). Runs after the structural validators so their
    # more specific errors surface first. tailo and poj are production languages, so this single gate also
    # enforces the tailo/poj pair (neither can be authored without the other passing this check).
    for namespace, key, entry in entries:
        values = entry["values"]
        missing = [lang for lang in PRODUCTION_LANGUAGES if not values.get(lang)]
        if missing:
            raise ValueError(
                f"{namespace}:{key}: missing production language(s) {missing} — every user-selectable "
                f"language {list(PRODUCTION_LANGUAGES)} must be authored"
            )


def validate_platform_has_keys(entries, platform: str) -> None:
    # A platform scoped to zero keys has no strings to ship. On the Swift platforms it does not even
    # compile: a raw-value enum with no cases is not legal Swift ("an enum with no cases cannot
    # declare a raw type"), and the hand-written StringResolver reads `key.rawValue`. Rust tolerates
    # an empty enum, but an input method with no strings is the same mistake. Fail here with the
    # cause instead, naming the fix. Like validate_production_completeness this is a property of the
    # FULL real source set, enforced at the CLI boundary only: unit fixtures scope keys to one
    # platform at a time on purpose, and nothing compiles their output.
    if not entries:
        raise ValueError(
            f"no key is scoped to {platform!r} — the generated StringKey enum would have no cases "
            f"(not even legal Swift for a raw-value enum); scope at least one key to {platform!r}"
        )


def validate_macos_bundle_name_key(bundle_name_values) -> None:
    # Like validate_production_completeness and validate_platform_has_keys, a property of the
    # FULL real source set enforced at the CLI boundary only — unit fixtures author their own tiny
    # namespaces and must not all have to declare a Home tab. Called where the bundle name is emitted,
    # the way validate_platform_has_keys is called where its platform's Swift is.
    if bundle_name_values is None:
        namespace, key = MACOS_BUNDLE_NAME_KEY
        raise ValueError(f"{namespace}:{key} is missing — it is the macOS bundle's localized name")


def _collect_entries(repo_root: Path):
    # Returns an ordered list of (namespace, key, entry) across all i18n/*.json sources.
    src_dir = repo_root / "i18n"
    entries = []
    for path in sorted(src_dir.glob("*.json")):
        namespace = path.stem
        data = load_namespace(path)
        validate_namespace(namespace, data, path)
        for key, entry in data["keys"].items():
            entries.append((namespace, key, entry))
    _validate_global(entries)
    return entries


def _emit_strings_xml(entries, lang: str) -> str:
    lines = ['<?xml version="1.0" encoding="utf-8"?>', f"<!-- {GENERATED_HEADER} -->", "<resources>"]
    for namespace, key, entry in entries:
        if lang not in entry["values"]:
            continue
        value = _finalize_value(entry["values"][lang], xml_escape, entry, "kotlin")
        lines.append(f'    <string name="{res_name(namespace, key)}">{value}</string>')
    lines.append("</resources>")
    return "\n".join(lines) + "\n"


def _emit_string_key(entries) -> str:
    lines = [
        f"// {GENERATED_HEADER}",
        "package com.siansiansu.taigikeyboard.i18n.generated",
        "",
        "import androidx.annotation.StringRes",
        "import com.siansiansu.taigikeyboard.R",
        "",
        "/** Typed key for every i18n string. [resId] points at the Hanji default in res/values/strings_i18n.xml. */",
        "enum class StringKey(",
        "    @StringRes val resId: Int,",
        ") {",
    ]
    for namespace, key, _entry in entries:
        lines.append(f"    {string_key_const(namespace, key)}(R.string.{res_name(namespace, key)}),")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _emit_taigi_map(entries) -> str:
    lines = [
        f"// {GENERATED_HEADER}",
        "package com.siansiansu.taigikeyboard.i18n.generated",
        "",
        "import com.siansiansu.taigikeyboard.i18n.DisplayLanguage",
        "",
        "/**",
        " * TL/POJ string overrides (the GeneratedMap resolution path — these languages have no OS locale).",
        " * Authored as a lockstep pair (tailo + poj hand-authored); a missing language falls back to Hanji.",
        " */",
        "object GeneratedTaigiStrings {",
    ]
    for lang in GENERATED_MAP_LANGUAGES:
        pairs = [(namespace, key, entry) for namespace, key, entry in entries if lang in entry["values"]]
        if pairs:
            lines.append(f"    private val {lang}: Map<StringKey, String> =")
            lines.append("        mapOf(")
            for namespace, key, entry in pairs:
                value = _finalize_value(entry["values"][lang], kotlin_escape, entry, "kotlin")
                lines.append(f'            StringKey.{string_key_const(namespace, key)} to "{value}",')
            lines.append("        )")
        else:
            lines.append(f"    private val {lang}: Map<StringKey, String> = emptyMap()")
    lines.extend(
        [
            "",
            "    fun lookup(",
            "        language: DisplayLanguage,",
            "        key: StringKey,",
            "    ): String? =",
            "        when (language) {",
            "            DisplayLanguage.TAILO -> tailo[key]",
            "            DisplayLanguage.POJ -> poj[key]",
            "            else -> null",
            "        }",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def _emit_l10n(entries) -> str:
    lines = [
        f"// {GENERATED_HEADER}",
        "package com.siansiansu.taigikeyboard.i18n.generated",
        "",
        "import androidx.compose.runtime.Composable",
        "import com.siansiansu.taigikeyboard.i18n.stringRes",
        "",
        "/** Typed Compose accessors — one per plain i18n key, resolved under the active display language. */",
        "object L10n {",
    ]
    for namespace, key, entry in entries:
        # Format keys get a typed StringResolver.<accessor>(args) function instead (StringResolverFormats.kt);
        # a plain String getter would leak the raw "%1$d" template and invite mis-use.
        if _placeholder_names_in_order(entry["values"][BASE_LANGUAGE]):
            continue
        accessor = l10n_accessor(namespace, key)
        const = string_key_const(namespace, key)
        lines.append(f"    val {accessor}: String")
        lines.append(f"        @Composable get() = stringRes(StringKey.{const})")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _plural_languages(entry: dict) -> list:
    # [(lang, nodes)] for authored languages whose message uses plural. Only English may be
    # plural-bearing: the generated selector is the CLDR `en` rule (n == 1), so a plural authored in
    # any other language would silently apply the wrong rule — reject it loudly instead.
    result = []
    for lang, text in entry["values"].items():
        nodes = parse_message(text)
        if _has_plural(nodes):
            if lang != "en":
                raise ValueError(
                    f"plural authored for {lang!r}, but only English has a plural selector "
                    f"(CLDR en: n == 1); add a per-language category rule before authoring plural in {lang!r}"
                )
            result.append((lang, nodes))
    return result


def _plural_template_parts(nodes, order, declared, target, escape, ternary) -> list:
    # AST -> a list of platform expression fragments (joined with `+`) that assemble the positional
    # template at runtime, selecting each plural arm by count. Literal/arg/`#` lower exactly as the
    # native-resource path does; a plural node becomes a per-count ternary over its lowered arms.
    def quoted(text: str) -> str:
        return '"' + escape(text) + '"'

    parts = []
    for node in nodes:
        if node[0] == "plural":
            other = quoted(_lower_nodes(node[2]["other"], order, declared, target, "other"))
            if "one" in node[2]:
                one = quoted(_lower_nodes(node[2]["one"], order, declared, target, "one"))
                parts.append(ternary(node[1], one, other))
            else:
                parts.append(other)
        else:
            parts.append(quoted(_lower_atom(node, order, declared, target)))
    return parts


def _emit_string_resolver_formats(entries) -> str:
    # Typed non-Compose format accessors: one extension fn per format key, args in canonical
    # (base-text first-appearance) order. Callers resolve outside a Composition (coroutine lambdas,
    # Activity callbacks); formatString()/formatTemplate() are the hand-written helpers in StringResolver.kt.
    # Decorate each format entry with its plural languages once (parsing is the cost), so the import
    # block and the bodies share one pass instead of re-parsing every value twice.
    fmt_entries = [
        (namespace, key, entry, _plural_languages(entry))
        for namespace, key, entry in entries
        if _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
    ]
    lines = [
        f"// {GENERATED_HEADER}",
        "package com.siansiansu.taigikeyboard.i18n.generated",
        "",
    ]
    if not fmt_entries:
        lines.append("// No format-arg keys are currently authored.")
        return "\n".join(lines) + "\n"
    has_plural_key = any(plural_langs for *_rest, plural_langs in fmt_entries)
    imports = [
        "import com.siansiansu.taigikeyboard.i18n.StringResolver",
        "import com.siansiansu.taigikeyboard.i18n.formatString",
    ]
    if has_plural_key:
        imports.insert(0, "import com.siansiansu.taigikeyboard.i18n.DisplayLanguage")
        imports.append("import com.siansiansu.taigikeyboard.i18n.formatTemplate")
    lines.extend(
        imports
        + [
            "",
            "// Typed format accessors for placeholder-bearing keys (the raw \"%1\\$d\" template never reaches a call site).",
        ]
    )
    for namespace, key, entry, plural_langs in fmt_entries:
        order = _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
        declared = entry["placeholders"]
        params = ", ".join(f"{name}: {PLACEHOLDER_TYPES[declared[name]]['kotlin']['param']}" for name in order)
        args = ", ".join(order)
        accessor = l10n_accessor(namespace, key)
        const = string_key_const(namespace, key)
        lines.append("")
        if not plural_langs:
            lines.append(f"fun StringResolver.{accessor}({params}): String =")
            lines.append(f"    formatString(StringKey.{const}, {args})")
            continue
        # Only English is plural-bearing (enforced above); its arms are selected at runtime, every
        # other language renders the native-resource `other` fallback via formatString.
        _lang, nodes = plural_langs[0]
        parts = _plural_template_parts(
            nodes, order, declared, "kotlin", kotlin_escape, lambda n, o, t: f"(if ({n} == 1) {o} else {t})"
        )
        template_expr = " +\n                ".join(parts)
        lines.extend(
            [
                f"fun StringResolver.{accessor}({params}): String =",
                f"    if (displayLanguage == DisplayLanguage.{ANDROID_ENGLISH_ENUM}) {{",
                "        // CLDR en plural: category 'one' iff n == 1; native resources hold the 'other' fallback.",
                "        formatTemplate(",
                f"            {template_expr},",
                f"            {args},",
                "        )",
                "    } else {",
                f"        formatString(StringKey.{const}, {args})",
                "    }",
            ]
        )
    return "\n".join(lines) + "\n"


# --- Swift emitters (iOS + macOS) -------------------------------------------


def _swift_format_arg(ptype: str, name: str) -> str:
    cast = PLACEHOLDER_TYPES[ptype]["swift"]["cast"]
    return f"{cast}({name})" if cast else name


def _emit_xcstrings(entries) -> str:
    # iOS String Catalog. Keyed by the synthetic res_name (shared with the Android R.string name, so
    # one naming function owns both platforms). Only App-Store-recognized native languages land here;
    # Hanji/TL/POJ are emitted into GeneratedTaigiStrings.swift and never create .lproj directories.
    # sort_keys makes the committed catalog byte-stable for the freshness check (check.py / Gradle) and aligns the key order
    # with Xcode's own alphabetical sort (comment < extractionState < localizations, state < value).
    strings = {}
    for namespace, key, entry in entries:
        localizations = {}
        for lang, bcp47 in IOS_NATIVE_LANGUAGES.items():
            if lang not in entry["values"]:
                continue
            text = entry["values"][lang]
            value = _finalize_value(text, _json_identity, entry, "swift")
            localizations[bcp47] = {"stringUnit": {"state": "translated", "value": value}}
        unit = {"extractionState": "manual", "localizations": localizations}
        comment = entry.get("comment")
        if comment:
            unit["comment"] = comment
        strings[res_name(namespace, key)] = unit
    catalog = {"sourceLanguage": XCSTRINGS_SOURCE_LANGUAGE, "strings": strings, "version": "1.0"}
    return json.dumps(catalog, indent=2, ensure_ascii=False, sort_keys=True) + "\n"


def _emit_swift_strings_map(entries, *, enum_name: str, doc: tuple, map_languages: tuple) -> str:
    # One map per mapped language plus a `lookup` covering EVERY DisplayLanguage case. The cases this
    # target does not map answer nil: on iOS that is `.system` + the two the String Catalog owns; on
    # macOS only `.system`, which has no authored strings anywhere and is the second guard behind
    # StringResolver's assertion that it never reaches a resolver.
    mapped_cases = [case for _lang, case in map_languages]
    unmapped_cases = [SWIFT_SYSTEM_CASE] + [
        case for case in SWIFT_LANGUAGE_CASES.values() if case not in mapped_cases
    ]
    lines = [f"// {GENERATED_HEADER}", "", *doc, f"enum {enum_name} {{"]
    for lang, case in map_languages:
        pairs = [(namespace, key, entry) for namespace, key, entry in entries if lang in entry["values"]]
        if pairs:
            lines.append(f"    private static let {case}: [StringKey: String] = [")
            for namespace, key, entry in pairs:
                value = _finalize_value(entry["values"][lang], swift_escape, entry, "swift")
                lines.append(f'        .{l10n_accessor(namespace, key)}: "{value}",')
            lines.append("    ]")
        else:
            lines.append(f"    private static let {case}: [StringKey: String] = [:]")
    lines.extend(
        [
            "",
            "    static func lookup(_ language: DisplayLanguage, _ key: StringKey) -> String? {",
            "        switch language {",
        ]
        + [f"        case .{case}: {case}[key]" for case in mapped_cases]
        + [
            "        case " + ", ".join(f".{case}" for case in unmapped_cases) + ": nil",
            "        }",
            "    }",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def _emit_swift_string_key(entries, *, doc: tuple) -> str:
    lines = [
        f"// {GENERATED_HEADER}",
        "",
        *doc,
        "enum StringKey: String {",
    ]
    for namespace, key, _entry in entries:
        lines.append(f'    case {l10n_accessor(namespace, key)} = "{res_name(namespace, key)}"')
    lines.append("}")
    return "\n".join(lines) + "\n"


def _emit_swift_formats(entries, *, plural_fallback_source: str) -> str:
    # Typed format accessors on StringResolver — one per placeholder-bearing key, args in canonical
    # (base-text first-appearance) order, each cast to match the %lld spec. The raw template never
    # reaches a call site; `format(_:_:)` is the hand-written variadic helper in StringResolver.swift.
    # Shared by both Swift targets: `plural_fallback_source` names where each platform's non-English
    # `other` arm is stored (iOS: the String Catalog; macOS: the generated map).
    fmt_entries = [
        (namespace, key, entry, _plural_languages(entry))
        for namespace, key, entry in entries
        if _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
    ]
    lines = [f"// {GENERATED_HEADER}", ""]
    if not fmt_entries:
        lines.append("// No format-arg keys are currently authored.")
        return "\n".join(lines) + "\n"
    lines.extend(["import Foundation", "", "extension StringResolver {"])
    for index, (namespace, key, entry, plural_langs) in enumerate(fmt_entries):
        order = _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
        declared = entry["placeholders"]
        params = ", ".join(
            f"{name}: {PLACEHOLDER_TYPES[declared[name]]['swift']['param']}" for name in order
        )
        args = ", ".join(_swift_format_arg(declared[name], name) for name in order)
        accessor = l10n_accessor(namespace, key)
        if index:
            lines.append("")
        if not plural_langs:
            lines.append(f"    func {accessor}({params}) -> String {{")
            lines.append(f"        format(.{accessor}, {args})")
            lines.append("    }")
            continue
        # Only English is plural-bearing (enforced above); its arms are selected at runtime, every
        # other language renders the stored `other` fallback via format(_:_:).
        _lang, nodes = plural_langs[0]
        parts = _plural_template_parts(
            nodes,
            order,
            declared,
            "swift",
            swift_escape,
            lambda n, o, t: f"({n} == 1 ? {o} : {t})",
        )
        template_expr = "\n                + ".join(parts)
        lines.extend(
            [
                f"    func {accessor}({params}) -> String {{",
                f"        if language == .{SWIFT_ENGLISH_ENUM} {{",
                f"            // CLDR en plural: category 'one' iff n == 1; {plural_fallback_source} holds the 'other' fallback.",
                "            return formatTemplate(",
                f"                {template_expr},",
                f"                {args}",
                "            )",
                "        }",
                f"        return format(.{accessor}, {args})",
                "    }",
            ]
        )
    lines.append("}")
    return "\n".join(lines) + "\n"


def _emit_rust_strings(entries) -> str:
    # One file for the whole Windows string surface: the typed key enum, the per-language lookup
    # tables, and one typed format function per placeholder-bearing key. Everything hand-written
    # (`DisplayLanguage`, `StringResolver`, `format_positional`) lives in the sibling `mod.rs`.
    lines = [
        f"// {GENERATED_HEADER}",
        "",
        "use super::{DisplayLanguage, StringResolver};",
        "",
        "/// Typed key for every Windows i18n string. `as_str` is the shared cross-platform key name,",
        "/// which is also the last-resort fallback text (`StringResolver::resolve`).",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]",
        "pub enum StringKey {",
    ]
    for namespace, key, _entry in entries:
        lines.append(f"    {rust_variant(namespace, key)},")
    lines.extend(["}", "", "impl StringKey {", "    pub fn as_str(self) -> &'static str {", "        match self {"])
    for namespace, key, _entry in entries:
        lines.append(f'            Self::{rust_variant(namespace, key)} => "{res_name(namespace, key)}",')
    lines.extend(["        }", "    }", "}", ""])

    lines.append("pub(super) fn lookup(language: DisplayLanguage, key: StringKey) -> Option<&'static str> {")
    lines.append("    match language {")
    for lang, variant in RUST_MAP_LANGUAGES:
        lines.append(f"        DisplayLanguage::{variant} => {lang}(key),")
    lines.append(f"        DisplayLanguage::{RUST_SYSTEM_VARIANT} => None,")
    lines.extend(["    }", "}"])

    for lang, _variant in RUST_MAP_LANGUAGES:
        pairs = [(namespace, key, entry) for namespace, key, entry in entries if lang in entry["values"]]
        lines.extend(["", f"fn {lang}(key: StringKey) -> Option<&'static str> {{"])
        if not pairs:
            lines.extend(["    let _ = key;", "    None", "}"])
            continue
        lines.append("    Some(match key {")
        for namespace, key, entry in pairs:
            value = _finalize_value(entry["values"][lang], rust_escape, entry, "rust")
            lines.append(f'        StringKey::{rust_variant(namespace, key)} => "{value}",')
        if len(pairs) != len(entries):
            lines.append("        _ => return None,")
        lines.extend(["    })", "}"])

    fmt_entries = [
        (namespace, key, entry, _plural_languages(entry))
        for namespace, key, entry in entries
        if _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
    ]
    lines.append("")
    if not fmt_entries:
        lines.append("// No format-arg keys are currently authored.")
        return "\n".join(lines) + "\n"
    lines.append("impl StringResolver {")
    for index, (namespace, key, entry, plural_langs) in enumerate(fmt_entries):
        order = _placeholder_names_in_order(entry["values"][BASE_LANGUAGE])
        declared = entry["placeholders"]
        params = ", ".join(f"{name}: {PLACEHOLDER_TYPES[declared[name]]['rust']['param']}" for name in order)
        args = ", ".join(f"&{name}" for name in order)
        accessor = rust_accessor(namespace, key)
        variant = rust_variant(namespace, key)
        if index:
            lines.append("")
        lines.append(f"    pub fn {accessor}(&self, {params}) -> String {{")
        if plural_langs:
            # Only English is plural-bearing (enforced above); the stored `other` arm is what every
            # other language renders through the plain lookup. One ternary PER plural node — the
            # Kotlin/Swift model — so a message with several plurals lets each follow its own count.
            _lang, nodes = plural_langs[0]
            parts = _plural_template_parts(
                nodes,
                order,
                declared,
                "rust",
                rust_escape,
                lambda n, o, t: f"if {n} == 1 {{ {o} }} else {{ {t} }}",
            )
            joined = ",\n                ".join(parts)
            lines.extend(
                [
                    f"        if self.language == DisplayLanguage::{RUST_LANGUAGE_VARIANTS['en']} {{",
                    "            // CLDR en plural: category 'one' iff n == 1; the generated map holds the 'other' fallback.",
                    "            let template = [",
                    f"                {joined},",
                    "            ]",
                    "            .concat();",
                    f"            return self.format_template(&template, &[{args}]);",
                    "        }",
                ]
            )
        lines.append(f"        self.format(StringKey::{variant}, &[{args}])")
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _bundle_name_values(all_entries):
    # The app-name entry's values, or None when the source does not carry that key.
    return next(
        (entry["values"] for namespace, key, entry in all_entries if (namespace, key) == MACOS_BUNDLE_NAME_KEY),
        None,
    )


def _emit_info_plist_strings(app_name: str) -> str:
    # Old-style plist (`"key" = "value";`), the format an `InfoPlist.strings` is. Written UTF-8;
    # `scripts/bundle-app.sh` converts it to the binary plist Apple's own apps ship.
    lines = [f"/* {GENERATED_HEADER} */"]
    lines += [f'"{plist_key}" = "{swift_escape(app_name)}";' for plist_key in MACOS_BUNDLE_NAME_PLIST_KEYS]
    return "\n".join(lines) + "\n"


def build_outputs(repo_root: Path, *, enforce_production_completeness: bool = False) -> dict:
    all_entries = _collect_entries(repo_root)
    # Real CLI builds (generate.py / check.py / Gradle) pass True so a missing production translation
    # fails the build. Unit tests default False — they drive build_outputs with partial fixtures to
    # exercise scope filtering / partial-language emission, which the completeness gate would reject.
    if enforce_production_completeness:
        # tailo + poj are production languages, so this one gate also enforces the tailo/poj pair (a
        # half-authored pair fails as a missing production language). No separate lockstep gate needed.
        validate_production_completeness(all_entries)
    # Android artifacts cover only android-scoped keys (D5 scope filtering); an iOS-only key
    # must not leak into the Android resource set.
    entries = [item for item in all_entries if "android" in item[2]["scope"]["platforms"]]

    outputs = {}
    # Native resource sets: always emit the base (Hanji) default; emit en/ja only once a key
    # carries that value, so authoring a language later just works (no silent drop).
    for lang, res_dir in NATIVE_RESOURCE_DIRS.items():
        has_value = any(lang in entry["values"] for _ns, _key, entry in entries)
        if lang == BASE_LANGUAGE or has_value:
            outputs[f"{ANDROID_RES_ROOT}/{res_dir}/strings_i18n.xml"] = _emit_strings_xml(entries, lang)
    outputs[f"{GEN_PKG_DIR}/StringKey.kt"] = _emit_string_key(entries)
    outputs[f"{GEN_PKG_DIR}/GeneratedTaigiStrings.kt"] = _emit_taigi_map(entries)
    outputs[f"{GEN_PKG_DIR}/L10n.kt"] = _emit_l10n(entries)
    outputs[f"{GEN_PKG_DIR}/StringResolverFormats.kt"] = _emit_string_resolver_formats(entries)

    # iOS artifacts cover only ios-scoped keys. English/Japanese use native .lproj bundles;
    # Hanji/TL/POJ use a generated Swift map because App Store Connect rejects their bundle tags.
    ios_entries = [item for item in all_entries if "ios" in item[2]["scope"]["platforms"]]
    if enforce_production_completeness:
        validate_platform_has_keys(ios_entries, "ios")
    outputs[IOS_XCSTRINGS] = _emit_xcstrings(ios_entries)
    outputs[f"{IOS_GEN_DIR}/StringKey.swift"] = _emit_swift_string_key(
        ios_entries,
        doc=("/// Typed key for every iOS i18n string. The raw value is the String Catalog key.",),
    )
    outputs[f"{IOS_GEN_DIR}/GeneratedTaigiStrings.swift"] = _emit_swift_strings_map(
        ios_entries,
        enum_name="GeneratedTaigiStrings",
        doc=("/// Hanji/TL/POJ display strings. These are product languages, not Apple bundle locales.",),
        map_languages=IOS_MAP_LANGUAGES,
    )
    outputs[f"{IOS_GEN_DIR}/StringResolverFormats.swift"] = _emit_swift_formats(
        ios_entries, plural_fallback_source="the catalog"
    )

    # macOS artifacts cover only macos-scoped keys. Every production language is a generated map —
    # the package has no resource bundle at all — so there is no catalog counterpart to emit.
    macos_entries = [item for item in all_entries if "macos" in item[2]["scope"]["platforms"]]
    if enforce_production_completeness:
        validate_platform_has_keys(macos_entries, "macos")
    outputs[f"{MACOS_GEN_DIR}/StringKey.swift"] = _emit_swift_string_key(
        macos_entries,
        doc=("/// Typed key for every macOS i18n string. The raw value is the shared cross-platform key name.",),
    )
    outputs[f"{MACOS_GEN_DIR}/GeneratedStrings.swift"] = _emit_swift_strings_map(
        macos_entries,
        enum_name="GeneratedStrings",
        doc=(
            "/// Display strings for every production language. macOS ships no string catalog, so English",
            "/// and Japanese live here alongside Hanji/TL/POJ instead of in `.lproj` bundles.",
        ),
        map_languages=MACOS_MAP_LANGUAGES,
    )
    outputs[f"{MACOS_GEN_DIR}/StringResolverFormats.swift"] = _emit_swift_formats(
        macos_entries, plural_fallback_source="the generated map"
    )

    # Windows artifacts cover only windows-scoped keys — one generated Rust module, every production
    # language a map, exactly the macOS shape in another language. An empty scope is legal Rust (an
    # empty enum) but would leave the input method with no strings, so it is refused like the Swift
    # platforms are.
    windows_entries = [item for item in all_entries if "windows" in item[2]["scope"]["platforms"]]
    if enforce_production_completeness:
        validate_platform_has_keys(windows_entries, "windows")
    outputs[WINDOWS_GEN_FILE] = _emit_rust_strings(windows_entries)

    # The bundle's own localized name, one `InfoPlist.strings` per system language the bundle answers
    # to. Emitted from the same authored values as everything else so the product can never be named
    # one thing in the app and another in the input-source menu.
    bundle_name_values = _bundle_name_values(all_entries)
    if enforce_production_completeness:
        validate_macos_bundle_name_key(bundle_name_values)
    if bundle_name_values:
        for lang, lproj in MACOS_BUNDLE_LOCALIZATIONS.items():
            outputs[f"{MACOS_APP_DIR}/{lproj}.lproj/InfoPlist.strings"] = _emit_info_plist_strings(
                bundle_name_values[lang]
            )

    return outputs
