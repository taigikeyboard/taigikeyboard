import Foundation

/// 台語輸入模式
enum InputMode: String, CaseIterable {
    case poj  // 白話字（Pe̍h-ōe-jī）
    case tl   // 台羅（Tâi-lô）
}

/// 字型選項
enum FontType: String, CaseIterable {
    case system     // 系統
    case openHuninn // 粉圓
    case iansui     // 芫荽
}
