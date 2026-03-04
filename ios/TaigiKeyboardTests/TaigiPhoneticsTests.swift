import XCTest
@testable import TaigiKeyboard

/// TaigiPhonetics unit tests
/// Ported from references/taigi-converter/tests/{phonetics,tl,poj}.test.js
final class TaigiPhoneticsTests: XCTestCase {

    // MARK: - A. stripToneMark

    func testStripToneMark_acuteAccentTone2() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("\u{00E1}")  // á
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "2")
    }

    func testStripToneMark_graveAccentTone3() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("\u{00E0}")  // à
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "3")
    }

    func testStripToneMark_circumflexTone5() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("\u{00E2}")  // â
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "5")
    }

    func testStripToneMark_macronTone7() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("\u{0101}")  // ā
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "7")
    }

    func testStripToneMark_verticalLineTone8() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("a\u{030D}")
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "8")
    }

    func testStripToneMark_breveTone9_POJ() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("\u{0103}")  // ă (a + breve)
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "9")
    }

    func testStripToneMark_doubleAcuteTone9_TL() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("a\u{030B}")
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "9")
    }

    func testStripToneMark_noMark() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("a")
        XCTAssertEqual(bare, "a")
        XCTAssertEqual(tone, "")
    }

    func testStripToneMark_trailingDigit() {
        let (bare, tone) = TaigiPhonetics.stripToneMark("ka2")
        XCTAssertEqual(bare, "ka")
        XCTAssertEqual(tone, "2")
    }

    func testStripToneMark_multiCharSyllable() {
        // tshiū = tshiu + macron on u
        let (bare, tone) = TaigiPhonetics.stripToneMark("tshi\u{016B}")
        XCTAssertEqual(bare, "tshiu")
        XCTAssertEqual(tone, "7")
    }

    // MARK: - B. normalizeToTL

    func testNormalizeToTL_cases() {
        let cases: [(input: String, expected: String)] = [
            ("ch", "ts"),
            ("chh", "tsh"),
            ("oa", "ua"),
            ("oe", "ue"),
            ("eng", "ing"),
            ("ek", "ik"),
            ("ou", "oo"),
            ("o\u{0358}", "oo"),  // o͘ -> oo
            ("\u{207F}", "nn"),   // ⁿ -> nn
            ("oonn", "onn"),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                TaigiPhonetics.normalizeToTL(input), expected,
                "normalizeToTL(\(input)) should be \(expected)"
            )
        }
    }

    // MARK: - C. isStopTone

    func testIsStopTone_stopEndings() {
        let stops: [(final: String, expected: Bool)] = [
            ("ap", true),
            ("at", true),
            ("ak", true),
            ("ah", true),
            ("a", false),
            ("an", false),
            ("ang", false),
            ("annh", true),  // nasal with h
        ]
        for (final, expected) in stops {
            XCTAssertEqual(
                TaigiPhonetics.isStopTone(final), expected,
                "isStopTone(\(final)) should be \(expected)"
            )
        }
    }

    // MARK: - D. splitInitialFinal

    func testSplitInitialFinal_validSyllables() {
        let cases: [(input: String, initial: String, final: String)] = [
            ("ka", "k", "a"),
            ("tshiu", "tsh", "iu"),
            ("a", "", "a"),       // no initial
            ("ng", "", "ng"),     // syllabic ng
            ("m", "", "m"),       // syllabic m
            ("phang", "ph", "ang"),
            ("iang", "", "iang"),
            ("oo", "", "oo"),
        ]
        for (input, expectedInitial, expectedFinal) in cases {
            let result = TaigiPhonetics.splitInitialFinal(input)
            XCTAssertNotNil(result, "splitInitialFinal(\(input)) should not be nil")
            XCTAssertEqual(result?.initial, expectedInitial, "initial of \(input)")
            XCTAssertEqual(result?.final, expectedFinal, "final of \(input)")
        }
    }

    func testSplitInitialFinal_invalidReturnsNil() {
        XCTAssertNil(TaigiPhonetics.splitInitialFinal("xyz"), "splitInitialFinal(\"xyz\") should be nil")
    }

    // MARK: - E. parseSyllable

    func testParseSyllable_simpleCases() {
        let cases: [(input: String, initial: String, final: String, tone: String)] = [
            // Simple digit tones
            ("ka2", "k", "a", "2"),
            ("kang1", "k", "ang", "1"),
            ("a1", "", "a", "1"),
            // Tone mark
            ("k\u{00E1}", "k", "a", "2"),  // ká
            // Inferred tones
            ("kah", "k", "ah", "4"),   // stop tone -> 4
            ("ka", "k", "a", "1"),     // non-stop -> 1
            // Aspirated initial
            ("pha3", "ph", "a", "3"),
            // tsh initial
            ("tshiu7", "tsh", "iu", "7"),
        ]
        for (input, expectedInitial, expectedFinal, expectedTone) in cases {
            let result = TaigiPhonetics.parseSyllable(input)
            XCTAssertNotNil(result, "parseSyllable(\(input)) should not be nil")
            XCTAssertEqual(result?.initial, expectedInitial, "initial of \(input)")
            XCTAssertEqual(result?.final, expectedFinal, "final of \(input)")
            XCTAssertEqual(result?.tone, expectedTone, "tone of \(input)")
        }
    }

    func testParseSyllable_pojForms() {
        let cases: [(input: String, initial: String, final: String, tone: String)] = [
            ("chhi2", "tsh", "i", "2"),    // ch->ts, chh->tsh
            ("koa1", "k", "ua", "1"),      // oa->ua
            ("koe1", "k", "ue", "1"),      // oe->ue
            ("peng5", "p", "ing", "5"),    // eng->ing
        ]
        for (input, expectedInitial, expectedFinal, expectedTone) in cases {
            let result = TaigiPhonetics.parseSyllable(input)
            XCTAssertNotNil(result, "parseSyllable(\(input)) should not be nil")
            XCTAssertEqual(result?.initial, expectedInitial, "initial of \(input)")
            XCTAssertEqual(result?.final, expectedFinal, "final of \(input)")
            XCTAssertEqual(result?.tone, expectedTone, "tone of \(input)")
        }
    }

    func testParseSyllable_syllabicConsonants() {
        let ngResult = TaigiPhonetics.parseSyllable("ng5")
        XCTAssertNotNil(ngResult, "parseSyllable(\"ng5\") should not be nil")
        XCTAssertEqual(ngResult?.initial, "", "initial of ng5")
        XCTAssertEqual(ngResult?.final, "ng", "final of ng5")
        XCTAssertEqual(ngResult?.tone, "5", "tone of ng5")

        let mResult = TaigiPhonetics.parseSyllable("m7")
        XCTAssertNotNil(mResult, "parseSyllable(\"m7\") should not be nil")
        XCTAssertEqual(mResult?.initial, "", "initial of m7")
        XCTAssertEqual(mResult?.final, "m", "final of m7")
        XCTAssertEqual(mResult?.tone, "7", "tone of m7")
    }

    func testParseSyllable_invalidReturnsNil() {
        XCTAssertNil(TaigiPhonetics.parseSyllable("xyz"), "parseSyllable(\"xyz\") should be nil")
    }

    // MARK: - F. toTL

    func testToTL_allTones() {
        let cases: [(initial: String, final: String, tone: String, expected: String)] = [
            ("k", "a", "1", "ka"),        // tone 1: no mark
            ("k", "a", "2", "k\u{00E1}"), // ká
            ("k", "a", "3", "k\u{00E0}"), // kà
            ("k", "ah", "4", "kah"),      // tone 4: no mark
            ("k", "a", "5", "k\u{00E2}"), // kâ
            ("k", "a", "7", "k\u{0101}"), // kā
            ("k", "ah", "8", "ka\u{030D}h"), // ka̍h
        ]
        for (initial, final, tone, expected) in cases {
            let result = TaigiPhonetics.toTL(initial: initial, final: final, tone: tone)
            XCTAssertEqual(result, expected, "toTL(\(initial), \(final), \(tone))")
        }
    }

    func testToTL_tone9_doubleAcute() {
        let result = TaigiPhonetics.toTL(initial: "k", final: "a", tone: "9")
        XCTAssertTrue(
            result.unicodeScalars.contains("\u{030B}"),
            "TL tone 9 should use double acute accent (U+030B)"
        )
    }

    func testToTL_vowelPriority() {
        let cases: [(initial: String, final: String, tone: String, expected: String)] = [
            // a takes priority
            ("k", "ai", "2", "k\u{00E1}i"),
            // oo: mark between o's
            ("k", "oo", "5", "k\u{00F4}o"),
            // e
            ("t", "e", "7", "t\u{0113}"),
            // o
            ("k", "o", "2", "k\u{00F3}"),
            // ui -> mark on i
            ("k", "ui", "3", "ku\u{00EC}"),
            // iu -> mark on u
            ("tsh", "iu", "7", "tshi\u{016B}"),
            // ng -> mark on n
            ("", "ng", "5", "n\u{0302}g"),
            // m -> mark on m
            ("", "m", "7", "m\u{0304}"),
        ]
        for (initial, final, tone, expected) in cases {
            let result = TaigiPhonetics.toTL(initial: initial, final: final, tone: tone)
            XCTAssertEqual(result, expected, "toTL(\(initial), \(final), \(tone))")
        }
    }

    func testToTL_noInitial() {
        let result = TaigiPhonetics.toTL(initial: "", final: "a", tone: "2")
        XCTAssertEqual(result, "\u{00E1}")  // á
    }

    func testToTL_complexFinal_iang() {
        // a takes priority in iang
        let result = TaigiPhonetics.toTL(initial: "k", final: "iang", tone: "5")
        XCTAssertEqual(result, "ki\u{00E2}ng")  // kiâng
    }

    // MARK: - G. toPOJ

    func testToPOJ_initialConversion() {
        // ts -> ch
        let result1 = TaigiPhonetics.toPOJ(initial: "ts", final: "u", tone: "2")
        XCTAssertEqual(result1, "ch\u{00FA}")  // chú

        // tsh -> chh
        let result2 = TaigiPhonetics.toPOJ(initial: "tsh", final: "iu", tone: "7")
        XCTAssertEqual(result2, "chhi\u{016B}")  // chhiū
    }

    func testToPOJ_finalConversions() {
        // nn -> ⁿ
        let annResult = TaigiPhonetics.toPOJ(initial: "k", final: "ann", tone: "2")
        XCTAssertTrue(annResult.contains("\u{207F}"), "nn should become ⁿ in POJ: got \(annResult)")

        // oo -> o͘
        let ooResult = TaigiPhonetics.toPOJ(initial: "k", final: "oo", tone: "1")
        XCTAssertTrue(
            ooResult.unicodeScalars.contains("\u{0358}"),
            "oo should become o͘ in POJ: got \(ooResult)"
        )

        // ua -> oa
        let uaResult = TaigiPhonetics.toPOJ(initial: "k", final: "ua", tone: "1")
        XCTAssertTrue(uaResult.contains("oa"), "ua should become oa in POJ: got \(uaResult)")

        // ue -> oe
        let ueResult = TaigiPhonetics.toPOJ(initial: "k", final: "ue", tone: "1")
        XCTAssertTrue(ueResult.contains("oe"), "ue should become oe in POJ: got \(ueResult)")

        // ing -> eng
        let ingResult = TaigiPhonetics.toPOJ(initial: "p", final: "ing", tone: "1")
        XCTAssertEqual(ingResult, "peng")

        // ik -> ek
        let ikResult = TaigiPhonetics.toPOJ(initial: "p", final: "ik", tone: "4")
        XCTAssertTrue(ikResult.contains("ek"), "ik should become ek in POJ: got \(ikResult)")
    }

    func testToPOJ_toneMarks() {
        let cases: [(initial: String, final: String, tone: String)] = [
            ("k", "a", "1"),  // no mark
            ("k", "a", "2"),  // acute
            ("k", "a", "5"),  // circumflex
            ("k", "a", "7"),  // macron
        ]

        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "1"), "ka")
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "2"), "k\u{00E1}")
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "5"), "k\u{00E2}")
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "7"), "k\u{0101}")
    }

    func testToPOJ_tone9_breve() {
        let result = TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "9")
        // POJ tone 9 uses breve: ă (U+0103)
        XCTAssertTrue(result.contains("\u{0103}"), "POJ tone 9 should use breve: got \(result)")
    }

    func testToPOJ_triphthong_iau_markOnA() {
        let result = TaigiPhonetics.toPOJ(initial: "", final: "iau", tone: "5")
        XCTAssertTrue(result.contains("\u{00E2}"), "iau mark should be on a: got \(result)")
    }

    func testToPOJ_diphthong_ai_markOnFirst() {
        let result = TaigiPhonetics.toPOJ(initial: "k", final: "ai", tone: "2")
        XCTAssertEqual(result, "k\u{00E1}i")
    }

    func testToPOJ_nonTsInitialUnchanged() {
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "k", final: "a", tone: "2"), "k\u{00E1}")
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "p", final: "a", tone: "2"), "p\u{00E1}")
        XCTAssertEqual(TaigiPhonetics.toPOJ(initial: "h", final: "a", tone: "2"), "h\u{00E1}")
    }

    // MARK: - H. convertSyllable

    func testConvertSyllable_tlMode() {
        // Tones 2-9 (except 4) get marks
        let result = TaigiPhonetics.convertSyllable("ka2", mode: .tl)
        XCTAssertEqual(result, "k\u{00E1}")

        // Tone 1 keeps digit
        XCTAssertEqual(TaigiPhonetics.convertSyllable("ka1", mode: .tl), "ka1")
        // Tone 4 keeps digit
        XCTAssertEqual(TaigiPhonetics.convertSyllable("kah4", mode: .tl), "kah4")
    }

    func testConvertSyllable_pojMode() {
        let result = TaigiPhonetics.convertSyllable("ka2", mode: .poj)
        XCTAssertEqual(result, "k\u{00E1}")
    }

    func testConvertSyllable_englishPassthrough() {
        XCTAssertEqual(TaigiPhonetics.convertSyllable("hello2", mode: .english), "hello2")
    }

    func testConvertSyllable_casePreservation() {
        // Uppercase first letter should be preserved
        let result = TaigiPhonetics.convertSyllable("Ka2", mode: .tl)
        XCTAssertTrue(result.first?.isUppercase == true, "Case should be preserved: got \(result)")
    }

    func testConvertSyllable_noToneDigit() {
        // No trailing digit -> returned as-is
        XCTAssertEqual(TaigiPhonetics.convertSyllable("ka", mode: .tl), "ka")
    }

    func testConvertSyllable_invalidSyllable() {
        // Invalid syllable returned as-is
        XCTAssertEqual(TaigiPhonetics.convertSyllable("xyz5", mode: .tl), "xyz5")
    }

    // MARK: - I. convertToToneMarks

    func testConvertToToneMarks_multiSyllable() {
        let result = TaigiPhonetics.convertToToneMarks("ka2-lang5", mode: .tl)
        XCTAssertEqual(result, "k\u{00E1}-l\u{00E2}ng")
    }

    func testConvertToToneMarks_mixedTones() {
        let result = TaigiPhonetics.convertToToneMarks("gua2-si7-hak8-sing1", mode: .tl)
        // gua2 -> guá, si7 -> sī, hak8 -> ha̍k (keep digit for 1)
        XCTAssertTrue(result.contains("gu\u{00E1}"), "gua2 should produce guá")
        XCTAssertTrue(result.contains("s\u{012B}"), "si7 should produce sī")
    }

    func testConvertToToneMarks_emptyString() {
        XCTAssertEqual(TaigiPhonetics.convertToToneMarks("", mode: .tl), "")
    }

    func testConvertToToneMarks_singleSyllable() {
        let result = TaigiPhonetics.convertToToneMarks("ho2", mode: .tl)
        XCTAssertEqual(result, "h\u{00F3}")
    }

    // MARK: - J. tlDisplayToPOJDisplay

    func testTlDisplayToPOJDisplay_tsConversion() {
        // TL ts -> POJ ch
        let result = TaigiPhonetics.tlDisplayToPOJDisplay("ts\u{00E1}i")  // tsái
        XCTAssertTrue(result.hasPrefix("ch"), "ts should become ch: got \(result)")
    }

    func testTlDisplayToPOJDisplay_casePreservation() {
        let result = TaigiPhonetics.tlDisplayToPOJDisplay("T\u{00E2}i")  // Tâi
        XCTAssertTrue(result.first?.isUppercase == true, "Case should be preserved: got \(result)")
    }

    func testTlDisplayToPOJDisplay_ooHandling() {
        // TL oo -> POJ o͘
        let result = TaigiPhonetics.tlDisplayToPOJDisplay("h\u{00F4}o")  // hôo
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{0358}" },
            "oo should become o͘ in POJ: got \(result)"
        )
    }

    func testTlDisplayToPOJDisplay_hyphenatedMultiSyllable() {
        let result = TaigiPhonetics.tlDisplayToPOJDisplay("t\u{00E2}i-g\u{00ED}")  // tâi-gí
        XCTAssertTrue(result.contains("-"), "Hyphens should be preserved")
    }

    func testTlDisplayToPOJDisplay_empty() {
        XCTAssertEqual(TaigiPhonetics.tlDisplayToPOJDisplay(""), "")
    }
}
