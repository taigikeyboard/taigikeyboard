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
            .padding(.horizontal, AppStyle.innerHorizontalPadding)
            .padding(.vertical, AppStyle.verticalPadding)
            .background(Color(.tertiarySystemFill))
            .clipShape(RoundedRectangle(cornerRadius: AppStyle.cardCornerRadius, style: .continuous))
            .padding(.horizontal, AppStyle.horizontalPadding)
            .padding(.vertical, AppStyle.verticalPadding)
        }
        .background(Color(.systemBackground))
        .padding(.bottom, 8)
    }
}
