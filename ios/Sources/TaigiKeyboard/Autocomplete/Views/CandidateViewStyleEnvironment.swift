import SwiftUI

// `CandidateView.Style` 的 SwiftUI Environment 掛接。
//
// 以 `View.candidateViewStyle(_:)` modifier 對整個候選詞層套用樣式，
// 子視圖（`ToolShortcutsToolbar`、`CandidateSuggestionsRow`、
// `CandidateButtonView`、`ExpandedCandidateOverlay`）透過
// `@Environment(\.candidateViewStyle)` 讀取。

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
