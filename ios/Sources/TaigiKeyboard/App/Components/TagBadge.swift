import SwiftUI

/// Small grey capsule tag beside a list row's text — dictionary source tags
/// and the §50 auto-learned badge share it.
struct TagBadge: View {
    let text: String

    var body: some View {
        Text(text)
            .font(AppStyle.captionFont)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(Color(.systemGray5))
            .clipShape(RoundedRectangle(cornerRadius: 4))
    }
}
