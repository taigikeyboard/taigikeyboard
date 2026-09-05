// 單次渲染週期使用的設定不可變快照。把 ~50 個鍵渲染需要的設定值打包,避免重複讀 UserDefaults。

import CoreGraphics
import Foundation

/// Immutable snapshot of settings needed during a single render cycle.
/// Avoids repeated UserDefaults reads when rendering ~50 keys.
// 渲染熱路徑用的設定快照。包含的欄位都是 ~50 個按鍵渲染時會反覆讀到的設定。
// 渲染週期外請走 live-read (KeyboardEnvironment / EngineSettingsProvider) 取得最新值。
struct SettingsSnapshot {
    // 目前輸入模式。
    let inputMode: InputMode
    // 鍵盤字體選擇。
    let fontType: FontType
    // 鍵盤排版。
    let keyboardLayoutType: KeyboardLayoutType
    // 翻譯方向是否反轉。
    let isTranslateSwapped: Bool
    // TPS 中 "or" 是否映射到ㄜ。
    let isTpsOrMappedToER: Bool
    // 鍵帽字體縮放係數。
    let keyFontSizeScale: CGFloat
    // 鍵帽圓角半徑。
    let keyCornerRadius: CGFloat
    // 鍵盤顏色組合。
    let colorSettings: KeyboardColorSettings
    // 候選列文字縮放係數(每主題)。
    let candidateTextSizeScale: CGFloat
    // 鍵帽邊框寬度(每主題)。
    let keyBorderWidth: CGFloat
    // 鍵帽陰影(每主題,三態):nil = 沿用 KeyboardKit 標準陰影(default / built-in 主題,= HEAD 觀感);
    // 0 = 明確無陰影(自訂主題 slider 拉到 0);>0 = 明確指定的陰影 point size。
    let keyShadowIntensity: CGFloat?
}
