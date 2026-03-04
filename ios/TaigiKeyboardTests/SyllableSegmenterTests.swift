import XCTest
@testable import TaigiKeyboard

/// SyllableSegmenter unit tests
final class SyllableSegmenterTests: XCTestCase {

    // MARK: - Basic Segmentation

    func testSegment_multiSyllableContinuous() {
        let result = SyllableSegmenter.segment("gua2si7soo")
        XCTAssertEqual(result, ["gua2", "si7", "soo"])
    }

    func testSegment_singleSyllable() {
        XCTAssertEqual(SyllableSegmenter.segment("ka2"), ["ka2"])
    }

    func testSegment_singleSyllableNoTone() {
        XCTAssertEqual(SyllableSegmenter.segment("ka"), ["ka"])
    }

    func testSegment_empty() {
        XCTAssertEqual(SyllableSegmenter.segment(""), [])
    }

    // MARK: - Hyphen Handling

    func testSegment_hyphenSeparated() {
        let result = SyllableSegmenter.segment("gua2-si7")
        XCTAssertEqual(result, ["gua2-", "si7"])
    }

    func testSegment_mixedContinuousAndHyphen() {
        let result = SyllableSegmenter.segment("gua2si7-soo")
        XCTAssertEqual(result, ["gua2", "si7-", "soo"])
    }

    func testSegment_multipleHyphens() {
        let result = SyllableSegmenter.segment("ka2-lang5-e5")
        XCTAssertEqual(result, ["ka2-", "lang5-", "e5"])
    }

    func testSegment_leadingHyphen_preserved() {
        let cases: [(input: String, expected: [String])] = [
            ("-gua2", ["-gua2"]),
            ("-gua2-si7", ["-gua2-", "si7"]),
            ("--a", ["--a"]),
            ("-", ["-"]),
            ("--", ["--"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "segment(\"\(input)\"): leading hyphen must be preserved"
            )
        }
    }

    // MARK: - POJ Input Forms

    func testSegment_pojChInitial() {
        // POJ "chhi" is recognized via trie
        let result = SyllableSegmenter.segment("chhi2ka1")
        XCTAssertEqual(result, ["chhi2", "ka1"])
    }

    func testSegment_pojOaFinal() {
        let result = SyllableSegmenter.segment("koa1sue3")
        XCTAssertEqual(result, ["koa1", "sue3"])
    }

    func testSegment_pojEngFinal() {
        let result = SyllableSegmenter.segment("peng5")
        XCTAssertEqual(result, ["peng5"])
    }

    // MARK: - Edge Cases

    func testSegment_unrecognizedFallsBackToSingleChars() {
        let result = SyllableSegmenter.segment("xyz")
        // Unrecognized characters become single-char segments
        XCTAssertEqual(result.count, 3)
        XCTAssertEqual(result, ["x", "y", "z"])
    }

    func testSegment_longInput() {
        let result = SyllableSegmenter.segment("bin5hian5")
        XCTAssertEqual(result, ["bin5", "hian5"])
    }

    // MARK: - Case Preservation

    func testSegment_uppercasePreserved() {
        let result = SyllableSegmenter.segment("Gua2si7")
        // Segmentation works on lowercased internally but preserves original case
        XCTAssertEqual(result.count, 2)
        XCTAssertTrue(result[0].hasPrefix("G"), "Uppercase should be preserved in output")
    }

    // MARK: - Tone Digits

    func testSegment_allToneDigits() {
        // Tone digits 1-9 should be consumed with syllables
        let cases: [(input: String, expected: [String])] = [
            ("ka1", ["ka1"]),
            ("ka2", ["ka2"]),
            ("ka3", ["ka3"]),
            ("kah4", ["kah4"]),
            ("ka5", ["ka5"]),
            ("ka7", ["ka7"]),
            ("kah8", ["kah8"]),
            ("ka9", ["ka9"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "segment(\(input))"
            )
        }
    }

    // MARK: - Complex Multi-Syllable

    func testSegment_fiveSyllables() {
        let result = SyllableSegmenter.segment("gua2si7hak8sing1e5")
        XCTAssertEqual(result, ["gua2", "si7", "hak8", "sing1", "e5"])
    }

    // MARK: - Onset Atomicity (MOE2 layout)

    func testSegment_standaloneOnset_notSplit() {
        // Multi-character onsets must remain atomic, not split into single chars
        let cases: [(input: String, expected: [String])] = [
            ("tsh", ["tsh"]),
            ("ts", ["ts"]),
            ("ph", ["ph"]),
            ("th", ["th"]),
            ("kh", ["kh"]),
            ("ng", ["ng"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "Onset '\(input)' must stay atomic"
            )
        }
    }

    func testSegment_standaloneOnset_poj() {
        let cases: [(input: String, expected: [String])] = [
            ("ch", ["ch"]),
            ("chh", ["chh"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "POJ onset '\(input)' must stay atomic"
            )
        }
    }

    func testSegment_trailingOnset_notSplit() {
        // When a complete syllable is followed by an incomplete onset,
        // the onset must remain atomic
        let cases: [(input: String, expected: [String])] = [
            ("ka2tsh", ["ka2", "tsh"]),
            ("ka2ph", ["ka2", "ph"]),
            ("gua2si7th", ["gua2", "si7", "th"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "segment(\(input)): trailing onset must stay atomic"
            )
        }
    }

    func testSegment_onsetWithFinal_stillPrefersSyllable() {
        // Adding onsets to trie must not break complete syllable segmentation.
        // Complete syllables always outscore onset + remainder (squared scoring).
        let cases: [(input: String, expected: [String])] = [
            ("tsha2", ["tsha2"]),
            ("pha3", ["pha3"]),
            ("thau5", ["thau5"]),
            ("khi2", ["khi2"]),
            ("tsai5", ["tsai5"]),
            ("chhi2ka1", ["chhi2", "ka1"]),
            ("phong", ["phong"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "segment(\(input)): complete syllable must not be split at onset boundary"
            )
        }
    }

    // MARK: - Nasalization Suffix Atomicity (MOE2 layout)

    func testSegment_standaloneNn_notSplit() {
        // "nn" is a single key on MOE2; must not split into "n"+"n"
        XCTAssertEqual(
            SyllableSegmenter.segment("nn"), ["nn"],
            "Standalone 'nn' must stay atomic"
        )
    }

    func testSegment_trailingNn_notSplit() {
        // After a completed syllable, trailing "nn" stays atomic
        XCTAssertEqual(
            SyllableSegmenter.segment("ka2nn"), ["ka2", "nn"],
            "Trailing 'nn' after toned syllable must stay atomic"
        )
    }

    func testSegment_nnInSyllable_stillPrefersSyllable() {
        // Complete nasalized syllables must not split at "nn" boundary
        let cases: [(input: String, expected: [String])] = [
            ("kann2", ["kann2"]),
            ("ann", ["ann"]),
            ("phiann3", ["phiann3"]),
            ("iunn5", ["iunn5"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input), expected,
                "segment(\(input)): 'nn' must remain part of complete syllable"
            )
        }
    }

    // MARK: - CVC+V Tie-Breaking (without checker)

    func testSegment_cvcvTie_withoutChecker_documentsCurrentBehavior() {
        // Without a wordPrefixChecker, ties resolve by first-arrival.
        // kina2jit8: ki(4)+na2(9)=13 wins over kin(9)+a2(4)=13
        // This documents the known limitation.
        XCTAssertEqual(
            SyllableSegmenter.segment("kina2jit8"),
            ["ki", "na2", "jit8"],
            "Without checker, first-arrival wins the tie (known limitation)"
        )
    }

    func testSegment_ama_withoutChecker_correctByDefault() {
        // ama: a(1)+ma(4)=5 wins over am(4)+a(1)=5 by first-arrival.
        // This is already correct without a checker.
        XCTAssertEqual(
            SyllableSegmenter.segment("ama"),
            ["a", "ma"],
            "Without checker, ama already segments correctly by first-arrival"
        )
    }

    // MARK: - CVC+V Tie-Breaking (with mock checker)

    func testSegment_kina2jit8_withChecker_prefersKinA2() {
        // Mock checker: "kin1a2" prefix has dictionary matches, "ki1na2" does not
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("kin1a2")
        }
        XCTAssertEqual(
            SyllableSegmenter.segment("kina2jit8", wordPrefixChecker: checker),
            ["kin", "a2", "jit8"],
            "With checker, kin+a2+jit8 should win tie (今仔日)"
        )
    }

    func testSegment_ama_withChecker_preservesCorrectPath() {
        // Mock checker: "ama" prefix has dictionary matches, "am1a" does not
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("ama")
        }
        XCTAssertEqual(
            SyllableSegmenter.segment("ama", wordPrefixChecker: checker),
            ["a", "ma"],
            "With checker, a+ma should be preserved (阿媽)"
        )
    }

    func testSegment_hita2e5_withChecker_prefersHitA2() {
        // hit+a2 should win over hi+ta2 when dictionary confirms
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("hit4a2")
        }
        XCTAssertEqual(
            SyllableSegmenter.segment("hita2e5", wordPrefixChecker: checker),
            ["hit", "a2", "e5"],
            "With checker, hit+a2+e5 should win tie (彼个)"
        )
    }

    func testSegment_noChecker_backwardCompatible() {
        // All existing behavior must be preserved when checker is nil
        let cases: [(input: String, expected: [String])] = [
            ("gua2si7soo", ["gua2", "si7", "soo"]),
            ("ka2", ["ka2"]),
            ("bin5hian5", ["bin5", "hian5"]),
            ("gua2si7-soo", ["gua2", "si7-", "soo"]),
        ]
        for (input, expected) in cases {
            XCTAssertEqual(
                SyllableSegmenter.segment(input, wordPrefixChecker: nil), expected,
                "segment(\(input), checker: nil) must match existing behavior"
            )
        }
    }

    // MARK: - isValidPrefix (Mode-Aware Trie Validation)

    func testIsValidPrefix_pojMode_rejectsTLOnlyInput() {
        let cases: [(input: String, expected: Bool, reason: String)] = [
            ("ts", false, "TL-only initial (POJ uses ch)"),
            ("tsh", false, "TL-only initial (POJ uses chh)"),
            ("gua", false, "TL-only final ua (POJ uses oa)"),
            ("ing", false, "TL-only final (POJ uses eng)"),
        ]
        for (input, expected, reason) in cases {
            XCTAssertEqual(
                SyllableSegmenter.isValidPrefix(input, mode: .poj), expected,
                "isValidPrefix(\"\(input)\", .poj): \(reason)"
            )
        }
    }

    func testIsValidPrefix_pojMode_acceptsPOJInput() {
        let cases: [(input: String, expected: Bool, reason: String)] = [
            ("ch", true, "POJ initial prefix"),
            ("chh", true, "POJ initial prefix"),
            ("goa", true, "POJ final oa"),
            ("eng", true, "POJ final"),
            ("hoo", true, "oo is shared (keyboard shorthand)"),
            ("ka", true, "shared syllable"),
            ("phang", true, "shared syllable"),
        ]
        for (input, expected, reason) in cases {
            XCTAssertEqual(
                SyllableSegmenter.isValidPrefix(input, mode: .poj), expected,
                "isValidPrefix(\"\(input)\", .poj): \(reason)"
            )
        }
    }

    func testIsValidPrefix_tlMode_rejectsPOJOnlyInput() {
        let cases: [(input: String, expected: Bool, reason: String)] = [
            ("ch", false, "POJ-only initial (TL uses ts)"),
            ("chh", false, "POJ-only initial (TL uses tsh)"),
            ("goa", false, "POJ-only final oa (TL uses ua)"),
        ]
        for (input, expected, reason) in cases {
            XCTAssertEqual(
                SyllableSegmenter.isValidPrefix(input, mode: .tl), expected,
                "isValidPrefix(\"\(input)\", .tl): \(reason)"
            )
        }
    }

    func testIsValidPrefix_tlMode_acceptsTLInput() {
        let cases: [(input: String, expected: Bool, reason: String)] = [
            ("ts", true, "TL initial prefix"),
            ("tsh", true, "TL initial prefix"),
            ("gua", true, "TL final ua"),
            ("ua", true, "TL final"),
            ("ing", true, "TL final"),
            ("eng", true, "TL final (嬰)"),
            ("ka", true, "shared syllable"),
        ]
        for (input, expected, reason) in cases {
            XCTAssertEqual(
                SyllableSegmenter.isValidPrefix(input, mode: .tl), expected,
                "isValidPrefix(\"\(input)\", .tl): \(reason)"
            )
        }
    }

    func testIsValidPrefix_emptyInput() {
        XCTAssertTrue(SyllableSegmenter.isValidPrefix("", mode: .poj))
        XCTAssertTrue(SyllableSegmenter.isValidPrefix("", mode: .tl))
    }

    // MARK: - Word Grouping

    func testGroupIntoWords_withChecker_groupsKnownWords() {
        // "kin1a2jit8" is a known word prefix → group together
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("kin1a2")
        }
        let syllables = ["gua2", "kin", "a2", "jit8"]
        let groups = SyllableSegmenter.groupIntoWords(syllables, wordPrefixChecker: checker)
        XCTAssertEqual(
            groups, [["gua2"], ["kin", "a2", "jit8"]],
            "kin+a2+jit8 should be grouped as one word (今仔日)"
        )
    }

    func testGroupIntoWords_withChecker_groupsAma() {
        // "ama" is a known word prefix → group together
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("ama")
        }
        let syllables = ["a", "ma"]
        let groups = SyllableSegmenter.groupIntoWords(syllables, wordPrefixChecker: checker)
        XCTAssertEqual(
            groups, [["a", "ma"]],
            "a+ma should be grouped as one word (阿媽)"
        )
    }

    func testGroupIntoWords_withoutChecker_eachSyllableSeparate() {
        let syllables = ["gua2", "kin", "a2", "jit8"]
        let groups = SyllableSegmenter.groupIntoWords(syllables, wordPrefixChecker: nil)
        XCTAssertEqual(
            groups, [["gua2"], ["kin"], ["a2"], ["jit8"]],
            "Without checker, each syllable should be its own group"
        )
    }

    func testGroupIntoWords_greediestMatch() {
        // Checker matches both 2-syllable and 3-syllable prefixes;
        // should pick the longest (greedy)
        let checker: SyllableSegmenter.WordPrefixChecker = { key in
            key.hasPrefix("kin1a2") // matches both kin1a2 (2) and kin1a2jit8 (3)
        }
        let syllables = ["kin", "a2", "jit8"]
        let groups = SyllableSegmenter.groupIntoWords(syllables, wordPrefixChecker: checker)
        XCTAssertEqual(
            groups, [["kin", "a2", "jit8"]],
            "Greedy matching should pick the longest word group"
        )
    }
}
