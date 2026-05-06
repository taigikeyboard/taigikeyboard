// 中文: 讓 NextWordService.AssociationEntry 在 SwiftUI ForEach 中可作為 Identifiable 使用。
// 中文: 以四欄 (prevWord/prevTl/nextWord/nextTl) tab 串接組成唯一 id。

import Foundation

extension NextWordService.AssociationEntry: Identifiable {
    // 中文: 由四個欄位組合的複合 id,僅供 SwiftUI ForEach diffing 使用。
    public var id: String {
        "\(prevWord)\t\(prevTl)\t\(nextWord)\t\(nextTl)"
    }
}
