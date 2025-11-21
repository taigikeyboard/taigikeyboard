import OSLog
import SwiftUI

/// 候選詞展開狀態管理類別
/// 控制候選詞視圖的展開/收合狀態
class CandidateExpandState: ObservableObject {
    /// 是否處於展開狀態
    @Published var isExpanded = false

    #if DEBUG
        private static var instanceCount = 0
        private let instanceId: Int
        private let logger = Logger(
            subsystem: LexiconConstants.Logging.subsystem,
            category: "CandidateExpandState",
        )
    #endif

    init() {
        #if DEBUG
            Self.instanceCount += 1
            instanceId = Self.instanceCount
        #endif
    }

    deinit {
        #if DEBUG
            Self.instanceCount -= 1
        #endif
    }

    /// 切換展開/收合狀態
    func toggle() {
        isExpanded.toggle()
    }

    /// 強制收合
    func collapse() {
        isExpanded = false
    }
}
