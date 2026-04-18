import Foundation

// Pure-logic phase types for AutocompleteService.
//
// These types are stateless and do not depend on KeyboardKit or service
// state, so they can be unit-tested independently. Naming mirrors the
// Android `TaigiAutocompleteService` (`determineInputType` /
// `applyContextBoost`) to ease future shared-core extraction.
//
// Note: not a shared-core candidate — still depends on `InputType`,
// `CandidateProcessor.isHanzi`, `InputNormalizer.hasToneMarks`,
// `TPSTables.containsTPS`, and `TPSToTL.convert`.

/// 依 rawInput 判斷輸入型別並建立 Trie 搜尋鍵。
enum AutocompleteInputClassifier {
    struct Classification: Equatable {
        let inputType: InputType
        let searchKey: String
    }

    /// `classify` 把 rawInput 轉成 `(inputType, searchKey)` 對。
    /// - rawInput 例：`gua2` / `guá` / `我` / TPS 符號
    static func classify(rawInput: String) -> Classification {
        Classification(
            inputType: determineInputType(rawInput),
            searchKey: buildSearchKey(from: rawInput),
        )
    }

    /// 判斷輸入文字的類型（漢字 / 帶聲調羅馬字 / 無聲調羅馬字）。
    static func determineInputType(_ text: String) -> InputType {
        if CandidateProcessor.isHanzi(text) {
            return .hanzi
        }
        if InputNormalizer.hasToneMarks(text) {
            return .romanWithTone
        }
        if containsNumericTone(text) {
            return .romanWithTone
        }
        return .romanWithoutTone
    }

    /// 檢查文字是否包含數字聲調（2, 3, 5, 6, 7, 8, 9）。
    /// 排除 1, 4, 0：1 / 4 是無調號聲調，0 是無效輸入。
    static func containsNumericTone(_ text: String) -> Bool {
        text.contains { char in
            char.isNumber && char != "1" && char != "4" && char != "0"
        }
    }

    /// Build a Trie-compatible search key from raw input.
    /// TPS 輸入先轉為 TL 羅馬字；其他原樣輸出。
    static func buildSearchKey(from rawInput: String) -> String {
        TPSTables.containsTPS(rawInput)
            ? TPSToTL.convert(rawInput)
            : rawInput
    }
}
