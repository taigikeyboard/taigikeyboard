import SwiftUI

/// Reusable search bar anchored at the bottom of a list via `.safeAreaInset`.
///
/// Usage:
/// ```
/// .safeAreaInset(edge: .bottom) {
///     SearchBar(text: $filterText, placeholder: "Search")
/// }
/// ```
struct SearchBar: View {
    @Binding var text: String
    let placeholder: String

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField(placeholder, text: $text)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                if !text.isEmpty {
                    Button {
                        text = ""
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color(.tertiarySystemFill))
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
        .background(Color(.systemBackground))
        .padding(.bottom, 8)
    }
}
