// Makes `NextWordService.AssociationEntry` Identifiable for SwiftUI ForEach,
// via a tab-joined composite of its four fields.

import Foundation

extension NextWordService.AssociationEntry: Identifiable {
    public var id: String {
        "\(prevWord)\t\(prevTl)\t\(nextWord)\t\(nextTl)"
    }
}
