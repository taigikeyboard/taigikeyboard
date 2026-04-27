@testable import TaigiKeyboard
import XCTest

/// CharacterInputPipeline tests — locks the contract of the collapsed
/// single-entry-point that wraps the four `TPSInputAdjuster` calls.
///
/// Pre-D9.4 commit-1 baseline: every fixture here is the agreed-canonical
/// behavior. Post-D9.4, the same fixtures must hold against the Rust
/// implementation called via `OP_TPS_INPUT_ADJUST`.
final class CharacterInputPipelineTests: XCTestCase {
    // MARK: - Non-TPS gate

    func testAdjust_nonTpsMode_returnsCharUnchanged_poj() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .poj, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_nonTpsMode_returnsCharUnchanged_tl() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tl, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_nonTpsMode_returnsCharUnchanged_english() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .english, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Dual-form initials at syllable start

    func testAdjust_dualForm_emptyBuffer_keepsInitial() {
        for ch in ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"] {
            let result = CharacterInputPipeline.adjust(ch, inputMode: .tps, rawInput: "")
            XCTAssertEqual(result.char, ch, "\(ch) on empty buffer should stay initial")
            XCTAssertNil(result.replaceLast)
        }
    }

    func testAdjust_dualForm_afterSpace_keepsInitial() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚ ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_dualForm_afterToneMark_keepsInitial() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚˊ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_dualForm_afterCheckedFinal_keepsInitial() {
        // After ㆴ (entering tone -p) → new syllable
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚㆴ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_dualForm_afterNasalFinal_keepsInitial() {
        // After ㆬ (syllabic m / -m final) → new syllable
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚㆬ")
        XCTAssertEqual(result.char, "ㄇ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Dual-form initials NOT at syllable start (final form)

    func testAdjust_M_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆬ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_N_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄋ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㄣ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_B_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄅ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆴ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_D_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄉ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆵ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_G_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄍ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆻ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_H_afterVowel_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄏ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆷ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - ㄫ context-dependent

    func testAdjust_NG_afterI_returnsIng() {
        let result = CharacterInputPipeline.adjust("ㄫ", inputMode: .tps, rawInput: "ㄧ")
        XCTAssertEqual(result.char, "ㄥ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_NG_afterOtherVowel_returnsSyllabicNg() {
        let result = CharacterInputPipeline.adjust("ㄫ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆭ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Hyphen quirk (locks current behavior — hyphen is NOT syllable boundary)

    /// Locks the current iOS contract: `"-"` is NOT in `syllableBoundaryChars`,
    /// so a dual-form key after a hyphen still gets the FINAL form. If parity
    /// audit later decides this is a bug, fix as a separate `parity:` PR.
    func testAdjust_dualForm_afterHyphen_returnsFinal() {
        let result = CharacterInputPipeline.adjust("ㄇ", inputMode: .tps, rawInput: "ㄚ-")
        XCTAssertEqual(result.char, "ㆬ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Non-dual-form pass-through

    func testAdjust_nonDualForm_passesThrough() {
        for ch in ["ㄆ", "ㄊ", "ㄎ", "ㄈ", "ㄌ"] {
            let result = CharacterInputPipeline.adjust(ch, inputMode: .tps, rawInput: "ㄚ")
            XCTAssertEqual(result.char, ch, "\(ch) is not dual-form, should pass through")
            XCTAssertNil(result.replaceLast)
        }
    }

    // MARK: - ㆮ → ㆯ disambiguation

    func testAdjust_AINN_afterI_becomesAUNN() {
        let result = CharacterInputPipeline.adjust("ㆮ", inputMode: .tps, rawInput: "ㄧ")
        XCTAssertEqual(result.char, "ㆯ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_AINN_afterOtherVowel_unchanged() {
        let result = CharacterInputPipeline.adjust("ㆮ", inputMode: .tps, rawInput: "ㄚ")
        XCTAssertEqual(result.char, "ㆮ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_otherChar_afterI_noDisambiguation() {
        let result = CharacterInputPipeline.adjust("ㄚ", inputMode: .tps, rawInput: "ㄧ")
        XCTAssertEqual(result.char, "ㄚ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Syllabic nasal replacement (ㄇ/ㄫ + tone)

    func testAdjust_toneAfterM_replacesWithSyllabicM() {
        let result = CharacterInputPipeline.adjust("ˊ", inputMode: .tps, rawInput: "ㄇ")
        XCTAssertEqual(result.char, "ˊ")
        XCTAssertEqual(result.replaceLast, "ㆬ")
    }

    func testAdjust_toneAfterNG_replacesWithSyllabicNg() {
        let result = CharacterInputPipeline.adjust("ˊ", inputMode: .tps, rawInput: "ㄫ")
        XCTAssertEqual(result.char, "ˊ")
        XCTAssertEqual(result.replaceLast, "ㆭ")
    }

    func testAdjust_toneAfterOtherConsonant_noReplace() {
        // Non-ㄇ/ㄫ consonant + tone → no syllabic nasal replacement.
        let result = CharacterInputPipeline.adjust("ˊ", inputMode: .tps, rawInput: "ㄉ")
        XCTAssertEqual(result.char, "ˊ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_nonToneAfterM_noReplace() {
        // ㄇ + non-tone (vowel) → no syllabic nasal replacement; ㄚ pass-through.
        let result = CharacterInputPipeline.adjust("ㄚ", inputMode: .tps, rawInput: "ㄇ")
        XCTAssertEqual(result.char, "ㄚ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_toneOnEmptyBuffer_noReplace() {
        let result = CharacterInputPipeline.adjust("ˊ", inputMode: .tps, rawInput: "")
        XCTAssertEqual(result.char, "ˊ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Palatalization (ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ)

    func testAdjust_iAfterTs_palatalizes() {
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "ㄗ")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertEqual(result.replaceLast, "ㄐ")
    }

    func testAdjust_iAfterTsh_palatalizes() {
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "ㄘ")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertEqual(result.replaceLast, "ㄑ")
    }

    func testAdjust_iAfterS_palatalizes() {
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "ㄙ")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertEqual(result.replaceLast, "ㄒ")
    }

    func testAdjust_iAfterJ_palatalizes() {
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "ㆡ")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertEqual(result.replaceLast, "ㆢ")
    }

    func testAdjust_inn_AfterTs_palatalizes() {
        // ㆪ (nasalized i) also triggers palatalization.
        let result = CharacterInputPipeline.adjust("ㆪ", inputMode: .tps, rawInput: "ㄗ")
        XCTAssertEqual(result.char, "ㆪ")
        XCTAssertEqual(result.replaceLast, "ㄐ")
    }

    func testAdjust_nonTriggerAfterAffricate_noPalatalization() {
        // ㄗ + ㄚ (non-trigger vowel) → no replace.
        let result = CharacterInputPipeline.adjust("ㄚ", inputMode: .tps, rawInput: "ㄗ")
        XCTAssertEqual(result.char, "ㄚ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_iAfterNonAffricate_noPalatalization() {
        // ㄆ + ㄧ (non-affricate previous) → no replace.
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "ㄆ")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertNil(result.replaceLast)
    }

    func testAdjust_iOnEmptyBuffer_noPalatalization() {
        let result = CharacterInputPipeline.adjust("ㄧ", inputMode: .tps, rawInput: "")
        XCTAssertEqual(result.char, "ㄧ")
        XCTAssertNil(result.replaceLast)
    }

    // MARK: - Combined / disjoint trigger guarantee

    /// Pins the contract that syllabic-nasal and palatalization triggers are
    /// disjoint by `lastChar`: `{ㄇ, ㄫ}` vs `{ㄗ, ㄘ, ㄙ, ㆡ}`. Both effects
    /// can never fire on the same call, so the `??`-style short-circuit is
    /// observationally identical to running both checks unconditionally.
    func testAdjust_syllabicNasalAndPalatalization_areDisjoint() {
        // Last char in syllabic-nasal set ({ㄇ, ㄫ}) is never in palatalization set.
        for last in ["ㄇ", "ㄫ"] {
            for incoming in ["ㄧ", "ㆪ"] {
                let result = CharacterInputPipeline.adjust(incoming, inputMode: .tps, rawInput: last)
                // No palatalization (last is not ㄗ/ㄘ/ㄙ/ㆡ).
                // No syllabic nasal (incoming is not a tone mark).
                XCTAssertEqual(result.char, incoming)
                XCTAssertNil(
                    result.replaceLast,
                    "incoming \(incoming) after \(last) must not trigger either replacement",
                )
            }
        }
    }
}
