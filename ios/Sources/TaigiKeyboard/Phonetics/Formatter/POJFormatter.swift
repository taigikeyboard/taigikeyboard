import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// POJ (Pe̍h-ōe-jī) tone-mark assembly.
///
/// Ported from `references/taigi-converter/src/poj.js`.
enum POJFormatter {
    /// Assemble a POJ syllable from TL initial + final + tone number.
    static func toPOJ(initial: String, final: String, tone: String) -> String {
        let pojInitial = PhoneticsTables.pojInitialFromTL[initial] ?? initial
        let pojFinal = tlFinalToPOJ(final)
        let mark = PhoneticsTables.pojToneMark(for: tone)
        let markedFinal = placePOJToneMark(pojFinal, mark: mark)
        return (pojInitial + markedFinal).precomposedStringWithCanonicalMapping
    }

    /// Convert TL final spelling to POJ final spelling.
    static func tlFinalToPOJ(_ final: String) -> String {
        var result = final
        for (tlPart, pojPart) in PhoneticsTables.pojFinalSubstitutions {
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
            } else if final.hasSuffix("\u{207F}") || final.hasSuffix("\u{1D3A}"),
                      !final.hasSuffix("h\u{207F}"), !final.hasSuffix("h\u{1D3A}")
            {
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
}
