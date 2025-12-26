import Foundation
import SwiftUI

/// 本地化文字結構（簡化版：僅支援漢字）
struct LocalizedText {
    let hanji: String

    init(hanji: String) {
        self.hanji = hanji
    }

    /// 便利初始化（直接傳入字串）
    init(_ text: String) {
        self.hanji = text
    }
}

/// 語言管理器（簡化版）
class LanguageManager: ObservableObject {
    static let shared = LanguageManager()

    private init() {}

    func text(_ localizedText: LocalizedText) -> String {
        localizedText.hanji
    }
}

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
