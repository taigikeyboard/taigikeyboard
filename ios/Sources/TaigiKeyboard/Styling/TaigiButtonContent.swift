import KeyboardKit
import SwiftUI

/// 自訂按鈕內容
///
/// 整合 ButtonImageProvider、ButtonTextProvider、ButtonFontProvider，
/// 依序判斷應顯示的內容：圖片 > 文字 > 標準內容。
///
/// - Note: `keyboardContext` 必須用 `@ObservedObject`，否則不會響應 `isComposingText` 等狀態變化
struct TaigiButtonContent<StandardContent: View>: View {
    let action: KeyboardAction
    @ObservedObject var keyboardContext: KeyboardContext
    let standardContent: StandardContent

    private var textProvider: ButtonTextProvider {
        ButtonTextProvider(keyboardContext: keyboardContext)
    }

    private var imageProvider: ButtonImageProvider {
        ButtonImageProvider(keyboardContext: keyboardContext)
    }

    private var fontProvider: ButtonFontProvider {
        ButtonFontProvider(keyboardContext: keyboardContext)
    }

    var body: some View {
        // 優先顯示自訂圖片
        if let image = imageProvider.buttonImage(for: action) {
            image
        }
        // 其次顯示自訂文字（套用自訂字型）
        else if let text = textProvider.buttonText(for: action) {
            Text(text)
                .font(fontProvider.buttonKeyboardFont(for: action).font)
        }
        // 否則使用標準內容
        else {
            standardContent
        }
    }
}
