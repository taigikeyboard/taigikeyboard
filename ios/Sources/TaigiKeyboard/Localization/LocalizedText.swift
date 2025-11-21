import Foundation

/// 顯示語言選項
enum DisplayLanguage: Equatable {
    case hanji
    case poj
    case tl
}

/// 多語系文字結構（漢字、白話字、台羅）
struct LocalizedText {
    let hanji: String
    let poj: String
    let tl: String

    /// 根據語言設定取得對應文字
    func text(for language: DisplayLanguage) -> String {
        switch language {
        case .hanji:
            hanji
        case .poj:
            poj
        case .tl:
            tl
        }
    }
}

/// 語言管理器（負責根據設定切換顯示語言）
class LanguageManager: ObservableObject {
    static let shared = LanguageManager()

    @Published var currentDisplayLanguage: DisplayLanguage = .hanji

    private init() {
        updateDisplayLanguage()
    }

    /// 根據使用者設定更新顯示語言
    func updateDisplayLanguage() {
        let settings = SharedSettings.shared

        let inputModeValue = settings.inputMode.rawValue
        let showHanji = settings.showHanjiMode

        let newLanguage: DisplayLanguage = if showHanji {
            .hanji
        } else {
            inputModeValue == "poj" ? .poj : .tl
        }

        if newLanguage != currentDisplayLanguage {
            currentDisplayLanguage = newLanguage

            NotificationCenter.default.post(name: .languageDidChange, object: nil)
        }
    }

    func text(_ localizedText: LocalizedText) -> String {
        localizedText.text(for: currentDisplayLanguage)
    }
}

import SwiftUI

extension View {
    func withLanguageEnvironment() -> some View {
        environmentObject(LanguageManager.shared)
    }
}

struct LocalizedTextView: View {
    let text: LocalizedText
    @ObservedObject private var languageManager = LanguageManager.shared

    init(_ text: LocalizedText) {
        self.text = text
    }

    var body: some View {
        Text(languageManager.text(text))
    }
}

extension Notification.Name {
    static let languageDidChange = Notification.Name("languageDidChange")
}
