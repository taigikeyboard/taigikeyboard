import SwiftUI

extension EnvironmentValues {
    /// Candidate UI theme (size + colors), injected by the composition root.
    @Entry var candidateTheme: CandidateTheme = .standard
}

extension View {
    func candidateTheme(_ theme: CandidateTheme) -> some View {
        environment(\.candidateTheme, theme)
    }
}
