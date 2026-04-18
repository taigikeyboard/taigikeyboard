import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// TL (Tâi-lô) tone-mark assembly.
///
/// Ported from `references/taigi-converter/src/tl.js`.
enum TLFormatter {
    /// Assemble a TL syllable from initial + final + tone number.
    static func toTL(initial: String, final: String, tone: String) -> String {
        var mark = PhoneticsTables.toneNumToCombining[tone] ?? ""
        if tone == "9" { mark = PhoneticsTables.tlTone9Combining }
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
}
