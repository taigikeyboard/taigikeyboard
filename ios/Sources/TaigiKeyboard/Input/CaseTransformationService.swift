import Foundation
import KeyboardKit

/// 大小寫轉換服務
///
/// 提供統一的大小寫轉換邏輯，支援：
/// - 使用者輸入時的字元轉換
/// - 候選詞的首字母大寫
/// - 聲調字母的正確轉換（POJ/TL）
///
/// 大小寫狀態完全由 KeyboardKit 管理，本服務只負責字元轉換。
enum CaseTransformer {

    // MARK: - Public API

    /// 輸入時轉換字元（ActionHandler 使用）
    ///
    /// - Parameters:
    ///   - char: 要轉換的字元
    ///   - keyboardCase: 當前鍵盤大小寫狀態（由 KeyboardKit 管理）
    ///   - isAutoCapitalizationEnabled: 是否啟用自動大寫（未使用，保留供未來擴展）
    ///   - inputMode: 輸入模式（POJ/TL）
    /// - Returns: 轉換後的字元
    static func transformForInput(
        _ char: String,
        keyboardCase: Keyboard.KeyboardCase,
        isAutoCapitalizationEnabled: Bool,
        inputMode: InputMode
    ) -> String {
        // 直接使用 KeyboardKit 管理的 keyboardCase 狀態
        return transform(char, to: keyboardCase, mode: inputMode)
    }

    /// 候選詞首字母大寫（AutocompleteService 使用）
    ///
    /// - Parameters:
    ///   - text: 候選詞文字
    ///   - input: 使用者輸入（用於判斷大小寫）
    ///   - isAutoCapitalizationEnabled: 是否啟用自動大寫
    ///   - inputMode: 輸入模式（POJ/TL）
    /// - Returns: 處理後的候選詞
    static func capitalizeCandidate(
        _ text: String,
        basedOn input: String,
        isAutoCapitalizationEnabled: Bool,
        inputMode: InputMode
    ) -> String {
        guard isAutoCapitalizationEnabled else {
            return text
        }

        guard !input.isEmpty, !text.isEmpty else {
            return text
        }

        let firstInputChar = input.first!
        guard firstInputChar.isUppercase else {
            return text
        }

        let firstTextChar = text.first!
        guard firstTextChar.isLetter else {
            return text
        }

        let capitalizedFirst = ToneUtilities.uppercaseToneLetter(
            String(firstTextChar),
            mode: inputMode
        )
        let rest = String(text.dropFirst())

        return capitalizedFirst + rest
    }

    // MARK: - Private Helpers

    /// 統一的字元轉換（使用聲調對照表）
    private static func transform(
        _ char: String,
        to targetCase: Keyboard.KeyboardCase,
        mode: InputMode
    ) -> String {
        switch targetCase {
        case .capsLocked:
            // Caps Lock：全部大寫（如 "tsh" → "TSH"）
            return ToneUtilities.uppercaseToneLetter(char, mode: mode)
        case .uppercased:
            // 句首大寫：僅首字母大寫（如 "tsh" → "Tsh"）
            return capitalizeFirstLetter(char, mode: mode)
        case .lowercased:
            return ToneUtilities.lowercaseToneLetter(char, mode: mode)
        @unknown default:
            return char.lowercased()
        }
    }

    /// 僅首字母大寫（支援聲調字母）
    private static func capitalizeFirstLetter(_ char: String, mode: InputMode) -> String {
        guard let first = char.first else { return char }
        let firstUpper = ToneUtilities.uppercaseToneLetter(String(first), mode: mode)
        let rest = String(char.dropFirst()).lowercased()
        return firstUpper + rest
    }
}
