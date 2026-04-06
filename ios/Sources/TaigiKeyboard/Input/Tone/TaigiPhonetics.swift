import Foundation

/// Unified Taigi phonetics engine
///
/// Ported from `references/taigi-converter/src/` (tables.js, phonetics.js, tl.js, poj.js).
/// Replaces ToneMappings, VowelAnalyzer, POJToneConverter, TLToneConverter with
/// a single NFD/NFC-based pipeline: parse syllable -> place combining mark -> normalize.
enum TaigiPhonetics {

    // MARK: - Data Tables

    static let tlInitials: Set<String> = [
        "p", "ph", "m", "b", "t", "th", "n", "l", "k", "kh", "ng", "g",
        "ts", "tsh", "s", "j", "h", "",
    ]

    static let tlFinals: Set<String> = [
        "a", "ah", "ap", "at", "ak", "ann", "annh", "am", "an", "ang",
        "e", "eh", "enn", "ennh",
        "i", "ih", "ip", "it", "ik", "inn", "innh", "im", "in", "ing",
        "o", "oh", "oo", "ooh", "op", "ok", "om", "ong", "onn", "onnh",
        "u", "uh", "ut", "un",
        "ai", "aih", "ainn", "ainnh", "au", "auh", "aunn", "aunnh",
        "ia", "iah", "iap", "iat", "iak", "iam", "ian", "iang", "iann", "iannh",
        "io", "ioh", "iok", "iong", "ionn",
        "iu", "iuh", "iut", "iunn", "iunnh",
        "ua", "uah", "uat", "uak", "uan", "uann", "uannh",
        "ue", "ueh", "uenn", "uennh",
        "ui", "uih", "uinn", "uinnh",
        "iau", "iauh", "iaunn", "iaunnh",
        "uai", "uaih", "uainn", "uainnh",
        "m", "mh", "ng", "ngh",
        "ioo", "iooh", "iai", "iaih",
        "er", "erh", "erk", "erm", "ere", "ereh", "eng",
        "ir", "irh", "irp", "irt", "irk", "irm", "irn", "irng", "irinn", "iri", "ie",
        "or", "orh", "ior", "iorh",
        "uang", "oi", "oih", "ee", "eeh",
    ]

    /// Tone number -> combining mark (NFD). Tones 1 and 4 have no mark.
    static let toneNumToCombining: [String: String] = [
        "1": "", "2": "\u{0301}", "3": "\u{0300}", "4": "",
        "5": "\u{0302}", "6": "\u{030C}", "7": "\u{0304}", "8": "\u{030D}", "9": "\u{0306}",
    ]

    /// TL tone 9 uses double acute accent (U+030B) instead of breve
    static let tlTone9Combining = "\u{030B}"

    /// Combining mark -> tone number (reverse of toneNumToCombining, plus POJ breve and TL double acute)
    static let combiningToToneNum: [Unicode.Scalar: String] = [
        "\u{0301}": "2",  // COMBINING ACUTE ACCENT
        "\u{0300}": "3",  // COMBINING GRAVE ACCENT
        "\u{0302}": "5",  // COMBINING CIRCUMFLEX ACCENT
        "\u{030C}": "6",  // COMBINING CARON
        "\u{0304}": "7",  // COMBINING MACRON
        "\u{030D}": "8",  // COMBINING VERTICAL LINE ABOVE
        "\u{0306}": "9",  // COMBINING BREVE (POJ tone 9)
        "\u{030B}": "9",  // COMBINING DOUBLE ACUTE ACCENT (TL tone 9)
    ]

    /// All combining scalars we recognize as tone marks
    private static let combiningScalars: Set<Unicode.Scalar> = Set(combiningToToneNum.keys)

    /// TL initial -> POJ initial
    static let pojInitialFromTL: [String: String] = ["ts": "ch", "tsh": "chh"]

    /// TL final -> POJ final substitutions (order matters: nn before oo)
    static let pojFinalSubstitutions: [(tl: String, poj: String)] = [
        ("nn", "\u{207F}"),    // nn -> ⁿ
        ("oo", "o\u{0358}"),   // oo -> o͘
        ("ua", "oa"),
        ("ue", "oe"),
        ("ing", "eng"),
        ("ik", "ek"),
    ]

    // MARK: - Core Parsing (from phonetics.js)

    /// Strip tone mark from text, returning (bare NFC text, tone number string).
    /// Recognizes both combining diacritics and trailing digit.
    static func stripToneMark(_ text: String) -> (bare: String, tone: String) {
        let decomposed = text.decomposedStringWithCanonicalMapping
        var foundMark: Unicode.Scalar?

        for scalar in decomposed.unicodeScalars {
            if combiningScalars.contains(scalar) {
                foundMark = scalar
                break
            }
        }

        if let mark = foundMark {
            let toneNum = combiningToToneNum[mark] ?? ""
            let bare = decomposed.unicodeScalars.filter { $0 != mark }
            let bareStr = String(String.UnicodeScalarView(bare))
            return (bareStr.precomposedStringWithCanonicalMapping, toneNum)
        }

        // No combining mark — check trailing digit
        if let last = text.last, let digit = Int(String(last)), (1...9).contains(digit) {
            let bare = String(text.dropLast())
            return (bare.precomposedStringWithCanonicalMapping, String(digit))
        }

        return (text.precomposedStringWithCanonicalMapping, "")
    }

    /// Normalize text to TL spelling (lowercase). Replaces POJ conventions with TL equivalents.
    static func normalizeToTL(_ text: String) -> String {
        var result = text
        result = result.replacingOccurrences(of: "ch", with: "ts")
        result = result.replacingOccurrences(of: "ou", with: "oo")
        result = result.replacingOccurrences(of: "o\u{0358}", with: "oo")
        result = result.replacingOccurrences(of: "\u{207F}", with: "nn")
        result = result.replacingOccurrences(of: "\u{1D3A}", with: "nn")
        result = result.replacingOccurrences(of: "oa", with: "ua")
        result = result.replacingOccurrences(of: "oe", with: "ue")
        result = result.replacingOccurrences(of: "eng", with: "ing")
        result = result.replacingOccurrences(of: "ek", with: "ik")
        result = result.replacingOccurrences(of: "oonn", with: "onn")
        return result
    }

    /// Check if a final ends in a stop consonant (p, t, k, h), ignoring trailing nn.
    static func isStopTone(_ final: String) -> Bool {
        let cleaned = final.lowercased().replacingOccurrences(of: "nn", with: "")
        return cleaned.hasSuffix("p") || cleaned.hasSuffix("t")
            || cleaned.hasSuffix("k") || cleaned.hasSuffix("h")
    }

    /// Split bare TL text into (initial, final) by iterating prefixes.
    static func splitInitialFinal(_ text: String) -> (initial: String, final: String)? {
        for i in 0...text.count {
            let initial = String(text.prefix(i))
            if tlInitials.contains(initial) {
                let final = String(text.dropFirst(i))
                if tlFinals.contains(final) {
                    return (initial, final)
                }
            }
        }
        return nil
    }

    /// Parse a syllable (with tone marks or trailing digit) into (initial, final, tone).
    /// Returns nil if the syllable cannot be parsed.
    static func parseSyllable(_ text: String) -> (initial: String, final: String, tone: String)? {
        let (bare, tone) = stripToneMark(text)
        let normalized = normalizeToTL(bare.lowercased())
        guard let (initial, final) = splitInitialFinal(normalized) else { return nil }
        let finalTone = tone.isEmpty ? (isStopTone(final) ? "4" : "1") : tone
        return (initial, final, finalTone)
    }

    // MARK: - TL Assembly (from tl.js)

    /// Assemble a TL syllable from initial + final + tone number.
    static func toTL(initial: String, final: String, tone: String) -> String {
        var mark = toneNumToCombining[tone] ?? ""
        if tone == "9" { mark = tlTone9Combining }
        let markedFinal = placeTLToneMark(final, mark: mark)
        return (initial + markedFinal).precomposedStringWithCanonicalMapping
    }

    /// Place tone mark on the correct vowel in a TL final.
    /// Rule priority: a > oo > ere > e > o > ui→i > iu→u > iri > i > u > ng > m
    private static func placeTLToneMark(_ final: String, mark: String) -> String {
        guard !mark.isEmpty else { return final }
        if final.contains("a") { return final.replacingFirst(of: "a", with: "a" + mark) }
        if final.contains("oo") { return final.replacingFirst(of: "oo", with: "o" + mark + "o") }
        if final.contains("ere") { return final.replacingFirst(of: "ere", with: "ere" + mark) }
        if final.contains("e") { return final.replacingFirst(of: "e", with: "e" + mark) }
        if final.contains("o") { return final.replacingFirst(of: "o", with: "o" + mark) }
        if final.contains("ui") { return final.replacingFirst(of: "i", with: "i" + mark) }
        if final.contains("iu") { return final.replacingFirst(of: "u", with: "u" + mark) }
        if final.contains("iri") { return final.replacingFirst(of: "iri", with: "iri" + mark) }
        if final.contains("i") { return final.replacingFirst(of: "i", with: "i" + mark) }
        if final.contains("u") { return final.replacingFirst(of: "u", with: "u" + mark) }
        if final.contains("ng") { return final.replacingFirst(of: "ng", with: "n" + mark + "g") }
        if final.contains("m") { return final.replacingFirst(of: "m", with: "m" + mark) }
        return final
    }

    // MARK: - POJ Assembly (from poj.js)

    /// Assemble a POJ syllable from TL initial + final + tone number.
    static func toPOJ(initial: String, final: String, tone: String) -> String {
        let pojInitial = pojInitialFromTL[initial] ?? initial
        let pojFinal = tlFinalToPOJ(final)
        var mark = toneNumToCombining[tone] ?? ""
        if tone == "9" { mark = "\u{0306}" }  // POJ uses breve for tone 9
        let markedFinal = placePOJToneMark(pojFinal, mark: mark)
        return (pojInitial + markedFinal).precomposedStringWithCanonicalMapping
    }

    /// Convert TL final spelling to POJ final spelling.
    static func tlFinalToPOJ(_ final: String) -> String {
        var result = final
        for (tlPart, pojPart) in pojFinalSubstitutions {
            result = result.replacingOccurrences(of: tlPart, with: pojPart)
        }
        return result
    }

    /// Place tone mark on the correct vowel in a POJ final.
    private static func placePOJToneMark(_ final: String, mark: String) -> String {
        guard !mark.isEmpty else { return final }

        // o͘ (o + U+0358) gets mark between o and combining dot
        if final.contains("o\u{0358}") {
            return final.replacingFirst(of: "o\u{0358}", with: "o" + mark + "\u{0358}")
        }

        // iau/oai -> mark on a
        if final.contains("iau") || final.contains("oai") {
            return final.replacingFirst(of: "a", with: "a" + mark)
        }

        // Two adjacent vowels
        if let match = final.range(of: "[aeiou]{2}", options: .regularExpression) {
            let start = match.lowerBound
            let first = final[start]
            let second = final[final.index(after: start)]
            let target: Character

            if first == "i" {
                target = second
            } else if second == "i" {
                target = first
            } else if final.count == 2 {
                target = first
            } else if (final.hasSuffix("\u{207F}") || final.hasSuffix("\u{1D3A}"))
                        && !final.hasSuffix("h\u{207F}") && !final.hasSuffix("h\u{1D3A}") {
                target = first
            } else {
                let afterSecond = final.index(start, offsetBy: 2)
                if afterSecond < final.endIndex {
                    let suffix = final[afterSecond]
                    if "nmgptkh\u{207F}\u{1D3A}".contains(suffix) {
                        target = second
                    } else {
                        target = first
                    }
                } else {
                    target = first
                }
            }
            return final.replacingFirst(of: String(target), with: String(target) + mark)
        }

        // Single vowel
        if let match = final.range(of: "[aeiou]", options: .regularExpression) {
            let vowel = final[match]
            return final.replacingFirst(of: String(vowel), with: String(vowel) + mark)
        }

        // Syllabic consonants: ng, m
        if final.contains("ng") { return final.replacingFirst(of: "n", with: "n" + mark) }
        if final.contains("m") { return final.replacingFirst(of: "m", with: "m" + mark) }
        return final
    }

    // MARK: - High-Level API

    /// Convert a single syllable (with trailing tone digit) to tone-marked form.
    /// Keyboard-specific: tones 1/4 keep the trailing digit (e.g. "gua1" stays "gua1").
    static func convertSyllable(_ syllable: String, mode: InputMode) -> String {
        guard let last = syllable.last,
              let tone = Int(String(last)),
              (1...9).contains(tone)
        else {
            return syllable
        }

        let baseForm = String(syllable.dropLast())
        guard !baseForm.isEmpty else { return syllable }

        // Tone 0 (impossible here), 1, 4: keep trailing digit for composing display
        if tone == 1 || tone == 4 {
            return syllable
        }

        // Parse the base form to get initial/final
        let normalized = normalizeToTL(baseForm.lowercased())
        guard let (initial, final) = splitInitialFinal(normalized) else {
            return syllable
        }

        let toneStr = String(tone)

        // Assemble with tone marks, preserving original case
        let assembled: String
        switch mode {
        case .poj:
            assembled = toPOJ(initial: initial, final: final, tone: toneStr)
        case .tl:
            assembled = toTL(initial: initial, final: final, tone: toneStr)
        case .english, .tps:
            return syllable
        }

        // Restore case: if original starts uppercase, capitalize result
        if let firstOrig = baseForm.first, firstOrig.isUppercase {
            return assembled.prefix(1).uppercased() + assembled.dropFirst()
        }
        return assembled
    }

    /// Convert hyphen-separated input to tone marks.
    /// Splits by "-", converts each syllable, rejoins with "-".
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        let syllables = input.components(separatedBy: "-")
        let converted = syllables.map { syllable in
            syllable.isEmpty ? "" : convertSyllable(syllable, mode: mode)
        }
        return converted.joined(separator: "-")
    }

    /// Convert POJ display text (with diacritics) to TL display text.
    /// Splits by "-" and " " (word boundary), for each syllable: strip tone -> normalizeToTL -> toTL.
    /// Preserves original separators (space = word boundary, hyphen = syllable boundary).
    static func pojDisplayToTLDisplay(_ text: String) -> String {
        guard !text.isEmpty else { return "" }

        // Split while preserving separators (space and hyphen)
        var tokens: [(text: String, separator: String)] = []
        var current = ""
        for char in text {
            if char == "-" || char == " " {
                tokens.append((text: current, separator: String(char)))
                current = ""
            } else {
                current.append(char)
            }
        }
        tokens.append((text: current, separator: ""))

        var result = ""
        for (i, token) in tokens.enumerated() {
            let s = token.text
            if s.isEmpty {
                // Preserve separator (e.g., "--" for 輕聲)
                if i < tokens.count - 1 || !token.separator.isEmpty {
                    result += token.separator
                }
                continue
            }

            let (bare, toneNum) = stripToneMark(s)
            let converted: String
            if bare.isEmpty {
                converted = s
            } else {
                let normalized = normalizeToTL(bare.lowercased())
                if let (initial, final) = splitInitialFinal(normalized) {
                    let tone = toneNum.isEmpty ? (isStopTone(final) ? "4" : "1") : toneNum
                    let tlResult = toTL(initial: initial, final: final, tone: tone)
                    if let first = s.first, first.isUppercase {
                        converted = tlResult.prefix(1).uppercased() + tlResult.dropFirst()
                    } else {
                        converted = tlResult
                    }
                } else {
                    converted = s
                }
            }

            result += converted
            if !token.separator.isEmpty {
                result += token.separator
            }
        }
        return result
    }

    /// Convert TL display text (with diacritics) to POJ display text.
    /// Splits by "-" and " " (word boundary), for each syllable: strip tone -> parse -> toPOJ.
    /// Preserves original separators (space = word boundary, hyphen = syllable boundary).
    static func tlDisplayToPOJDisplay(_ text: String) -> String {
        guard !text.isEmpty else { return "" }

        // Split while preserving separators (space and hyphen)
        var tokens: [(text: String, separator: String)] = []
        var current = ""
        for char in text {
            if char == "-" || char == " " {
                tokens.append((text: current, separator: String(char)))
                current = ""
            } else {
                current.append(char)
            }
        }
        tokens.append((text: current, separator: ""))

        var result = ""
        for (i, token) in tokens.enumerated() {
            let s = token.text
            if s.isEmpty {
                // Preserve separator (e.g., "--" for 輕聲)
                if i < tokens.count - 1 || !token.separator.isEmpty {
                    result += token.separator
                }
                continue
            }

            let (bare, toneNum) = stripToneMark(s)
            let converted: String
            if bare.isEmpty {
                converted = s
            } else {
                let normalized = normalizeToTL(bare.lowercased())
                if let (initial, final) = splitInitialFinal(normalized) {
                    let tone = toneNum.isEmpty ? (isStopTone(final) ? "4" : "1") : toneNum
                    let pojResult = toPOJ(initial: initial, final: final, tone: tone)
                    if let first = s.first, first.isUppercase {
                        converted = pojResult.prefix(1).uppercased() + pojResult.dropFirst()
                    } else {
                        converted = pojResult
                    }
                } else {
                    converted = s
                }
            }

            result += converted
            if !token.separator.isEmpty {
                result += token.separator
            }
        }
        return result
    }
}

// MARK: - String Helper

private extension String {
    /// Replace first occurrence of `target` with `replacement`.
    func replacingFirst(of target: String, with replacement: String) -> String {
        guard let range = self.range(of: target) else { return self }
        var result = self
        result.replaceSubrange(range, with: replacement)
        return result
    }
}
