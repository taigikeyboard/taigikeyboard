import SwiftUI

extension EnvironmentValues {
    /// 候選詞 UI 主題（尺寸 + 顏色），由組合根注入。
    @Entry var candidateTheme: CandidateTheme = .standard
}

extension View {
    /// 套用候選詞 UI 主題
    ///
    /// - Parameter theme: 要套用的主題
    /// - Returns: 套用主題後的視圖
    func candidateTheme(_ theme: CandidateTheme) -> some View {
        environment(\.candidateTheme, theme)
    }
}
