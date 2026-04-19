@testable import TaigiKeyboard
import XCTest

/// CaseTransformer 測試
final class CaseTransformerTests: XCTestCase {
    // MARK: - transformForInput Tests

    /// 測試大寫狀態下的一般字母轉換
    func testTransformForInput_uppercased_normalLetter() {
        let result = CaseTransformer.transformForInput(
            "a",
            letterCase: .uppercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "A")
    }

    /// 測試小寫狀態下的一般字母轉換
    func testTransformForInput_lowercased_normalLetter() {
        let result = CaseTransformer.transformForInput(
            "A",
            letterCase: .lowercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "a")
    }

    /// 測試 Caps Lock 狀態下的轉換
    func testTransformForInput_capsLocked_normalLetter() {
        let result = CaseTransformer.transformForInput(
            "a",
            letterCase: .capsLocked,
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "A")
    }

    /// 測試 POJ 模式下聲調字母的大寫轉換
    func testTransformForInput_uppercased_toneLetter_POJ() {
        let result = CaseTransformer.transformForInput(
            "á",
            letterCase: .uppercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "Á")
    }

    /// 測試 POJ 模式下聲調字母的小寫轉換
    func testTransformForInput_lowercased_toneLetter_POJ() {
        let result = CaseTransformer.transformForInput(
            "Á",
            letterCase: .lowercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "á")
    }

    /// 測試 TL 模式下聲調字母的大寫轉換
    func testTransformForInput_uppercased_toneLetter_TL() {
        let result = CaseTransformer.transformForInput(
            "ê",
            letterCase: .uppercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .tl,
        )
        XCTAssertEqual(result, "Ê")
    }

    /// 測試 TL 模式下聲調字母的小寫轉換
    func testTransformForInput_lowercased_toneLetter_TL() {
        let result = CaseTransformer.transformForInput(
            "Ê",
            letterCase: .lowercased,
            isAutoCapitalizationEnabled: true,
            inputMode: .tl,
        )
        XCTAssertEqual(result, "ê")
    }

    // MARK: - Auto Capitalization Disabled Tests

    /// 測試自動大寫關閉時，小寫狀態保持小寫
    func testTransformForInput_autoCapOff_lowercased() {
        let result = CaseTransformer.transformForInput(
            "a",
            letterCase: .lowercased,
            isAutoCapitalizationEnabled: false,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "a")
    }

    /// 測試自動大寫關閉時，手動 Shift 仍可大寫
    func testTransformForInput_autoCapOff_manualShift() {
        let result = CaseTransformer.transformForInput(
            "a",
            letterCase: .uppercased,
            isAutoCapitalizationEnabled: false,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "A")
    }

    /// 測試自動大寫關閉時，Caps Lock 仍可大寫
    func testTransformForInput_autoCapOff_capsLock() {
        let result = CaseTransformer.transformForInput(
            "a",
            letterCase: .capsLocked,
            isAutoCapitalizationEnabled: false,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "A")
    }

    /// 測試自動大寫關閉時，手動 Shift 可轉換聲調字母
    func testTransformForInput_autoCapOff_manualShift_toneLetter() {
        let result = CaseTransformer.transformForInput(
            "ô",
            letterCase: .uppercased,
            isAutoCapitalizationEnabled: false,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "Ô")
    }

    // MARK: - capitalizeCandidate Tests

    /// 測試候選詞首字母大寫
    func testCapitalizeCandidate_inputUppercase() {
        let result = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "Tai",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "Tâi-gí")
    }

    /// 測試候選詞保持小寫（輸入為小寫）
    func testCapitalizeCandidate_inputLowercase() {
        let result = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "tai",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "tâi-gí")
    }

    /// 測試自動大寫關閉時候選詞不變
    func testCapitalizeCandidate_autoCapOff() {
        let result = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "Tai",
            isAutoCapitalizationEnabled: false,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "tâi-gí")
    }

    /// 測試空輸入時候選詞不變
    func testCapitalizeCandidate_emptyInput() {
        let result = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "tâi-gí")
    }

    /// 測試空候選詞返回空字串
    func testCapitalizeCandidate_emptyText() {
        let result = CaseTransformer.capitalizeCandidate(
            "",
            basedOn: "Tai",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "")
    }

    /// 測試候選詞首字為聲調字母時的大寫轉換
    func testCapitalizeCandidate_toneLetterFirst() {
        let result = CaseTransformer.capitalizeCandidate(
            "ô-pêh-sai",
            basedOn: "O",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "Ô-pêh-sai")
    }

    /// 測試候選詞首字為非字母時不變
    func testCapitalizeCandidate_nonLetterFirst() {
        let result = CaseTransformer.capitalizeCandidate(
            "123abc",
            basedOn: "A",
            isAutoCapitalizationEnabled: true,
            inputMode: .poj,
        )
        XCTAssertEqual(result, "123abc")
    }

    // MARK: - POJ Tone Letter Mapping Tests

    /// 測試 POJ 所有聲調字母的大寫轉換
    func testPOJ_uppercaseToneLetters() {
        let testCases: [(input: String, expected: String)] = [
            // a
            ("á", "Á"), ("à", "À"), ("â", "Â"), ("ǎ", "Ǎ"), ("ā", "Ā"), ("a̍", "A̍"), ("ă", "Ă"),
            // e
            ("é", "É"), ("è", "È"), ("ê", "Ê"), ("ě", "Ě"), ("ē", "Ē"), ("e̍", "E̍"), ("ĕ", "Ĕ"),
            // i
            ("í", "Í"), ("ì", "Ì"), ("î", "Î"), ("ǐ", "Ǐ"), ("ī", "Ī"), ("i̍", "I̍"), ("ĭ", "Ĭ"),
            // o
            ("ó", "Ó"), ("ò", "Ò"), ("ô", "Ô"), ("ǒ", "Ǒ"), ("ō", "Ō"), ("o̍", "O̍"), ("ŏ", "Ŏ"),
            // u
            ("ú", "Ú"), ("ù", "Ù"), ("û", "Û"), ("ǔ", "Ǔ"), ("ū", "Ū"), ("u̍", "U̍"), ("ŭ", "Ŭ"),
            // n
            ("ń", "Ń"), ("ǹ", "Ǹ"), ("n̂", "N̂"), ("ň", "Ň"), ("n̄", "N̄"), ("n̍", "N̍"), ("n̋", "N̋"),
            // m
            ("ḿ", "Ḿ"), ("m̀", "M̀"), ("m̂", "M̂"), ("m̌", "M̌"), ("m̄", "M̄"), ("m̍", "M̍"), ("m̋", "M̋"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .uppercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .poj,
            )
            XCTAssertEqual(result, expected, "POJ uppercase: \(input) should be \(expected), got \(result)")
        }
    }

    /// 測試 POJ 所有聲調字母的小寫轉換
    func testPOJ_lowercaseToneLetters() {
        let testCases: [(input: String, expected: String)] = [
            // A
            ("Á", "á"), ("À", "à"), ("Â", "â"), ("Ǎ", "ǎ"), ("Ā", "ā"), ("A̍", "a̍"), ("Ă", "ă"),
            // E
            ("É", "é"), ("È", "è"), ("Ê", "ê"), ("Ě", "ě"), ("Ē", "ē"), ("E̍", "e̍"), ("Ĕ", "ĕ"),
            // I
            ("Í", "í"), ("Ì", "ì"), ("Î", "î"), ("Ǐ", "ǐ"), ("Ī", "ī"), ("I̍", "i̍"), ("Ĭ", "ĭ"),
            // O
            ("Ó", "ó"), ("Ò", "ò"), ("Ô", "ô"), ("Ǒ", "ǒ"), ("Ō", "ō"), ("O̍", "o̍"), ("Ŏ", "ŏ"),
            // U
            ("Ú", "ú"), ("Ù", "ù"), ("Û", "û"), ("Ǔ", "ǔ"), ("Ū", "ū"), ("U̍", "u̍"), ("Ŭ", "ŭ"),
            // N
            ("Ń", "ń"), ("Ǹ", "ǹ"), ("N̂", "n̂"), ("Ň", "ň"), ("N̄", "n̄"), ("N̍", "n̍"), ("N̋", "n̋"),
            // M
            ("Ḿ", "ḿ"), ("M̀", "m̀"), ("M̂", "m̂"), ("M̌", "m̌"), ("M̄", "m̄"), ("M̍", "m̍"), ("M̋", "m̋"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .lowercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .poj,
            )
            XCTAssertEqual(result, expected, "POJ lowercase: \(input) should be \(expected), got \(result)")
        }
    }

    // MARK: - TL Tone Letter Mapping Tests

    /// 測試 TL 特有聲調字母的大寫轉換
    func testTL_uppercaseToneLetters() {
        let testCases: [(input: String, expected: String)] = [
            // TL 特有的 9 調
            ("a̋", "A̋"), ("e̋", "E̋"), ("i̋", "I̋"), ("ő", "Ő"), ("ű", "Ű"),
            // oo (TL 特有)
            ("óo", "Óo"), ("òo", "Òo"), ("ôo", "Ôo"), ("ǒo", "Ǒo"), ("ōo", "Ōo"), ("o̍o", "O̍o"), ("őo", "Őo"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .uppercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .tl,
            )
            XCTAssertEqual(result, expected, "TL uppercase: \(input) should be \(expected), got \(result)")
        }
    }

    /// 測試 TL 特有聲調字母的小寫轉換
    func testTL_lowercaseToneLetters() {
        let testCases: [(input: String, expected: String)] = [
            // TL 特有的 9 調
            ("A̋", "a̋"), ("E̋", "e̋"), ("I̋", "i̋"), ("Ő", "ő"), ("Ű", "ű"),
            // oo (TL 特有)
            ("Óo", "óo"), ("Òo", "òo"), ("Ôo", "ôo"), ("Ǒo", "ǒo"), ("Ōo", "ōo"), ("O̍o", "o̍o"), ("Őo", "őo"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .lowercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .tl,
            )
            XCTAssertEqual(result, expected, "TL lowercase: \(input) should be \(expected), got \(result)")
        }
    }

    // MARK: - POJ o͘ (o with dot) Tests

    /// 測試 POJ o͘ 聲調字母的大寫轉換
    func testPOJ_oDot_uppercase() {
        let testCases: [(input: String, expected: String)] = [
            ("ó͘", "Ó͘"), ("ò͘", "Ò͘"), ("ô͘", "Ô͘"), ("ǒ͘", "Ǒ͘"), ("ō͘", "Ō͘"), ("o̍͘", "O̍͘"), ("ŏ͘", "Ŏ͘"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .uppercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .poj,
            )
            XCTAssertEqual(result, expected, "POJ o͘ uppercase: \(input) should be \(expected), got \(result)")
        }
    }

    /// 測試 POJ o͘ 聲調字母的小寫轉換
    func testPOJ_oDot_lowercase() {
        let testCases: [(input: String, expected: String)] = [
            ("Ó͘", "ó͘"), ("Ò͘", "ò͘"), ("Ô͘", "ô͘"), ("Ǒ͘", "ǒ͘"), ("Ō͘", "ō͘"), ("O̍͘", "o̍͘"), ("Ŏ͘", "ŏ͘"),
        ]

        for (input, expected) in testCases {
            let result = CaseTransformer.transformForInput(
                input,
                letterCase: .lowercased,
                isAutoCapitalizationEnabled: true,
                inputMode: .poj,
            )
            XCTAssertEqual(result, expected, "POJ o͘ lowercase: \(input) should be \(expected), got \(result)")
        }
    }

    // MARK: - INVARIANT wrappers — Phase 0 §9

    func test_INVARIANT_case_transformer_is_deterministic() {
        // Non-determinism (e.g. reading global settings) would break cross-platform parity.
        let fixtures: [(text: String, input: String, autoCap: Bool, mode: InputMode)] = [
            ("台語", "T", true, .tl),
            ("tâi-gí", "t", false, .tl),
            ("Guá", "G", true, .poj),
            ("hō-gē", "H", true, .poj),
        ]
        for (text, input, autoCap, mode) in fixtures {
            let first = CaseTransformer.capitalizeCandidate(
                text,
                basedOn: input,
                isAutoCapitalizationEnabled: autoCap,
                inputMode: mode,
            )
            let second = CaseTransformer.capitalizeCandidate(
                text,
                basedOn: input,
                isAutoCapitalizationEnabled: autoCap,
                inputMode: mode,
            )
            XCTAssertEqual(first, second, "Non-determinism on (\(text), \(input), \(autoCap), \(mode))")
        }
    }

    func test_INVARIANT_case_transformer_honors_auto_cap_flag() {
        let on = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "T",
            isAutoCapitalizationEnabled: true,
            inputMode: .tl,
        )
        let off = CaseTransformer.capitalizeCandidate(
            "tâi-gí",
            basedOn: "T",
            isAutoCapitalizationEnabled: false,
            inputMode: .tl,
        )
        XCTAssertNotEqual(on, off, "autoCap flag must change output when input signals capitalization")
    }
}
