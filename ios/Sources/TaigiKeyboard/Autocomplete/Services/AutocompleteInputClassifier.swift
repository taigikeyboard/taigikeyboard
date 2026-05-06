// 中文: rawInput 分類器的 Swift 端薄包裝 — 把字串送進 Rust 引擎判斷輸入型別。

import Foundation

/// 依 rawInput 判斷輸入型別並建立 Trie 搜尋鍵。
///
/// Thin wrapper over `RustEngineBridge.classifyInput`. Precedence contract:
/// `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
enum AutocompleteInputClassifier {
    // 中文: 分類結果 — 對應的 InputType 加上要丟進 Trie 的 searchKey。
    struct Classification: Equatable {
        let inputType: InputType
        let searchKey: String
    }

    /// `classify` 把 rawInput 轉成 `(inputType, searchKey)` 對。
    /// - rawInput 例：`gua2` / `guá` / `我` / TPS 符號
    static func classify(rawInput: String) -> Classification {
        let result = RustEngineBridge.classifyInput(rawInput)
        return Classification(inputType: result.inputType, searchKey: result.searchKey)
    }
}
