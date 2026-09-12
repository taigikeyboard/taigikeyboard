// TaigiCallouts+Maps.swift
// Callout data: TaigiToneMaps (POJ/TL tone variations), TPSCallouts, MOE1Callouts,
// MOE2Callouts, SymbolCallouts. Each enum provides [String: [String]] action maps.
// Some keys (e.g. ",", ".", "-") intentionally appear in both layout-specific and
// SymbolCallouts with different values — the builder checks layout-specific first.

import Foundation
import KeyboardKit

/// Namespace for Taigi long-press callout data and builders. Our own
/// namespace — KK's `Callouts` namespace is deprecated and removed in KK 11.
public enum TaigiCallouts {}

public extension TaigiCallouts {
    /// Maps base characters to their toned variants, sorted by tone number.
    ///
    /// D9.4: tables come from `RustEngineBridge.toneVariations` (init bulk-pull
    /// cached on first access via Swift `static let`, thread-safe by
    /// construction). The previous platform-side `buildToneMap(mode:)` and its
    /// helpers (`combiningMark` / `buildVariations`) are now built in Rust by
    /// `engine/phonetics/src/tone_variations.rs` to match the McBopomofo /
    /// khiin-rs "platform owns zero phonetics" architecture.
    enum TaigiToneMaps {
        static var poj: [String: [String]] {
            RustEngineBridge.toneVariations.poj
        }

        static var tl: [String: [String]] {
            RustEngineBridge.toneVariations.tl
        }
    }

    /// TPS layout callouts (方音符號 long-press variants)
    enum TPSCallouts {
        /// Glyph keys — long-press prepends the key's own glyph before these
        /// variants (see `calloutChars`). USER scope 2026-08-21: letter-variant
        /// + number-shortcut keys include their own glyph.
        static let glyphActions: [String: [String]] = [
            // Row 1: number shortcuts (digits accessible via long-press)
            "ㆠ": ["1"], "ˋ": ["2"], "˪": ["3"], "ㆣ": ["4"], "ˊ": ["5"],
            "ˇ": ["6"], "˫": ["7"], "˙": ["8"], "ㆩ": ["0"],
            // Checked tone finals (入聲韻尾)
            "ㄅ": ["ㆴ"],
            "ㄉ": ["ㆵ"],
            "ㄍ": ["ㆻ"],
            "ㄏ": ["ㆷ"],
            // Affricates
            "ㄗ": ["ㄐ"],
            "ㄘ": ["ㄑ"],
            // Nasals
            "ㄇ": ["ㆬ"],
            "ㄋ": ["ㄣ"],
            "ㄫ": ["ㆭ", "ㄥ"],
            // Vowels
            "ㆰ": ["ㆱ"],
            "ㆤ": ["ㄝ"],
            // Voiced initials
            "ㆡ": ["ㆢ"],
            // Nasalized vowels (ㆪ also has number shortcut "9")
            "ㆪ": ["ㆳ", "9"],
            "ㆮ": ["ㆯ"],
            // Other consonants
            "ㄙ": ["ㄒ"],
        ]

        /// Punctuation keys — long-press stays variant-only (no base glyph).
        static let punctuationActions: [String: [String]] = [
            ",": ["。"],
            "，": ["。"],
        ]

        /// Combined table — keycap hints (`ButtonTextProvider`) read this to
        /// show variants only, so the base glyph never duplicates on its own
        /// keycap. Long-press callouts go through `calloutChars` instead.
        static let actions: [String: [String]] =
            glyphActions.merging(punctuationActions) { glyph, _ in glyph }

        /// Callout characters for a TPS key: the key's own glyph first, then
        /// its variants — so long-press still offers the base letter
        /// (base-first, matching KeyboardKit's English callouts). Punctuation
        /// keeps variant-only callouts. Returns nil for keys without callouts.
        static func calloutChars(for char: String) -> [String]? {
            if let variants = glyphActions[char] {
                return [char] + variants
            }
            return punctuationActions[char]
        }
    }

    /// MOE1 layout punctuation callouts (full-width variants)
    enum MOE1Callouts {
        static let actions: [String: [String]] = [
            // Full-width keys (when isTranslateSwapped)
            "，": ["，", "、", "；", "："],
            "。": ["。", "！", "？", "…"],
            // Half-width keys (when not isTranslateSwapped)
            ",": ["，", "、", "；", "："],
            ".": ["。", "！", "？", "…"],
            // Parentheses (always half-width in MOE1 layout)
            "(": ["（", "『", "「", "《", "〈"],
            ")": ["）", "』", "」", "》", "〉"],
            // Hyphen
            "-": ["-", "_", "~", "'", "^", "@", "#", "\""],
        ]
    }

    /// MOE2 layout punctuation callouts
    enum MOE2Callouts {
        static let actions: [String: [String]] = [
            // Hyphen
            "-": ["-", "_", "~", "'", "^", "@", "#", "\""],
            // Comma (full-width)
            "，": ["，", "、", "；", "：", "（", "「"],
            // Comma (half-width)
            ",": ["，", "、", "；", "：", "（", "「"],
            // Period (full-width)
            "。": ["。", "！", "？", "…", "）", "」"],
            // Period (half-width)
            ".": ["。", "！", "？", "…", "）", "」"],
            // Question mark (full-width)
            "？": ["？", "！", "＊", "＆", "/", "＼", "｜"],
            // Question mark (half-width)
            "?": ["?", "!", "*", "&", "/", "\\", "|"],
        ]
    }

    /// Symbol keyboard callouts (long-press alternatives for numeric & symbolic pages)
    /// Both half-width and full-width entries are needed because the actual character
    /// depends on isTranslateSwapped state.
    enum SymbolCallouts {
        static let actions: [String: [String]] = [
            // === Page 1 Row 1: High-frequency symbols ===
            "~": ["～", "≈", "∼"],
            "～": ["~", "≈", "∼"],
            "/": ["⁄", "÷"],
            "=": ["＝", "≠", "≈", "≡"],
            "＝": ["=", "≠", "≈", "≡"],
            "_": ["＿", "─", "﹏"],
            "＿": ["_", "─", "﹏"],
            "《": ["〈", "«", "‹"],
            "》": ["〉", "»", "›"],
            "〈": ["《", "‹", "<"],
            "〉": ["》", "›", ">"],
            "|": ["｜", "‖", "¦"],
            "｜": ["|", "‖", "¦"],
            "‧": ["·", "•", "°", "‥"],
            "·": ["‧", "•", "°", "‥"],

            // === Page 1 Row 2: Quotation marks & brackets ===
            "\u{201C}": ["「", "«", "‹"], // "
            "\u{201D}": ["」", "»", "›"], // "
            "「": ["\u{201C}", "«", "‹"],
            "」": ["\u{201D}", "»", "›"],
            "\u{2018}": ["『", "‹"], // '
            "\u{2019}": ["』", "›"], // '
            "『": ["\u{2018}", "‹"],
            "』": ["\u{2019}", "›"],
            "(": ["（", "〔", "﹙"],
            ")": ["）", "〕", "﹚"],
            "（": ["(", "〔", "﹙"],
            "）": [")", "〕", "﹚"],
            "【": ["〔", "﹝"],
            "】": ["〕", "﹞"],
            "[": ["［", "〔"],
            "]": ["］", "〕"],

            // === Page 1 Row 3: Punctuation ===
            ":": ["：", "∶"],
            "：": [":", "∶"],
            ";": ["；"],
            "；": [";"],
            "'": ["、", "′", "‛"],
            "、": ["'", "′", "‛"],
            "-": ["–", "—", "﹏"],
            "—": ["─", "–"],
            "...": ["⋯", "‥"],
            "⋯": ["…", "‥"],
            "$": ["€", "£", "¥", "¢", "₩"],
            "%": ["‰", "‱"],
            "#": ["＃", "№"],
            "&": ["＆", "§"],

            // === Page 1 Row 4: Common symbols ===
            ".": ["。", "‧", "·"],
            "。": [".", "‧", "·"],
            ",": ["，", "、"],
            "，": [",", "、"],
            "?": ["？", "¿"],
            "？": ["?", "¿"],
            "!": ["！", "¡", "‽"],
            "！": ["!", "¡", "‽"],
            "*": ["×", "★", "☆"],
            "+": ["±", "＋"],
            "@": ["＠", "©", "®"],

            // === Page 2 Row 1: Programming brackets ===
            "{": ["｛", "﹛"],
            "}": ["｝", "﹜"],
            "｛": ["{", "﹛"],
            "｝": ["}", "﹜"],
            "«": ["‹", "《"],
            "»": ["›", "》"],
            "<": ["＜", "≤", "〈"],
            ">": ["＞", "≥", "〉"],

            // === Page 2 Row 2: Arrows & special symbols ===
            "\\": ["＼", "∖"],
            "＼": ["\\", "∖"],
            "←": ["⇐", "◀"],
            "→": ["⇒", "▶"],
            "↑": ["⇑", "▲"],
            "↓": ["⇓", "▼"],

            // === Page 2 Row 3: Currency & special ===
            "€": ["¢", "$", "£", "¥"],
            "℃": ["℉", "°"],
            "©": ["®", "™"],

            // === Page 2 Row 4: Math ===
            "×": ["*", "·", "✱"],
            "÷": ["/", "⁄"],
            "≠": ["=", "≈", "≡"],
        ]
    }
}
