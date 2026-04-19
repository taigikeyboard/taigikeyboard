import Foundation

extension NextWordService.AssociationEntry: Identifiable {
    public var id: String {
        "\(prevWord)\t\(prevTl)\t\(nextWord)\t\(nextTl)"
    }
}
