@testable import TaigiKeyboard
import XCTest

/// TPSConverter unit tests
/// Ported from references/taigi-converter/tests/zhuyin.test.js
///
/// Mapping: ref toZhuyin -> iOS toTPS, ref fromZhuyin -> iOS toTL, ref isZhuyin -> iOS containsTPS
///
/// Known intentional differences from reference:
/// - iOS does not add trailing space for tone 1 (ref uses space as tone 1 marker)
/// - iOS now uses U+31BB (ㆻ) for k-checked-tone, aligned with ref and Unicode 13.0
/// - iOS does not support punctuation conversion or encode-safe mode
/// - iOS toTL does not insert hyphens between multi-syllable output
final class TPSConverterTests: XCTestCase {
    // MARK: - containsTPS (ref: isZhuyin)

    func testContainsTPS_tpsChars() {
        // ref: "detects TPS characters" — isZhuyin("\u3105\u311a ")
        XCTAssertTrue(TPSConverter.containsTPS("ㄅㄚ"), "containsTPS(\"ㄅㄚ\") should be true")
    }

    func testContainsTPS_latinOnly() {
        // ref: "rejects plain latin" — !isZhuyin("ka2")
        XCTAssertFalse(TPSConverter.containsTPS("ka2"), "containsTPS(\"ka2\") should be false")
    }

    func testContainsTPS_empty() {
        // ref: "rejects empty string" — !isZhuyin("")
        XCTAssertFalse(TPSConverter.containsTPS(""), "containsTPS(\"\") should be false")
    }

    func testContainsTPS_mixed() {
        XCTAssertTrue(TPSConverter.containsTPS("abcㄅdef"), "containsTPS(\"abcㄅdef\") should be true")
    }

    // MARK: - toTPS (ref: toZhuyin)

    func testToTPS_simpleSyllable() {
        // ref: toZhuyin("pa1") = "ㄅㄚ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("pa1"), "ㄅㄚ")
    }

    func testToTPS_aspirated() {
        // ref: toZhuyin("pha1") = "ㄆㄚ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("pha1"), "ㄆㄚ")
    }

    func testToTPS_tone2() {
        // ref: toZhuyin("ka2") = "ㄍㄚˋ"
        XCTAssertEqual(TPSConverter.toTPS("ka2"), "ㄍㄚˋ")
    }

    func testToTPS_tone3() {
        // ref: toZhuyin("ka3") = "ㄍㄚ˪"
        XCTAssertEqual(TPSConverter.toTPS("ka3"), "ㄍㄚ˪")
    }

    func testToTPS_tone5() {
        // ref: toZhuyin("ka5") = "ㄍㄚˊ"
        XCTAssertEqual(TPSConverter.toTPS("ka5"), "ㄍㄚˊ")
    }

    func testToTPS_tone7() {
        // ref: toZhuyin("ka7") = "ㄍㄚ˫"
        XCTAssertEqual(TPSConverter.toTPS("ka7"), "ㄍㄚ˫")
    }

    func testToTPS_stopTone_p4() {
        // ref: toZhuyin("kap4") = "ㄍㄚㆴ"
        XCTAssertEqual(TPSConverter.toTPS("kap4"), "ㄍㄚㆴ")
    }

    func testToTPS_stopTone_t4() {
        // ref: toZhuyin("kat4") = "ㄍㄚㆵ"
        XCTAssertEqual(TPSConverter.toTPS("kat4"), "ㄍㄚㆵ")
    }

    func testToTPS_stopTone_k4() {
        // ref: toZhuyin("kak4") = "ㄍㄚ\u31bb"
        // Now aligned with ref: both use U+31BB (ㆻ)
        XCTAssertEqual(TPSConverter.toTPS("kak4"), "ㄍㄚㆻ")
    }

    func testToTPS_stopTone_h4() {
        // ref: toZhuyin("kah4") = "ㄍㄚㆷ"
        XCTAssertEqual(TPSConverter.toTPS("kah4"), "ㄍㄚㆷ")
    }

    func testToTPS_tone_p8() {
        // ref: toZhuyin("kap8") = "ㄍㄚㆴ˙"
        XCTAssertEqual(TPSConverter.toTPS("kap8"), "ㄍㄚㆴ˙")
    }

    func testToTPS_tone8_nonStop() {
        // ref: toZhuyin("a8") = "ㄚ˙" (U+02D9)
        let result = TPSConverter.toTPS("a8")
        XCTAssertTrue(
            result.unicodeScalars.contains("\u{02D9}"),
            "Non-stop tone 8 should produce U+02D9 DOT ABOVE: got \(result)",
        )
    }

    func testToTPS_tone9() {
        // ref: toZhuyin("a9") = "ㄚˆ"
        XCTAssertEqual(TPSConverter.toTPS("a9"), "ㄚˆ")
    }

    func testToTPS_standaloneM() {
        // ref: toZhuyin("m1") = "ㆬ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("m1"), "ㆬ")
    }

    func testToTPS_standaloneNg() {
        // ref: toZhuyin("ng1") = "ㆭ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("ng1"), "ㆭ")
    }

    func testToTPS_nasalVowel() {
        // ref: toZhuyin("ann1") = "ㆩ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("ann1"), "ㆩ")
    }

    func testToTPS_multiCharConsonant_tshi() {
        // ref: toZhuyin("tshi1") = "ㄑㄧ " — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTPS("tshi1"), "ㄑㄧ")
    }

    func testToTPS_nasalizedInn_afterPalatalizedTs() {
        // ref: toZhuyin("tsinn5") = "ㄐㆪˊ"
        XCTAssertEqual(TPSConverter.toTPS("tsinn5"), "ㄐㆪˊ")
    }

    func testToTPS_nasalizedInn_afterPalatalizedTsh() {
        // ref: toZhuyin("tshinn5") = "ㄑㆪˊ"
        XCTAssertEqual(TPSConverter.toTPS("tshinn5"), "ㄑㆪˊ")
    }

    // MARK: - toTPS: Additional Coverage

    func testToTPS_tone6() {
        XCTAssertEqual(TPSConverter.toTPS("ka6"), "ㄍㄚˇ")
    }

    func testToTPS_tone1And4_noMark() {
        let t1 = TPSConverter.toTPS("ka1")
        XCTAssertFalse(
            t1.contains("ˋ") || t1.contains("˪") || t1.contains("ˊ") || t1.contains("˫"),
            "toTPS(\"ka1\") should have no tone mark: got \(t1)",
        )
    }

    func testToTPS_hyphenBecomesSpace() {
        let result = TPSConverter.toTPS("gua2-gua2")
        XCTAssertTrue(result.contains(" "), "Hyphen should become space: got \(result)")
    }

    func testToTPS_mWithVowel_staysConsonant() {
        let result = TPSConverter.toTPS("ma2")
        XCTAssertTrue(result.hasPrefix("ㄇ"), "m with vowel should stay ㄇ: got \(result)")
    }

    func testToTPS_ngWithVowel_staysConsonant() {
        let result = TPSConverter.toTPS("nga2")
        XCTAssertTrue(result.hasPrefix("ㄫ"), "ng with vowel should stay ㄫ: got \(result)")
    }

    func testToTPS_standaloneM_withTone() {
        XCTAssertEqual(TPSConverter.toTPS("m7"), "ㆬ˫")
    }

    func testToTPS_standaloneNg_withTone() {
        XCTAssertEqual(TPSConverter.toTPS("ng5"), "ㆭˊ")
    }

    func testToTPS_palatalizedNonNasalized_notAffected() {
        let result = TPSConverter.toTPS("tsia2")
        XCTAssertTrue(result.hasPrefix("ㄐㄧ"), "Non-nasalized should keep ㄧ: got \(result)")
    }

    func testToTPS_tone8_stop_unaffected() {
        let kap8 = TPSConverter.toTPS("kap8")
        XCTAssertTrue(kap8.hasSuffix("ㆴ˙"), "toTPS(\"kap8\") should end with ㆴ˙: got \(kap8)")
        let kah8 = TPSConverter.toTPS("kah8")
        XCTAssertTrue(kah8.hasSuffix("ㆷ˙"), "toTPS(\"kah8\") should end with ㆷ˙: got \(kah8)")
    }

    // MARK: - toTPS: o → oo before stop tone (ref: toZhuyin post-processing)

    func testToTPS_oBecomesOO_beforeStopK4() {
        // ref: toZhuyin("ok4") — vowel ㄛ should become ㆦ before checked k
        XCTAssertEqual(TPSConverter.toTPS("ok4"), "ㆦㆻ")
    }

    func testToTPS_oBecomesOO_beforeStopP4() {
        // ref: toZhuyin("op4") — vowel ㄛ should become ㆦ before checked p
        XCTAssertEqual(TPSConverter.toTPS("op4"), "ㆦㆴ")
    }

    func testToTPS_oBecomesOO_beforeStopT4() {
        // ref: toZhuyin("ot4") — vowel ㄛ should become ㆦ before checked t
        XCTAssertEqual(TPSConverter.toTPS("ot4"), "ㆦㆵ")
    }

    func testToTPS_oBecomesOO_withConsonant() {
        // ref: toZhuyin("bok4") — b + o + k4 → ㆠㆦㆻ
        XCTAssertEqual(TPSConverter.toTPS("bok4"), "ㆠㆦㆻ")
    }

    func testToTPS_oStaysO_beforeStopH4() {
        // h4 should NOT trigger o→oo (ref only applies to p/t/k)
        XCTAssertEqual(TPSConverter.toTPS("oh4"), "ㄛㆷ")
    }

    func testToTPS_ooStaysOO_beforeNonStop() {
        // oo with non-stop tone should stay as ㆦ
        XCTAssertEqual(TPSConverter.toTPS("oo2"), "ㆦˋ")
    }

    // MARK: - toTL (ref: fromZhuyin)

    func testToTL_ka2() {
        // ref: fromZhuyin("ㄍㄚˋ") = "ka2"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚˋ"), "ka2")
    }

    func testToTL_tone1_noMark() {
        // ref: fromZhuyin("ㄍㄚ ") = "ka1" — uses trailing space as tone 1 marker
        // iOS difference: no trailing space convention, implicit tone 1 without digit
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚ"), "ka")
    }

    func testToTL_tone3() {
        // ref: fromZhuyin("ㄍㄚ˪") = "ka3"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚ˪"), "ka3")
    }

    func testToTL_tone5() {
        // ref: fromZhuyin("ㄍㄚˊ") = "ka5"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚˊ"), "ka5")
    }

    func testToTL_tone7() {
        // ref: fromZhuyin("ㄍㄚ˫") = "ka7"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚ˫"), "ka7")
    }

    func testToTL_stopTone_p4() {
        // ref: fromZhuyin("ㄍㄚㆴ") = "kap4"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆴ"), "kap4")
    }

    func testToTL_stopTone_t4() {
        // ref: fromZhuyin("ㄍㄚㆵ") = "kat4"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆵ"), "kat4")
    }

    func testToTL_stopTone_k4() {
        // ref: fromZhuyin("ㄍㄚ\u31bb") = "kak4"
        // Now aligned with ref: both use U+31BB (ㆻ)
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆻ"), "kak4")
    }

    func testToTL_stopTone_h4() {
        // ref: fromZhuyin("ㄍㄚㆷ") = "kah4"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷ"), "kah4")
    }

    func testToTL_stopTone_p8() {
        // ref: fromZhuyin("ㄍㄚㆴ˙") = "kap8"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆴ˙"), "kap8")
    }

    func testToTL_tone9() {
        // ref: fromZhuyin("ㄚˆ") = "a9"
        XCTAssertEqual(TPSConverter.toTL("ㄚˆ"), "a9")
    }

    func testToTL_standaloneM() {
        // ref: fromZhuyin("ㆬ ") = "m1"
        // iOS difference: no trailing space convention, so no "1" appended
        XCTAssertEqual(TPSConverter.toTL("ㆬ"), "m")
    }

    func testToTL_standaloneNg() {
        // ref: fromZhuyin("ㆭ ") = "ng1"
        XCTAssertEqual(TPSConverter.toTL("ㆭ"), "ng")
    }

    func testToTL_ngAsVowel() {
        // ref: fromZhuyin("ㆭˊ") = "ng5"
        XCTAssertEqual(TPSConverter.toTL("ㆭˊ"), "ng5")
    }

    func testToTL_nasalVowel_ann() {
        // ref: fromZhuyin("ㆩ ") = "ann1" — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTL("ㆩ"), "ann")
    }

    func testToTL_multiCharInitial_tshi() {
        // ref: fromZhuyin("ㄑㄧ ") = "tshi1" — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTL("ㄑㄧ"), "tshi")
    }

    func testToTL_compoundVowel_iau() {
        // ref: fromZhuyin("ㄉㄧㄠ ") = "tiau1" — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTL("ㄉㄧㄠ"), "tiau")
    }

    func testToTL_aspirated_ph() {
        // ref: fromZhuyin("ㄆㄚ ") = "pha1" — adapted: no trailing space
        XCTAssertEqual(TPSConverter.toTL("ㄆㄚ"), "pha")
    }

    func testToTL_palatalizedNasalized_tsinn() {
        // ref: fromZhuyin("ㄐㆪˊ") = "tsinn5"
        XCTAssertEqual(TPSConverter.toTL("ㄐㆪˊ"), "tsinn5")
    }

    func testToTL_palatalizedNasalized_tshinn() {
        // ref: fromZhuyin("ㄑㆪˊ") = "tshinn5"
        XCTAssertEqual(TPSConverter.toTL("ㄑㆪˊ"), "tshinn5")
    }

    func testToTL_tone6() {
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚˇ"), "ka6")
    }

    // MARK: - toTL: Full Consonant Coverage

    func testToTL_basicConsonants() {
        let cases: [(tps: String, expected: String)] = [
            ("ㄅㄚ", "pa"),
            ("ㄆㄚ", "pha"),
            ("ㄇㄚ", "ma"),
            ("ㆠㄚ", "ba"),
            ("ㄉㄚ", "ta"),
            ("ㄊㄚ", "tha"),
            ("ㄋㄚ", "na"),
            ("ㄌㄚ", "la"),
            ("ㄍㄚ", "ka"),
            ("ㄎㄚ", "kha"),
            ("ㄫㄚ", "nga"),
            ("ㆣㄚ", "ga"),
            ("ㄏㄚ", "ha"),
            ("ㄗㄚ", "tsa"),
            ("ㄘㄚ", "tsha"),
            ("ㄙㄚ", "sa"),
            ("ㆡㄚ", "ja"),
        ]
        for (tps, expected) in cases {
            XCTAssertEqual(TPSConverter.toTL(tps), expected, "toTL(\(tps))")
        }
    }

    func testToTL_compoundConsonants() {
        XCTAssertEqual(TPSConverter.toTL("ㄐㄧㄚ"), "tsia")
        XCTAssertEqual(TPSConverter.toTL("ㄑㄧㄚ"), "tshia")
        XCTAssertEqual(TPSConverter.toTL("ㄒㄧㄚ"), "sia")
    }

    // MARK: - toTL: Full Vowel Coverage

    func testToTL_singleVowels() {
        XCTAssertEqual(TPSConverter.toTL("ㄚ"), "a")
        XCTAssertEqual(TPSConverter.toTL("ㆤ"), "e")
        XCTAssertEqual(TPSConverter.toTL("ㄧ"), "i")
        XCTAssertEqual(TPSConverter.toTL("ㄛ"), "o")
        XCTAssertEqual(TPSConverter.toTL("ㄨ"), "u")
    }

    func testToTL_compoundVowels() {
        XCTAssertEqual(TPSConverter.toTL("ㆦ"), "oo")
        XCTAssertEqual(TPSConverter.toTL("ㄞ"), "ai")
        XCTAssertEqual(TPSConverter.toTL("ㄠ"), "au")
        XCTAssertEqual(TPSConverter.toTL("ㄤ"), "ang")
        XCTAssertEqual(TPSConverter.toTL("ㆲ"), "ong")
    }

    func testToTL_nasalVowels() {
        XCTAssertEqual(TPSConverter.toTL("ㆩ"), "ann")
        XCTAssertEqual(TPSConverter.toTL("ㆥ"), "enn")
        XCTAssertEqual(TPSConverter.toTL("ㆪ"), "inn")
        XCTAssertEqual(TPSConverter.toTL("ㆧ"), "onn")
        XCTAssertEqual(TPSConverter.toTL("ㆫ"), "unn")
    }

    func testToTL_syllabicConsonants() {
        XCTAssertEqual(TPSConverter.toTL("ㆬ"), "m")
        XCTAssertEqual(TPSConverter.toTL("ㆭ"), "ng")
    }

    func testToTL_checkedTone8_allFinals() {
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆴ˙"), "kap8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆵ˙"), "kat8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆻ˙"), "kak8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷ˙"), "kah8")
    }

    func testToTL_keyboardInput_checkedTone8() {
        // Keyboard outputs ˙ (U+02D9) — verify full syllable converts correctly
        // This was the original bug: ㄍㄚㆻ˙ produced "kak48" instead of "kak8"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆻ\u{02D9}"), "kak8")
    }

    // MARK: - toTL: Tone 8 standalone (˙ U+02D9)

    func testToTL_tone8_standalone_dotAbove() {
        // "˙" (U+02D9 DOT ABOVE) should map to tone 8
        XCTAssertEqual(TPSConverter.toTL("ㄚ˙"), "a8")
    }

    func testToTL_tone8_standalone_withConsonant() {
        // Full syllable with standalone tone 8
        XCTAssertEqual(TPSConverter.toTL("ㄌㄚ˙"), "la8")
    }

    // MARK: - toTL: oo → o before stop tone (ref: fromZhuyin post-processing)

    func testToTL_oo_becomesO_beforeStopK4() {
        // ref: fromZhuyin — if syllable has "oo" and tone is k4, replace oo with o
        XCTAssertEqual(TPSConverter.toTL("ㆦㆻ"), "ok4")
    }

    func testToTL_oo_becomesO_beforeStopP8() {
        XCTAssertEqual(TPSConverter.toTL("ㆦㆴ˙"), "op8")
    }

    func testToTL_oo_becomesO_withConsonant() {
        // "bok4" — b + oo + k4 → should output "bok4" not "book4"
        XCTAssertEqual(TPSConverter.toTL("ㆠㆦㆻ"), "bok4")
    }

    func testToTL_oo_staysOO_beforeStopH4() {
        // h4 should NOT trigger oo→o (ref regex only matches [ptk])
        XCTAssertEqual(TPSConverter.toTL("ㆦㆷ"), "ooh4")
    }

    func testToTL_oo_staysOO_nonStop() {
        // oo with non-stop tone should remain oo
        XCTAssertEqual(TPSConverter.toTL("ㆦˋ"), "oo2")
    }

    // MARK: - toTLMultiSyllable

    func testToTLMultiSyllable_spaceSeparated() {
        // ㄍ→k (voiceless velar), not g (ㆣ is voiced velar)
        let result = TPSConverter.toTLMultiSyllable("ㄍㄨㄚˋ ㄙㄨˊ")
        XCTAssertEqual(result, "kua2 su5")
    }

    func testToTLMultiSyllable_singleSyllable() {
        XCTAssertEqual(TPSConverter.toTLMultiSyllable("ㄍㄚˋ"), "ka2")
    }

    // MARK: - Round-Trip: TPS -> TL -> TPS

    func testRoundTrip_tpsToTlToTps() {
        let cases: [(tps: String, expectedTl: String)] = [
            ("ㄍㄨㄚˋ", "kua2"), // ㄍ→k (voiceless velar)
            ("ㄉㄧㄠˊ", "tiau5"),
            ("ㄍㄚㆻ˙", "kak8"),
        ]
        for (tps, expectedTl) in cases {
            let tl = TPSConverter.toTL(tps)
            XCTAssertEqual(tl, expectedTl, "toTL(\(tps))")
            let backToTps = TPSConverter.toTPS(tl)
            XCTAssertEqual(backToTps, tps, "Round-trip: toTPS(toTL(\(tps))) should equal \(tps)")
        }
    }

    // MARK: - Syllable Boundary: ㄏ (initial) vs ㆷ (entering tone coda)

    func testToTL_syllableBoundary_initialAfterVowel() {
        // ㄍㄛㄏ: ㄏ is initial h-, NOT coda -h. Must NOT match "koh" (閣).
        // ㄏ (U+310F) is in consonants table, ㆷ (U+31B7) is in tones table.
        XCTAssertEqual(TPSConverter.toTL("ㄍㄛㄏ"), "ko h",
                       "ㄍㄛㄏ should insert space before ㄏ (initial), not produce 'koh'")
    }

    func testToTL_syllableBoundary_enteringToneCoda() {
        // ㄍㄛㆷ: ㆷ is entering tone -h coda. Should produce "koh4".
        XCTAssertEqual(TPSConverter.toTL("ㄍㄛㆷ"), "koh4",
                       "ㄍㄛㆷ should match koh4 via entering tone coda")
    }

    func testToTL_syllableBoundary_multiSyllable() {
        // ㄍㄛㄏㄧㆲˊ: ko + hiong5, not "kohiong5"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄛㄏㄧㆲˊ"), "ko hiong5",
                       "Consonant after vowel should start new syllable")
    }

    func testToTL_syllableBoundary_allCheckedCodas() {
        // All entering tone codas should work without space insertion
        let cases: [(input: String, expected: String, desc: String)] = [
            ("ㄍㄚㆴ", "kap4", "ㆴ = -p coda"),
            ("ㄍㄚㆵ", "kat4", "ㆵ = -t coda"),
            ("ㄍㄚㆻ", "kak4", "ㆻ = -k coda"),
            ("ㄍㄚㆷ", "kah4", "ㆷ = -h coda"),
        ]
        for (input, expected, desc) in cases {
            XCTAssertEqual(TPSConverter.toTL(input), expected, desc)
        }
    }

    func testToTL_syllableBoundary_consecutiveInitials() {
        // ㄍˋㄏㄧㆲˊ: tone resets syllable, so ㄏ starts cleanly
        // Note: ㄍ is consonant-only when ˋ hits → "k 2", then ㄏ starts new syllable
        XCTAssertEqual(TPSConverter.toTL("ㄍˋㄏㄧㆲˊ"), "k 2 hiong5")
    }

    // MARK: - Syllable Boundary: ㄫ (initial) vs ㆭ (syllabic ng)

    func testToTL_syllableBoundary_syllabicNg() {
        // ㆭˊ: ㆭ is syllabic ng (vowel table) → "ng5" → matches 黃
        XCTAssertEqual(TPSConverter.toTL("ㆭˊ"), "ng5",
                       "ㆭˊ (syllabic ng + tone) should produce 'ng5'")
    }

    func testToTL_syllableBoundary_initialNg() {
        // ㄫˊ: ㄫ is initial ng (consonant table) → should NOT produce "ng5"
        // Consonant-only + tone → space separates them
        XCTAssertEqual(TPSConverter.toTL("ㄫˊ"), "ng 5",
                       "ㄫˊ (initial ng + tone) should NOT match syllabic ng5 (黃)")
    }

    func testToTL_syllableBoundary_syllabicM() {
        // ㆬˋ: ㆬ is syllabic m (vowel table) → "m2"
        XCTAssertEqual(TPSConverter.toTL("ㆬˋ"), "m2",
                       "ㆬˋ (syllabic m + tone) should produce 'm2'")
    }

    func testToTL_syllableBoundary_initialM() {
        // ㄇˋ: ㄇ is initial m (consonant table) → should NOT produce "m2"
        XCTAssertEqual(TPSConverter.toTL("ㄇˋ"), "m 2",
                       "ㄇˋ (initial m + tone) should NOT match syllabic m2")
    }

    func testToTL_syllableBoundary_initialNgWithVowel() {
        // ㄫㄚˋ: normal syllable — initial ng + vowel a + tone 2
        XCTAssertEqual(TPSConverter.toTL("ㄫㄚˋ"), "nga2",
                       "ㄫㄚˋ should produce normal syllable 'nga2'")
    }

    // MARK: - Palatalized vs Non-Palatalized Affricates

    func testToTL_palatalized_tshi() {
        // ㄑㄧ˪: compound initial ㄑㄧ → "tshi3" → matches 試
        XCTAssertEqual(TPSConverter.toTL("ㄑㄧ˪"), "tshi3",
                       "ㄑㄧ˪ (palatalized compound) should produce 'tshi3'")
    }

    func testToTL_nonPalatalized_tsh_i() {
        // ㄘㄧ˪: ㄘ + ㄧ is invalid TPS → should NOT produce "tshi3"
        XCTAssertEqual(TPSConverter.toTL("ㄘㄧ˪"), "tsh i3",
                       "ㄘㄧ˪ (non-palatalized + ㄧ) should NOT match 'tshi3' (試)")
    }

    func testToTL_palatalized_allPairs() {
        // All 4 palatalized compound initials
        let cases: [(input: String, expected: String, desc: String)] = [
            ("ㄐㄧ˪", "tsi3", "ㄐㄧ = tsi"),
            ("ㄑㄧ˪", "tshi3", "ㄑㄧ = tshi"),
            ("ㄒㄧ˪", "si3", "ㄒㄧ = si"),
            ("ㆢㄧ˪", "ji3", "ㆢㄧ = ji"),
        ]
        for (input, expected, desc) in cases {
            XCTAssertEqual(TPSConverter.toTL(input), expected, desc)
        }
    }

    func testToTL_nonPalatalized_allPairs() {
        // All 4 non-palatalized + ㄧ (invalid TPS, should insert space)
        let cases: [(input: String, expected: String, desc: String)] = [
            ("ㄗㄧ˪", "ts i3", "ㄗ + ㄧ invalid"),
            ("ㄘㄧ˪", "tsh i3", "ㄘ + ㄧ invalid"),
            ("ㄙㄧ˪", "s i3", "ㄙ + ㄧ invalid"),
            ("ㆡㄧ˪", "j i3", "ㆡ + ㄧ invalid"),
        ]
        for (input, expected, desc) in cases {
            XCTAssertEqual(TPSConverter.toTL(input), expected, desc)
        }
    }

    func testToTL_nonPalatalized_otherVowels() {
        // Non-palatalized affricates with non-ㄧ vowels should work normally
        XCTAssertEqual(TPSConverter.toTL("ㄘㄚˋ"), "tsha2", "ㄘ + ㄚ is valid")
        XCTAssertEqual(TPSConverter.toTL("ㄗㄨˊ"), "tsu5", "ㄗ + ㄨ is valid")
    }

    // MARK: - Empty Input

    func testToTL_empty() {
        XCTAssertEqual(TPSConverter.toTL(""), "")
    }

    func testToTPS_empty() {
        XCTAssertEqual(TPSConverter.toTPS(""), "")
    }

    // MARK: - toTPS: ing special case (ng as vowel → ㄥ after ㄧ)

    func testToTPS_ing() {
        // ing uses ㄥ (not ㆭ)
        XCTAssertEqual(TPSConverter.toTPS("ing5"), "ㄧㄥˊ")
    }

    func testToTPS_king() {
        // consonant + ing
        XCTAssertEqual(TPSConverter.toTPS("king5"), "ㄍㄧㄥˊ")
    }

    func testToTPS_ung_usesNg() {
        // non-ing ng uses ㆭ
        XCTAssertEqual(TPSConverter.toTPS("ung7"), "ㄨㆭ˫")
    }

    // MARK: - toTPS: or vowel mapping

    func testToTPS_or_default() {
        // Default: or → ㄛ
        XCTAssertEqual(TPSConverter.toTPS("or2"), "ㄛˋ", "or2 default should map to ㄛˋ")
    }

    func testToTPS_or_mapsToER() {
        // orMapsToER: or → ㄜ
        XCTAssertEqual(TPSConverter.toTPS("or2", orMapsToER: true), "ㄜˋ", "or2 with orMapsToER should map to ㄜˋ")
    }

    func testToTPS_ior_default() {
        // ior decomposes to i + or → ㄧㄛ
        XCTAssertEqual(TPSConverter.toTPS("ior2"), "ㄧㄛˋ", "ior2 default should map to ㄧㄛˋ")
    }

    func testToTPS_ior_mapsToER() {
        XCTAssertEqual(TPSConverter.toTPS("ior2", orMapsToER: true), "ㄧㄜˋ", "ior2 with orMapsToER should map to ㄧㄜˋ")
    }

    func testToTPS_orh4_default() {
        // orh4: or + checked h4 → ㄛㆷ
        XCTAssertEqual(TPSConverter.toTPS("orh4"), "ㄛㆷ", "orh4 default should map to ㄛㆷ")
    }

    func testToTPS_orh4_mapsToER() {
        XCTAssertEqual(TPSConverter.toTPS("orh4", orMapsToER: true), "ㄜㆷ", "orh4 with orMapsToER should map to ㄜㆷ")
    }

    func testToTPS_kor_default() {
        // consonant + or → ㄍㄛ
        XCTAssertEqual(TPSConverter.toTPS("kor2"), "ㄍㄛˋ", "kor2 default should map to ㄍㄛˋ")
    }

    func testToTPS_kor_mapsToER() {
        XCTAssertEqual(TPSConverter.toTPS("kor2", orMapsToER: true), "ㄍㄜˋ", "kor2 with orMapsToER should map to ㄍㄜˋ")
    }

    func testToTPS_er_unaffected() {
        // er should always map to ㄜ regardless of orMapsToER
        XCTAssertEqual(TPSConverter.toTPS("er2"), "ㄜˋ", "er2 should always be ㄜˋ")
        XCTAssertEqual(TPSConverter.toTPS("er2", orMapsToER: true), "ㄜˋ", "er2 with orMapsToER should still be ㄜˋ")
    }

    func testToTPS_o_unaffected() {
        // Plain o should be unaffected by orMapsToER
        XCTAssertEqual(TPSConverter.toTPS("o2"), "ㄛˋ", "o2 should always be ㄛˋ")
        XCTAssertEqual(TPSConverter.toTPS("o2", orMapsToER: true), "ㄛˋ", "o2 with orMapsToER should still be ㄛˋ")
    }

    // MARK: - adjustTPSInitialKey

    func testAdjustTPSInitialKey_m_atSyllableStart() {
        // Empty buffer → syllable start → ㄇ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄇ", afterRawInput: ""), "ㄇ")
    }

    func testAdjustTPSInitialKey_m_afterVowel() {
        // After ㄧ → ㆬ (final form)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄇ", afterRawInput: "ㄧ"), "ㆬ")
    }

    func testAdjustTPSInitialKey_ng_afterI() {
        // After ㄧ → ㄥ (ing special case)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄫ", afterRawInput: "ㄧ"), "ㄥ")
    }

    func testAdjustTPSInitialKey_ng_afterOtherVowel() {
        // After ㄚ → ㆭ (final form)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄫ", afterRawInput: "ㄚ"), "ㆭ")
    }

    func testAdjustTPSInitialKey_m_afterTone() {
        // After tone mark = new syllable → ㄇ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄇ", afterRawInput: "ㄇㄚˋ"), "ㄇ")
    }

    func testAdjustTPSInitialKey_m_afterCheckedFinal() {
        // After checked tone final = new syllable → ㄇ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄇ", afterRawInput: "ㄍㄚㆴ"), "ㄇ")
    }

    func testAdjustTPSInitialKey_ng_atSyllableStart() {
        // Empty buffer → syllable start → ㄫ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄫ", afterRawInput: ""), "ㄫ")
    }

    func testAdjustTPSInitialKey_n_atSyllableStart() {
        // Empty buffer → syllable start → ㄋ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: ""), "ㄋ")
    }

    func testAdjustTPSInitialKey_n_afterVowel() {
        // After ㄧ → ㄣ (final form): ㄒㄧㄋ → ㄒㄧㄣ (sin)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: "ㄒㄧ"), "ㄣ")
    }

    func testAdjustTPSInitialKey_n_afterOtherVowel() {
        // After ㄚ → ㄣ (final form)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: "ㄚ"), "ㄣ")
    }

    func testAdjustTPSInitialKey_n_afterTone() {
        // After tone mark = new syllable → ㄋ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: "ㄒㄧㄣˋ"), "ㄋ")
    }

    func testAdjustTPSInitialKey_n_afterCheckedFinal() {
        // After checked tone final = new syllable → ㄋ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: "ㄍㄚㆴ"), "ㄋ")
    }

    func testAdjustTPSInitialKey_n_afterSpace() {
        // After space = new syllable → ㄋ
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄋ", afterRawInput: "ㄚ "), "ㄋ")
    }

    // MARK: - Stop coda auto-select (ㄅ→ㆴ, ㄉ→ㆵ, ㄍ→ㆻ, ㄏ→ㆷ)

    func testAdjustTPSInitialKey_p_atSyllableStart() {
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄅ", afterRawInput: ""), "ㄅ")
    }

    func testAdjustTPSInitialKey_p_afterVowel() {
        // ㄍㄚ + ㄅ → ㆴ (kap coda)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄅ", afterRawInput: "ㄍㄚ"), "ㆴ")
    }

    func testAdjustTPSInitialKey_p_afterTone() {
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄅ", afterRawInput: "ㄚˋ"), "ㄅ")
    }

    func testAdjustTPSInitialKey_p_afterCheckedFinal() {
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄅ", afterRawInput: "ㄍㄚㆴ"), "ㄅ")
    }

    func testAdjustTPSInitialKey_t_afterVowel() {
        // ㄍㄚ + ㄉ → ㆵ (kat coda)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄉ", afterRawInput: "ㄍㄚ"), "ㆵ")
    }

    func testAdjustTPSInitialKey_k_afterVowel() {
        // ㄍㄚ + ㄍ → ㆻ (kak coda)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄍ", afterRawInput: "ㄍㄚ"), "ㆻ")
    }

    func testAdjustTPSInitialKey_h_afterVowel() {
        // ㄍㄚ + ㄏ → ㆷ (kah coda)
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄏ", afterRawInput: "ㄍㄚ"), "ㆷ")
    }

    func testAdjustTPSInitialKey_h_atSyllableStart() {
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄏ", afterRawInput: ""), "ㄏ")
    }

    func testAdjustTPSInitialKey_p_afterSpace() {
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄅ", afterRawInput: "ㄚ "), "ㄅ")
    }

    func testAdjustTPSInitialKey_nonTargetKey() {
        // Non-target key → pass through unchanged
        XCTAssertEqual(TPSConverter.adjustTPSInitialKey("ㄌ", afterRawInput: "ㄧ"), "ㄌ")
    }

    // MARK: - palatalizationReplacement

    func testPalatalization_s_beforeI() {
        // ㄙ + ㄧ → replace ㄙ with ㄒ
        XCTAssertEqual(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄙ"), "ㄒ")
    }

    func testPalatalization_ts_beforeI() {
        // ㄗ + ㄧ → replace ㄗ with ㄐ
        XCTAssertEqual(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄗ"), "ㄐ")
    }

    func testPalatalization_tsh_beforeI() {
        // ㄘ + ㄧ → replace ㄘ with ㄑ
        XCTAssertEqual(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄘ"), "ㄑ")
    }

    func testPalatalization_j_beforeI() {
        // ㆡ + ㄧ → replace ㆡ with ㆢ
        XCTAssertEqual(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㆡ"), "ㆢ")
    }

    func testPalatalization_s_beforeNasalizedI() {
        // ㄙ + ㆪ → replace ㄙ with ㄒ (nasalized i also triggers)
        XCTAssertEqual(TPSConverter.palatalizationReplacement(forIncoming: "ㆪ", lastRawChar: "ㄙ"), "ㄒ")
    }

    func testPalatalization_noTrigger_nonIVowel() {
        // ㄙ + ㄚ → nil (ㄚ is not a palatalization trigger)
        XCTAssertNil(TPSConverter.palatalizationReplacement(forIncoming: "ㄚ", lastRawChar: "ㄙ"))
    }

    func testPalatalization_noTrigger_alreadyPalatalized() {
        // ㄒ + ㄧ → nil (ㄒ is already palatalized)
        XCTAssertNil(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄒ"))
    }

    func testPalatalization_noTrigger_nonAffricate() {
        // ㄍ + ㄧ → nil (ㄍ is not an affricate)
        XCTAssertNil(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄍ"))
    }

    func testPalatalization_noTrigger_emptyRawInput() {
        // nil last char → nil
        XCTAssertNil(TPSConverter.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: nil))
    }

    // MARK: - Nasalized vowel + checked tone

    func testToTL_nasalizedVowel_checkedTone() {
        XCTAssertEqual(TPSConverter.toTL("ㄏㆯㆷ˙"), "haunnh8",
                       "ㆯ(aunn) + ㆷ˙(h8) should produce haunnh8")
        XCTAssertEqual(TPSConverter.toTL("ㄏㆯㆷ"), "haunnh4",
                       "ㆯ(aunn) + ㆷ(h4) should produce haunnh4")
        XCTAssertEqual(TPSConverter.toTL("ㆩㆷ˙"), "annh8",
                       "ㆩ(ann) + ㆷ˙(h8) should produce annh8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㆯㆷ˙"), "kaunnh8",
                       "ㄍ + ㆯ(aunn) + ㆷ˙(h8) should produce kaunnh8")
    }

    // MARK: - Multi-syllable with entering tones

    func testToTL_multiSyllable_enteringTone() {
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷㄏㄧㆲˊ"), "kah4 hiong5",
                       "Entering tone followed by new syllable should have space")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷ˙ㄍㄚㆷ˙"), "kah8 kah8",
                       "Two entering-tone syllables should be space-separated")
    }

    // MARK: - syllabicNasalReplacement

    func testSyllabicNasalReplacement_m_beforeToneMark() {
        XCTAssertEqual(TPSConverter.syllabicNasalReplacement(forIncoming: "˫", lastRawChar: "ㄇ"), "ㆬ")
        XCTAssertEqual(TPSConverter.syllabicNasalReplacement(forIncoming: "ˋ", lastRawChar: "ㄇ"), "ㆬ")
        XCTAssertEqual(TPSConverter.syllabicNasalReplacement(forIncoming: "ˊ", lastRawChar: "ㄇ"), "ㆬ")
    }

    func testSyllabicNasalReplacement_ng_beforeToneMark() {
        XCTAssertEqual(TPSConverter.syllabicNasalReplacement(forIncoming: "ˊ", lastRawChar: "ㄫ"), "ㆭ")
        XCTAssertEqual(TPSConverter.syllabicNasalReplacement(forIncoming: "˫", lastRawChar: "ㄫ"), "ㆭ")
    }

    func testSyllabicNasalReplacement_nonTone_noChange() {
        XCTAssertNil(TPSConverter.syllabicNasalReplacement(forIncoming: "ㄚ", lastRawChar: "ㄇ"),
                     "Vowel is not a tone mark")
    }

    func testSyllabicNasalReplacement_otherConsonant_noChange() {
        XCTAssertNil(TPSConverter.syllabicNasalReplacement(forIncoming: "˫", lastRawChar: "ㄍ"),
                     "ㄍ is not ㄇ/ㄫ")
    }

    func testSyllabicNasalReplacement_alreadySyllabic_noChange() {
        XCTAssertNil(TPSConverter.syllabicNasalReplacement(forIncoming: "˫", lastRawChar: "ㆬ"),
                     "ㆬ is already syllabic")
    }

    func testSyllabicNasalReplacement_nilLastChar() {
        XCTAssertNil(TPSConverter.syllabicNasalReplacement(forIncoming: "˫", lastRawChar: nil))
    }
}
