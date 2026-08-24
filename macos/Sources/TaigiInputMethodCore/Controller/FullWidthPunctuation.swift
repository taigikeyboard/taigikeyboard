// Maps typed half-width punctuation to its full-width form for hanji-first output.

import Foundation

/// The full-width punctuation policy: which typed characters become full-width,
/// and when the mapping applies at all.
///
/// Applied only while the 漢羅對調 swap has Hanji coming first, mirroring the
/// MOE input method's rule (漢字模式全形, 臺羅模式半形): romanized output reads
/// as Latin text and keeps Latin punctuation, Hanji output reads as CJK text
/// and gets CJK punctuation. That mode IS the whole gate — there is no
/// setting beside it (USER 2026-08-24) — and it is deliberately the
/// complement of `AutoSpacePolicy.isGateActive`: auto-space serves the
/// roman-first mode, this mapping serves the hanji-first one, and the two are
/// never active together (macOS pins `isOutputBothScripts` false,
/// `RetiredSettingsCleanup`). The caller reads the mode
/// (`TaigiInputController.fullWidthMapped`).
enum FullWidthPunctuation {
    /// The MOE manual's 符號快捷鍵對照表, minus what this input method must
    /// keep half-width: digits are TL/POJ tone markers, the hyphen is the
    /// syllable separator, letters spell the romanization, and the straight
    /// double quote serves as both opening and closing so a one-to-one map
    /// cannot pick a side (the same reason `AutoSpacePunctuation` excludes it).
    private static let map: [Character: String] = [
        ",": "，", ".": "。", "?": "？", "!": "！",
        ";": "；", ":": "：",
        "(": "（", ")": "）",
        "[": "「", "]": "」",
        "{": "『", "}": "』",
        "<": "《", ">": "》",
        "'": "、",
    ]

    /// The full-width form of one typed character, or nil when the key is not
    /// punctuation this policy maps. Multi-character strings are never mapped:
    /// a key event carries one typed character, and anything longer came from
    /// somewhere this policy has no business rewriting.
    static func mapped(_ text: String) -> String? {
        guard text.count == 1, let character = text.first else { return nil }
        return map[character]
    }
}
