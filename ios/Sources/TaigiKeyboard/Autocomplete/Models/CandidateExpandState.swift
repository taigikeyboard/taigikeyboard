import Combine
import SwiftUI

/// 候選詞展開狀態
///
/// 控制候選詞視圖的展開和收合狀態。
class CandidateExpandState: ObservableObject {
    /// 是否處於展開狀態
    @Published var isExpanded = false

    /// 切換展開/收合狀態
    func toggle() {
        isExpanded.toggle()
    }

    /// 強制收合
    func collapse() {
        isExpanded = false
    }
}
