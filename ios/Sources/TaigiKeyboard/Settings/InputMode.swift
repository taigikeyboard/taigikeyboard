import Foundation

/// 輸入模式
enum InputMode: String, CaseIterable {
    case poj      // 白話字（Pe̍h-ōe-jī）
    case tl       // 台羅（Tâi-lô）
    case english  // 英文

    /// 是否為台語輸入模式（POJ 或 TL）
    var isTaigi: Bool {
        self == .poj || self == .tl
    }
}

/// 字型選項
enum FontType: String, CaseIterable {
    case system     // 系統
    case openHuninn // 粉圓
    case iansui     // 芫荽
}
