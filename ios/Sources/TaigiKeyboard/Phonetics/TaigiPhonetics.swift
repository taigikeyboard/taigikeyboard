import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Taigi phonetics facade.
///
/// Ported from `references/taigi-converter/src/` (tables.js, phonetics.js, tl.js, poj.js).
/// Stable public API; implementation is split across dedicated layers:
/// - `PhoneticsTables` — data tables (initials, finals, combining marks)
/// - `SyllableParser` — strip tones, normalize to TL, split initial/final, parse syllable
/// - `TLFormatter` / `POJFormatter` — assemble tone-marked syllables
/// - `PhoneticsConverter` — high-level syllable and display conversions
///
/// External callers and `TaigiPhoneticsTests` (spec) continue to use `TaigiPhonetics.xxx`.
enum TaigiPhonetics {
    // MARK: - Data Tables (re-export)

    static var toneNumToCombining: [String: String] {
        PhoneticsTables.toneNumToCombining
    }

    static var tlTone9Combining: String {
        PhoneticsTables.tlTone9Combining
    }

    static var combiningToToneNum: [Unicode.Scalar: String] {
        PhoneticsTables.combiningToToneNum
    }

    // MARK: - Parsing

    static func stripToneMark(_ text: String) -> (bare: String, tone: String) {
        SyllableParser.stripToneMark(text)
    }

    static func normalizeToTL(_ text: String) -> String {
        SyllableParser.normalizeToTL(text)
    }

    static func isStopTone(_ final: String) -> Bool {
        SyllableParser.isStopTone(final)
    }

    static func splitInitialFinal(_ text: String) -> (initial: String, final: String)? {
        SyllableParser.splitInitialFinal(text)
    }

    static func parseSyllable(_ text: String) -> (initial: String, final: String, tone: String)? {
        SyllableParser.parseSyllable(text)
    }

    // MARK: - Assembly

    static func toTL(initial: String, final: String, tone: String) -> String {
        TLFormatter.toTL(initial: initial, final: final, tone: tone)
    }

    static func toPOJ(initial: String, final: String, tone: String) -> String {
        POJFormatter.toPOJ(initial: initial, final: final, tone: tone)
    }

    // MARK: - High-Level API

    static func convertSyllable(_ syllable: String, mode: InputMode) -> String {
        PhoneticsConverter.convertSyllable(syllable, mode: mode)
    }

    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        PhoneticsConverter.convertToToneMarks(input, mode: mode)
    }

    static func pojDisplayToTLDisplay(_ text: String) -> String {
        PhoneticsConverter.pojDisplayToTLDisplay(text)
    }

    static func tlDisplayToPOJDisplay(_ text: String) -> String {
        PhoneticsConverter.tlDisplayToPOJDisplay(text)
    }
}
