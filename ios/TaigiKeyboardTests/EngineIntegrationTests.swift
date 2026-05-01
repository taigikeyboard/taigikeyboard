import XCTest
@testable import TaigiKeyboard

/// Cross-component engine integration tests
final class EngineIntegrationTests: XCTestCase {

    // MARK: - A. Normalization Pipeline

    func testNormalizationPipeline_tlDiacriticInput() {
        // TL diacritic input -> TL numeric key
        let result = RustEngineBridge.normalizeInput("h\u{00F3}-b\u{00F4}")
        XCTAssertEqual(result, "ho2bo5")
    }

    func testNormalizationPipeline_pojDiacriticInput() {
        // POJ diacritic input -> stays in POJ numeric form
        // (trie prefix poj: is added at query boundary, not here)
        let result = RustEngineBridge.normalizeInput("ch\u{00FA}")
        XCTAssertEqual(result, "chu2")
    }

    func testNormalizationPipeline_tlNumericInput() {
        // Already numeric TL -> passes through
        let result = RustEngineBridge.normalizeInput("ka2")
        XCTAssertEqual(result, "ka2")
    }

    func testNormalizationPipeline_pojNumericInput() {
        // POJ numeric stays in POJ form (no oa->ua conversion)
        let result = RustEngineBridge.normalizeInput("koa1")
        XCTAssertEqual(result, "koa1")
    }

    func testNormalizationPipeline_multiSyllable() {
        let result = RustEngineBridge.normalizeInput("t\u{00E2}i-g\u{00ED}")
        XCTAssertEqual(result, "tai5gi2")
    }

    // MARK: - B. Tone Mark Round-Trip (POJ mode cross-component)

    func testToneMarkRoundTrip_pojMode() {
        let marked = TaigiPhonetics.convertSyllable("ka2", mode: .poj)
        XCTAssertEqual(marked, "k\u{00E1}")  // ká (same for simple vowel)

        let restored = ToneRestoration.restore(marked, mode: .poj)
        XCTAssertEqual(restored, "ka")
    }

}
