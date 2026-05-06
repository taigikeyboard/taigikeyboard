// 中文: CandidateView.Style 的 SwiftUI environment 注入點與便利 modifier。

import SwiftUI

extension EnvironmentValues {
    /// 候選詞視圖樣式
    @Entry var candidateViewStyle: CandidateView.Style = .standard
}

extension View {
    /// 套用候選詞視圖樣式
    ///
    /// - Parameter style: 要套用的樣式
    /// - Returns: 套用樣式後的視圖
    func candidateViewStyle(_ style: CandidateView.Style) -> some View {
        environment(\.candidateViewStyle, style)
    }
}
