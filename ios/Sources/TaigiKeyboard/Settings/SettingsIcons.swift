// 設定頁開關列使用的 SF Symbol 圖示常數,與 Android SettingsIcons.kt 對齊。

import Foundation

// MARK: - 設定頁 SF Symbol 圖示

// 對應 Android SettingsIcons.kt

enum SettingsIcons {
    // 候選詞顯示模式 (漢羅並排 / 羅馬字) 選擇列圖示。
    static let candidateDisplayMode = "character.textbox"
    // 「同時輸出漢羅雙寫」開關圖示。
    static let isOutputBothScripts = "character.book.closed"
    // 顯示當咧拍的字 (§34/S22) 開關圖示。
    static let literalRomanCandidate = "abc"
    // 自動大寫開關圖示。
    static let autoCapitalization = "textformat.size"
    // 自動空格開關圖示。
    static let autoSpace = "space"
    // 工具列展開/收合開關圖示。
    static let toolbar = "menubar.rectangle"
    // 地球鍵 (切換鍵盤) 開關圖示。
    static let globeKey = "globe"
    // 鍵盤音效回饋開關圖示。
    static let soundFeedback = "speaker.wave.2"
    // 鍵盤震動回饋開關圖示。
    static let vibrationFeedback = "iphone.radiowaves.left.and.right"
}
