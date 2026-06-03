@testable import TaigiKeyboard
import XCTest

/// Pure-function tests for `CustomDictionaryDerivation`.
final class CustomDictionaryDerivationTests: XCTestCase {
    // MARK: - generateNotone

    func testGenerateNotone_pojNasal() {
        // ⁿ (U+207F) → nn
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saⁿ"), "sann")
    }

    func testGenerateNotone_uppercaseNasal() {
        // ᴺ (U+1D3A) → nn
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saᴺ"), "sann")
    }

    func testGenerateNotone_toneMarksStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("hó"), "ho")
    }

    func testGenerateNotone_multiSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("gâu-tsá"), "gautsa")
    }

    func testGenerateNotone_spaceStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("lí hó"), "liho")
    }

    func testGenerateNotone_digitsStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("ho2"), "ho")
    }

    func testGenerateNotone_numericMultiSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("gau5-tsa2"), "gautsa")
    }

    func testGenerateNotone_mixedNasalAndDigit() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saⁿ2"), "sann")
    }

    // MARK: - generateAbbrev

    func testGenerateAbbrev_twoSyllables() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("gâu-tsá"), "gt")
    }

    func testGenerateAbbrev_nasalSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ-á"), "sa")
    }

    func testGenerateAbbrev_spaceSeparated() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("lí hó"), "lh")
    }

    func testGenerateAbbrev_singleSyllable_empty() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ"), "")
    }

    func testGenerateAbbrev_singleWord_empty() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("hó"), "")
    }

    // MARK: - generateRomanNum (delegates to InputNormalizer)

    func testGenerateRomanNum_delegatesToInputNormalizer() {
        // Parity check — generateRomanNum is documented as RustEngineBridge.normalizeInput(_).
        XCTAssertEqual(CustomDictionaryDerivation.generateRomanNum("gâu-tsá"), "gau5tsa2")
    }

    // MARK: - queryKey / searchKeys (v3.6.1 R3 cross-mode)

    // trace (engine/phonetics/src/custom_search.rs): tone-aware = ASCII digit
    // present → form "num"; mode .tl → family "tl"; key fused = "ho2".
    func testQueryKey_toneAware_tlFamilyNumForm() {
        let q = CustomDictionaryDerivation.queryKey(for: "ho2", mode: .tl)
        XCTAssertEqual(q?.family, "tl")
        XCTAssertEqual(q?.form, "num")
        XCTAssertEqual(q?.key, "ho2")
    }

    // trace: digit present + hyphen → fuse_latin_numeric drops hyphen → "gau5tsa2".
    func testQueryKey_toneAware_stripsHyphens() {
        let q = CustomDictionaryDerivation.queryKey(for: "gau5-tsa2", mode: .tl)
        XCTAssertEqual(q?.form, "num")
        XCTAssertEqual(q?.key, "gau5tsa2")
    }

    // trace: no digit → form "notone"; derive_notone("hó") strips diacritic → "ho".
    func testQueryKey_toneless_tlFamilyNotoneForm() {
        let q = CustomDictionaryDerivation.queryKey(for: "hó", mode: .tl)
        XCTAssertEqual(q?.family, "tl")
        XCTAssertEqual(q?.form, "notone")
        XCTAssertEqual(q?.key, "ho")
    }

    // trace: parse_input_mode(.poj) → family "poj".
    func testQueryKey_pojMode_pojFamily() {
        let q = CustomDictionaryDerivation.queryKey(for: "chiah", mode: .poj)
        XCTAssertEqual(q?.family, "poj")
    }

    // trace: input.trim().is_empty() → None.
    func testQueryKey_emptyInput_nil() {
        XCTAssertNil(CustomDictionaryDerivation.queryKey(for: "", mode: .tl))
        XCTAssertNil(CustomDictionaryDerivation.queryKey(for: "   ", mode: .tl))
    }

    // trace: derive_custom_search_keys("chiah") materializes the {tl,poj,tps}
    // bundle; canonical TL = "tsiah", POJ = "chiah".
    func testSearchKeys_pojStored_carriesBothLatinFamilies() {
        let keys = CustomDictionaryDerivation.searchKeys(for: "chiah")
        XCTAssertTrue(keys.contains(CustomSearchKey(family: "tl", form: "notone", key: "tsiah")))
        XCTAssertTrue(keys.contains(CustomSearchKey(family: "poj", form: "notone", key: "chiah")))
    }

    func testSearchKeys_empty_yieldsNothing() {
        XCTAssertTrue(CustomDictionaryDerivation.searchKeys(for: "").isEmpty)
        XCTAssertTrue(CustomDictionaryDerivation.searchKeys(for: "   ").isEmpty)
    }

    // MARK: - INVARIANT wrappers — Phase 0 §10

    func test_INVARIANT_custom_derivation_matches_input_normalizer() {
        // The `roman_num` key must be exactly what RustEngineBridge.normalizeInput(_)
        // produces, or custom-dictionary entries become invisible through the main search.
        let fixtures = ["gâu-tsá", "tāi-tsì", "hó", "tsiah8-pá"]
        for input in fixtures {
            XCTAssertEqual(
                CustomDictionaryDerivation.generateRomanNum(input),
                RustEngineBridge.normalizeInput(input),
                "Parity failure on \(input)",
            )
        }
    }

    func test_INVARIANT_abbrev_key_is_one_char_per_syllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("gâu-tsá"), "gt")
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("lí-hó-bô"), "lhb")
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ-á-kûn"), "sak")
    }
}
