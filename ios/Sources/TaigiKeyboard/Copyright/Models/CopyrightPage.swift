import SwiftUI

/// 版權頁面資料模型
struct CopyrightPage: Identifiable {
    let id: Int
    let title: LocalizedText
    let description: LocalizedText
    let accentColor: Color
    let licenseDescription: LocalizedText
    let buttons: [CopyrightButton]
}

/// 版權頁面按鈕資料
struct CopyrightButton {
    let text: LocalizedText
    let url: String
}
