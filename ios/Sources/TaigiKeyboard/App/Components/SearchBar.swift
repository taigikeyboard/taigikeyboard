// 中文: 可重用的搜尋列元件,以 .safeAreaInset 釘在 List 底部。

import SwiftUI

/// Reusable search bar anchored at the bottom of a list via `.safeAreaInset`.
///
/// Usage:
/// ```
/// .safeAreaInset(edge: .bottom) {
///     SearchBar(text: $filterText, placeholder: "Search")
/// }
/// ```
// 中文: 底部固定式搜尋列。text 雙向綁定,placeholder 為空白時顯示的提示。
struct SearchBar: View {
    @Binding var text: String
    let placeholder: String

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack {
                Image(latinSystemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField(placeholder, text: $text)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                if !text.isEmpty {
                    Button {
                        text = ""
                    } label: {
                        Image(latinSystemName: "xmark.circle.fill")
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
