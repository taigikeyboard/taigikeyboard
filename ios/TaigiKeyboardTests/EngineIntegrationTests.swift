import XCTest
@testable import TaigiKeyboard

/// Cross-component engine integration tests
final class EngineIntegrationTests: XCTestCase {

    // MARK: - A. Normalization Pipeline

    func testNormalizationPipeline_tlDiacriticInput() {
        // TL diacritic input -> TL numeric key
        let result = InputNormalizer.normalize("h\u{00F3}-b\u{00F4}", mode: .tl)
        XCTAssertEqual(result, "ho2bo5")
    }

    func testNormalizationPipeline_pojDiacriticInput() {
        // POJ diacritic input -> stays in POJ numeric form
        // (trie prefix poj: is added at query boundary, not here)
        let result = InputNormalizer.normalize("ch\u{00FA}", mode: .poj)
        XCTAssertEqual(result, "chu2")
    }

    func testNormalizationPipeline_tlNumericInput() {
        // Already numeric TL -> passes through
        let result = InputNormalizer.normalize("ka2", mode: .tl)
        XCTAssertEqual(result, "ka2")
    }

    func testNormalizationPipeline_pojNumericInput() {
        // POJ numeric stays in POJ form (no oa->ua conversion)
        let result = InputNormalizer.normalize("koa1", mode: .poj)
        XCTAssertEqual(result, "koa1")
    }

    func testNormalizationPipeline_multiSyllable() {
        let result = InputNormalizer.normalize("t\u{00E2}i-g\u{00ED}", mode: .tl)
        XCTAssertEqual(result, "tai5gi2")
    }

    // MARK: - B. Tone Mark Round-Trip (POJ mode cross-component)

    func testToneMarkRoundTrip_pojMode() {
        let marked = TaigiPhonetics.convertSyllable("ka2", mode: .poj)
        XCTAssertEqual(marked, "k\u{00E1}")  // ká (same for simple vowel)

        let restored = ToneRestoration.restore(marked, mode: .poj)
        XCTAssertEqual(restored, "ka")
    }

    // MARK: - C. Segmenter + Normalizer Pipeline

    func testSegmenterThenNormalizer_continuousInput() {
        // Segment continuous input, then normalize each syllable
        let segments = SyllableSegmenter.segment("gua2si7")
        XCTAssertEqual(segments, ["gua2", "si7"])

        let normalized = InputNormalizer.normalize(
            segments.joined(separator: "-"), mode: .tl
        )
        XCTAssertEqual(normalized, "gua2si7")
    }

    func testSegmenterThenNormalizer_pojInput() {
        let segments = SyllableSegmenter.segment("chhi2ka1")
        XCTAssertEqual(segments, ["chhi2", "ka1"])

        let normalized = InputNormalizer.normalize(
            segments.joined(separator: "-"), mode: .poj
        )
        // Stays in POJ form (no ch->ts conversion)
        XCTAssertEqual(normalized, "chhi2ka1")
    }

    func testSegmenterThenNormalizer_singleSyllable() {
        let segments = SyllableSegmenter.segment("lang5")
        XCTAssertEqual(segments, ["lang5"])

        let normalized = InputNormalizer.normalize("lang5", mode: .tl)
        XCTAssertEqual(normalized, "lang5")
    }

}
