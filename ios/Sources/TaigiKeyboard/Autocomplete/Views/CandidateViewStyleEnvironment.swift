// SwiftUI environment injection point + convenience modifier for CandidateView.Style.

import SwiftUI

extension EnvironmentValues {
    @Entry var candidateViewStyle: CandidateView.Style = .standard
}

extension View {
    func candidateViewStyle(_ style: CandidateView.Style) -> some View {
        environment(\.candidateViewStyle, style)
    }
}
