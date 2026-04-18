import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Phonetics data tables for TL/POJ parsing and assembly.
///
/// Ported from `references/taigi-converter/src/tables.js`.
enum PhoneticsTables {
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
    /// Tone 9 entry here is POJ (breve); TL overrides via `tlToneMark(for:)`.
    static let toneNumToCombining: [String: String] = [
        "1": "", "2": "\u{0301}", "3": "\u{0300}", "4": "",
        "5": "\u{0302}", "6": "\u{030C}", "7": "\u{0304}", "8": "\u{030D}", "9": "\u{0306}",
    ]

    /// TL tone 9 uses double acute accent (U+030B) instead of breve.
    static let tlTone9Combining = "\u{030B}"

    /// Resolve combining mark for TL tone digit. Tone 9 is TL-specific (double acute).
    static func tlToneMark(for tone: String) -> String {
        tone == "9" ? tlTone9Combining : toneNumToCombining[tone] ?? ""
    }

    /// Resolve combining mark for POJ tone digit. Tone 9 already maps to breve.
    static func pojToneMark(for tone: String) -> String {
        toneNumToCombining[tone] ?? ""
    }

    /// Combining mark -> tone number. Includes POJ breve and TL double acute for tone 9.
    static let combiningToToneNum: [Unicode.Scalar: String] = [
        "\u{0301}": "2", // COMBINING ACUTE ACCENT
        "\u{0300}": "3", // COMBINING GRAVE ACCENT
        "\u{0302}": "5", // COMBINING CIRCUMFLEX ACCENT
        "\u{030C}": "6", // COMBINING CARON
        "\u{0304}": "7", // COMBINING MACRON
        "\u{030D}": "8", // COMBINING VERTICAL LINE ABOVE
        "\u{0306}": "9", // COMBINING BREVE (POJ tone 9)
        "\u{030B}": "9", // COMBINING DOUBLE ACUTE ACCENT (TL tone 9)
    ]

    /// All combining scalars recognized as tone marks.
    static let combiningScalars: Set<Unicode.Scalar> = Set(combiningToToneNum.keys)

    /// TL initial -> POJ initial.
    static let pojInitialFromTL: [String: String] = ["ts": "ch", "tsh": "chh"]

    /// TL final -> POJ final substitutions. Order matters: nn before oo.
    static let pojFinalSubstitutions: [(tl: String, poj: String)] = [
        ("nn", "\u{207F}"), // nn -> ⁿ
        ("oo", "o\u{0358}"), // oo -> o͘
        ("ua", "oa"),
        ("ue", "oe"),
        ("ing", "eng"),
        ("ik", "ek"),
    ]
}
