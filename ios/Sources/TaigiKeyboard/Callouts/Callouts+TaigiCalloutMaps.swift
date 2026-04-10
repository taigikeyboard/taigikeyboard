// Callouts+TaigiCalloutMaps.swift
// Callout data: TaigiToneMaps (POJ/TL tone variations), TPSCallouts, MOE1Callouts,
// MOE2Callouts, SymbolCallouts. Each enum provides [String: [String]] action maps.
// Some keys (e.g. ",", ".", "-") intentionally appear in both layout-specific and
// SymbolCallouts with different values — the builder checks layout-specific first.

import Foundation
import KeyboardKit

public extension Callouts {
    /// Maps base characters to their toned variants, sorted by tone number
    enum TaigiToneMaps {
        static let poj: [String: [String]] = buildToneMap(mode: .poj)
        static let tl: [String: [String]] = buildToneMap(mode: .tl)

        /// Returns the combining mark for a given tone number and input mode.
        /// POJ and TL share all marks except tone 9: POJ uses breve (U+0306, already in toneNumToCombining),
        /// TL uses double acute (U+030B).
        private static func combiningMark(for tone: String, mode: InputMode) -> String {
            if tone == "9", mode == .tl {
                return TaigiPhonetics.tlTone9Combining
            }
            return TaigiPhonetics.toneNumToCombining[tone] ?? ""
        }

        /// Builds toned variations by inserting combining marks between base and suffix.
        /// Example: base="o", suffix="o" → ["óo", "òo", "ôo", ...] (TL oo variants)
        private static func buildVariations(
            base: String, suffix: String = "", toneNumbers: [String], mode: InputMode,
        ) -> [String] {
            toneNumbers.compactMap { tone in
                let mark = combiningMark(for: tone, mode: mode)
                guard !mark.isEmpty else { return nil }
                return (base + mark + suffix).precomposedStringWithCanonicalMapping
            }
        }

        private static func buildToneMap(mode: InputMode) -> [String: [String]] {
            let toneNumbers = ["2", "3", "5", "6", "7", "8", "9"]
            var mapping: [String: [String]] = [:]

            for base in ["a", "e", "i", "o", "u"] {
                let variations = buildVariations(base: base, toneNumbers: toneNumbers, mode: mode)
                mapping[base] = variations
                mapping[base.uppercased()] = variations.map { $0.uppercased() }
            }

            // POJ: o͘ (o + combining dot above right U+0358)
            if mode == .poj {
                let variations = buildVariations(base: "o", suffix: "\u{0358}", toneNumbers: toneNumbers, mode: mode)
                mapping["o\u{0358}"] = variations
                // Single logical character — full uppercasing is correct (o͘ → O͘)
                mapping["O\u{0358}"] = variations.map { $0.uppercased() }
            }

            // TL: oo (double o — tone mark on first o)
            if mode == .tl {
                let variations = buildVariations(base: "o", suffix: "o", toneNumbers: toneNumbers, mode: mode)
                mapping["oo"] = variations
                // Multi-char: capitalize only first character (oo → Oo, not OO)
                mapping["Oo"] = variations.map { $0.prefix(1).uppercased() + $0.dropFirst() }
            }

            // Syllabic consonants: n, m (appended to existing vowel entries above)
            for base in ["n", "m"] {
                let variations = buildVariations(base: base, toneNumbers: toneNumbers, mode: mode)
                mapping[base] = (mapping[base] ?? []) + variations
                mapping[base.uppercased()] = (mapping[base.uppercased()] ?? []) + variations.map { $0.uppercased() }
            }

            // ng: tone mark goes on n, g is suffix
            let ngVariations = buildVariations(base: "n", suffix: "g", toneNumbers: toneNumbers, mode: mode)
            mapping["ng"] = ngVariations
            // Multi-char: capitalize only first character (ng → Ng, not NG)
            mapping["Ng"] = ngVariations.map { $0.prefix(1).uppercased() + $0.dropFirst() }

            mapping["n"] = (mapping["n"] ?? []) + ["ⁿ"]

            return mapping
        }
    }

    /// TPS layout callouts (方音符號 long-press variants)
    enum TPSCallouts {
        static let actions: [String: [String]] = [
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
            // Punctuation
            ",": ["。"],
            "，": ["。"],
        ]
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
