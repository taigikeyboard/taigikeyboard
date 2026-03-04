import Foundation
import KeyboardKit

/// Taigi-specific tone mapping for callout actions
/// Extends KeyboardKit's Callouts namespace following framework conventions
public extension Callouts {
    /// Provides tone variation mappings for POJ and TL input modes
    /// Maps base characters to their tone variations, sorted by tone number
    enum TaigiToneMaps {
        /// POJ mode: Base character to tone variations mapping
        static let poj: [String: [String]] = buildToneMap(mode: .poj)

        /// TL mode: Base character to tone variations mapping
        static let tl: [String: [String]] = buildToneMap(mode: .tl)

        /// Build tone map dynamically from combining marks
        private static func buildToneMap(mode: InputMode) -> [String: [String]] {
            let baseChars = ["a", "e", "i", "o", "u"]
            let toneNumbers = ["2", "3", "5", "6", "7", "8", "9"]
            var mapping: [String: [String]] = [:]

            for base in baseChars {
                var variations: [String] = []
                for tone in toneNumbers {
                    let mark: String
                    if mode == .poj {
                        mark = tone == "9" ? "\u{0306}" : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    } else {
                        mark = tone == "9" ? TaigiPhonetics.tlTone9Combining : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    }
                    if !mark.isEmpty {
                        let toned = (base + mark).precomposedStringWithCanonicalMapping
                        variations.append(toned)
                    }
                }
                mapping[base] = variations
                mapping[base.uppercased()] = variations.map { $0.uppercased() }
            }

            // POJ: add o͘ variants
            if mode == .poj {
                let oDot = "o\u{0358}"  // o͘
                var oDotVariations: [String] = []
                for tone in toneNumbers {
                    let mark = tone == "9" ? "\u{0306}" : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    if !mark.isEmpty {
                        let toned = ("o" + mark + "\u{0358}").precomposedStringWithCanonicalMapping
                        oDotVariations.append(toned)
                    }
                }
                mapping[oDot] = oDotVariations
                let oDotUpper = "O\u{0358}"  // O͘
                mapping[oDotUpper] = oDotVariations.map { $0.uppercased() }
            }

            // TL: add oo variants
            if mode == .tl {
                var ooVariations: [String] = []
                for tone in toneNumbers {
                    let mark = tone == "9" ? TaigiPhonetics.tlTone9Combining : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    if !mark.isEmpty {
                        let toned = ("o" + mark + "o").precomposedStringWithCanonicalMapping
                        ooVariations.append(toned)
                    }
                }
                mapping["oo"] = ooVariations
                mapping["Oo"] = ooVariations.map { $0.prefix(1).uppercased() + $0.dropFirst() }
            }

            // Syllabic consonants: n, ng, m
            for base in ["n", "m"] {
                var variations: [String] = []
                for tone in toneNumbers {
                    let mark: String
                    if mode == .poj {
                        mark = tone == "9" ? "\u{0306}" : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    } else {
                        mark = tone == "9" ? TaigiPhonetics.tlTone9Combining : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                    }
                    if !mark.isEmpty {
                        let toned = (base + mark).precomposedStringWithCanonicalMapping
                        variations.append(toned)
                    }
                }
                mapping[base] = (mapping[base] ?? []) + variations
                mapping[base.uppercased()] = (mapping[base.uppercased()] ?? []) + variations.map { $0.uppercased() }
            }

            // ng variants: tone mark goes on n
            var ngVariations: [String] = []
            for tone in toneNumbers {
                let mark: String
                if mode == .poj {
                    mark = tone == "9" ? "\u{0306}" : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                } else {
                    mark = tone == "9" ? TaigiPhonetics.tlTone9Combining : (TaigiPhonetics.toneNumToCombining[tone] ?? "")
                }
                if !mark.isEmpty {
                    let toned = ("n" + mark + "g").precomposedStringWithCanonicalMapping
                    ngVariations.append(toned)
                }
            }
            mapping["ng"] = ngVariations
            mapping["Ng"] = ngVariations.map { $0.prefix(1).uppercased() + $0.dropFirst() }

            // Nasal marker ⁿ on 'n' key
            mapping["n"] = (mapping["n"] ?? []) + ["ⁿ"]

            return mapping
        }
    }

    /// TPS layout callouts (方音符號 long-press variants)
    enum TPSCallouts {
        static let actions: [String: [String]] = [
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
            "ㄫ": ["ㆭ", "ㄥ"],
            // Vowels
            "ㆰ": ["ㆱ"],
            "ㆤ": ["ㄝ"],
            // Voiced initials
            "ㆡ": ["ㆢ"],
            // Other consonants
            "ㄙ": ["ㄒ"],
            // Punctuation
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
            "？": ["？", "！", "＊", "＆", "／", "＼", "｜"],
            // Question mark (half-width)
            "?": ["?", "!", "*", "&", "/", "\\", "|"],
        ]
    }
}
