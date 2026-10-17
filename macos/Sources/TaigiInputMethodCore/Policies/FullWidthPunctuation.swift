// Maps typed half-width punctuation to its full-width form for hanji-first output.

import Foundation

/// The full-width punctuation policy: which typed characters become full-width,
/// and when the mapping applies at all.
///
/// Applied only while the Hanji/romanization swap has Hanji coming first, mirroring the
/// MOE input method's rule (full-width in Hanji mode, half-width in TL mode): romanized output reads
/// as Latin text and keeps Latin punctuation, Hanji output reads as CJK text
/// and gets CJK punctuation. That mode IS the whole gate — there is no
/// setting beside it (USER 2026-08-24) — and it was written as the exact
/// complement of the auto-space gate: auto-space served the
/// roman-first mode, this mapping the hanji-first one.
///
/// Since the Hanji/romanization key (2026-08-25) they can meet. Space commits a romanization
/// while the settings lead with hanji, which earns a trailing space in the very
/// mode this mapping serves, so a following `?` matches both rules. The
/// auto-space swap is read first and wins: the word in front of the caret is
/// romanization, and romanization keeps Latin punctuation whatever the mode
/// would say about a hanji word (the key path reads the swap before the map).
/// The mode is the only switch: no chord types the other width (USER
/// 2026-10-07). TPS is full width only; there ⌃ on a key of this map types its
/// mark (`KeyEventSnapshot.tpsPunctuationChord(inputMode:)`) and the swap
/// attaches the full-width glyph.
enum FullWidthPunctuation {
    /// The MOE manual's symbol shortcut table (符號快捷鍵對照表), minus what this input method must
    /// keep half-width: digits are TL/POJ tone markers, the hyphen is the
    /// syllable separator, letters spell the romanization, and the straight
    /// double quote serves as both opening and closing so a one-to-one map
    /// cannot pick a side (the same reason the engine's auto-space attaching set excludes it).
    /// Backslash also types the ideographic comma, alongside the MOE apostrophe shortcut.
    private static let map: [Character: String] = [
        ",": "，", ".": "。", "?": "？", "!": "！",
        ";": "；", ":": "：",
        "(": "（", ")": "）",
        "[": "「", "]": "」",
        "{": "『", "}": "』",
        "<": "《", ">": "》",
        "'": "、", "\\": "、",
        // The shifted number row (`⇧2`…`⇧8`, `⇧-`, `⇧=`): symbols the MOE table
        // also writes full-width. `!` `(` `)` above complete the row; `~` (⇧ on
        // the backtick key) is its first key (user report 2026-09-28).
        "~": "～", "@": "＠", "#": "＃", "$": "＄", "%": "％", "^": "＾", "&": "＆", "*": "＊",
        "_": "＿", "+": "＋",
    ]

    /// Mapped keys whose ⌃ chord stays the host's even under TPS: ⌃⇧` (`~`)
    /// is VS Code's New Terminal on every desktop.
    private static let hostChordKeys: Set<Character> = ["~"]

    /// Whether ⌃ on the key that typed `text` is the TPS punctuation chord: a
    /// mapped key outside `hostChordKeys` (the desktop core's
    /// `is_punctuation_chord_key`).
    static func isPunctuationChordKey(_ text: String) -> Bool {
        mapped(text) != nil && !text.contains(where: hostChordKeys.contains)
    }

    /// The full-width form of one typed character, or nil when the key is not
    /// punctuation this policy maps. Multi-character strings are never mapped:
    /// a key event carries one typed character, and anything longer came from
    /// somewhere this policy has no business rewriting.
    static func mapped(_ text: String) -> String? {
        guard text.count == 1, let character = text.first else { return nil }
        return map[character]
    }
}
