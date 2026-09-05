// 鍵盤擴充與主 App 共用的字型工具集中地。
// 兩條字型解析路徑 — 鍵面字 (ButtonFontProvider,走 per-render snapshot.fontType) 與
// 工具列/候選 UI (globalFont / globalUIFont,走 SharedSettings.shared.fontType)。
// 字型是全域設定(非每主題),兩條路徑皆讀同一個全域 fontType。

import SwiftUI
import UIKit

/// Font utilities for the keyboard extension and main app.
///
/// Two font resolution paths exist due to KeyboardKit's architecture:
/// - **Key rendering**: `ButtonFontProvider` (per-render `SettingsSnapshot`, called via `TaigiButtonContent`)
/// - **Toolbar / candidate UI**: `globalFont` / `globalUIFont` below (reads `SharedSettings.shared` directly,
///   because these are called from ~15 SwiftUI views outside KeyboardKit's key pipeline)
///
/// Both paths resolve the same user-selected font (System / Open Huninn / Iansui)
/// using `FontType.customFontName` as the shared font-name source.
///
/// Callers: `CandidateView`, `CandidateCellHelper`, `ExpandedCandidateOverlay`,
/// `SymbolSelectionOverlay`, `SettingsSelectionOverlay`, `TaigiButtonContent` (space label),
/// `AppStyle`, `TaigiKeyboardApp`, `KeyboardPreviewPanel`
// 字型工具命名空間 — 自訂字型的 PostScript 名稱常數 + 全域字型解析。
enum KeyboardFonts {
    /// PostScript font name for jf-openhuninn (粉圓)
    static let openHuninnFontName = "jf-openhuninn-2.1"
    /// PostScript font name for Iansui (芫荽)
    static let iansuiFontName = "Iansui-Regular"
    /// PostScript font name for GenYoMin (源樣明體)
    static let genYoMinFontName = "GenYoMin2TW-R"
    /// PostScript font name for GenYoGothic (源樣烏體)
    static let genYoGothicFontName = "GenYoGothic2TW-R"

    /// SwiftUI Font based on the user's font setting.
    /// Used by toolbar buttons and candidate views (not keyboard keys).
    // 工具列按鈕、候選詞 view 的 SwiftUI 字型;鍵面字走 ButtonFontProvider,不走這裡。
    static func globalFont(size: CGFloat) -> Font {
        if let name = SharedSettings.shared.fontType.customFontName {
            return Font.custom(name, size: size)
        }
        return Font.system(size: size)
    }

    /// UIKit UIFont based on the user's font setting.
    /// Used where UIKit measurement is needed (e.g. candidate cell width calculation).
    // 候選詞 cell 寬度量測等需要 UIKit UIFont 的場合使用。
    static func globalUIFont(size: CGFloat) -> UIFont {
        if let name = SharedSettings.shared.fontType.customFontName {
            return UIFont(name: name, size: size) ?? UIFont.systemFont(ofSize: size)
        }
        return UIFont.systemFont(ofSize: size)
    }
}
