import SwiftUI

/// Fixed candidate-view geometry constants.
///
/// View-agnostic constants unaffected by user appearance settings. Values
/// that depend on `candidateTextSizeScale` / `colorSettings` come from
/// `CandidateTheme` (via the SwiftUI environment) instead.
enum CandidateViewModels {
    enum UI {
        static let buttonSpacing: CGFloat = 14
        static let maxDisplayCount: Int = 200

        static let expandedRowSpacing: CGFloat = 6
        static let expandedItemSpacing: CGFloat = 4
        static let expandedMinRowHeight: CGFloat = 35
        static let expandedButtonVerticalPadding: CGFloat = 7
    }

    enum Spacing {
        static let small: CGFloat = 2
    }

    enum Colors {
        static let separatorColor: Color = .init(.separator)
    }
}
