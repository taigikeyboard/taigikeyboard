import XCTest
@testable import TaigiKeyboard

/// TPSConverter unit tests
/// Ported from references/taigi-converter/tests/zhuyin.test.js
///
/// Mapping: ref toZhuyin -> iOS toTPS, ref fromZhuyin -> iOS toTL, ref isZhuyin -> iOS containsTPS
///
/// Known intentional differences from reference:
/// - iOS does not add trailing space for tone 1 (ref uses space as tone 1 marker)
/// - iOS uses U+31B6 (ㆶ) for k-checked-tone; ref uses U+31BB (ㆻ)
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
        // iOS uses U+31B6 (ㆶ) instead of ref's U+31BB (ㆻ)
        XCTAssertEqual(TPSConverter.toTPS("kak4"), "ㄍㄚㆶ")
    }

    func testToTPS_stopTone_h4() {
        // ref: toZhuyin("kah4") = "ㄍㄚㆷ"
        XCTAssertEqual(TPSConverter.toTPS("kah4"), "ㄍㄚㆷ")
    }

    func testToTPS_tone_p8() {
        // ref: toZhuyin("kap8") = "ㄍㄚㆴ̇"
        XCTAssertEqual(TPSConverter.toTPS("kap8"), "ㄍㄚㆴ̇")
    }

    func testToTPS_tone8_nonStop() {
        // ref: toZhuyin("a8") = "ㄚ\u0307"
        let result = TPSConverter.toTPS("a8")
        XCTAssertTrue(
            result.unicodeScalars.contains("\u{0307}"),
            "Non-stop tone 8 should produce combining dot above: got \(result)"
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
            "toTPS(\"ka1\") should have no tone mark: got \(t1)"
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
        XCTAssertTrue(kap8.hasSuffix("ㆴ̇"), "toTPS(\"kap8\") should end with ㆴ̇: got \(kap8)")
        let kah8 = TPSConverter.toTPS("kah8")
        XCTAssertTrue(kah8.hasSuffix("ㆷ̇"), "toTPS(\"kah8\") should end with ㆷ̇: got \(kah8)")
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
        // iOS uses U+31B6 (ㆶ) instead of ref's U+31BB (ㆻ)
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆶ"), "kak4")
    }

    func testToTL_stopTone_h4() {
        // ref: fromZhuyin("ㄍㄚㆷ") = "kah4"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷ"), "kah4")
    }

    func testToTL_stopTone_p8() {
        // ref: fromZhuyin("ㄍㄚㆴ̇") = "kap8"
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆴ̇"), "kap8")
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
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆴ̇"), "kap8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆵ̇"), "kat8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆶ̇"), "kak8")
        XCTAssertEqual(TPSConverter.toTL("ㄍㄚㆷ̇"), "kah8")
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
            ("ㄍㄨㄚˋ", "kua2"),   // ㄍ→k (voiceless velar)
            ("ㄉㄧㄠˊ", "tiau5"),
            ("ㄍㄚㆶ̇", "kak8"),
        ]
        for (tps, expectedTl) in cases {
            let tl = TPSConverter.toTL(tps)
            XCTAssertEqual(tl, expectedTl, "toTL(\(tps))")
            let backToTps = TPSConverter.toTPS(tl)
            XCTAssertEqual(backToTps, tps, "Round-trip: toTPS(toTL(\(tps))) should equal \(tps)")
        }
    }

    // MARK: - Empty Input

    func testToTL_empty() {
        XCTAssertEqual(TPSConverter.toTL(""), "")
    }

    func testToTPS_empty() {
        XCTAssertEqual(TPSConverter.toTPS(""), "")
    }
}
