import SwiftUI

/// 設定區塊容器
struct SettingsSection<Content: View>: View {
    let titleContent: LocalizedText?
    let content: Content

    init(titleContent: LocalizedText? = nil, @ViewBuilder content: () -> Content) {
        self.titleContent = titleContent
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let titleContent {
                LocalizedTextView(titleContent)
                    .font(Font.Theme.headline)
                    .foregroundColor(Color.Theme.textPrimary)
                    .padding(.horizontal, 24)
                    .padding(.top, 8)
            }

            VStack(spacing: 0) {
                content
            }
            .themedCard()
        }
    }
}

/// 設定頁 Toggle 開關項目
struct SettingsToggleItem: View {
    let titleContent: LocalizedText
    @Binding var isOn: Bool
    let isFirst: Bool
    let isLast: Bool
    let onChange: ((Bool) -> Void)?
    @State private var isPressed = false

    init(
        titleContent: LocalizedText,
        isOn: Binding<Bool>,
        isFirst: Bool = false,
        isLast: Bool = false,
        onChange: ((Bool) -> Void)? = nil
    ) {
        self.titleContent = titleContent
        _isOn = isOn
        self.isFirst = isFirst
        self.isLast = isLast
        self.onChange = onChange
    }

    var body: some View {
        HStack(spacing: 16) {
            LocalizedTextView(titleContent)
                .font(Font.Theme.body)
                .foregroundColor(Color.Theme.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading)

            Toggle("", isOn: $isOn)
                .labelsHidden()
                .tint(Color.Theme.accent)
                .onChange(of: isOn) { _, newValue in
                    onChange?(newValue)
                }
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 12)
        .frame(minHeight: 48)
        .background(
            Rectangle()
                .fill(isPressed ? Color.Theme.accent.opacity(0.08) : Color.clear)
                .animation(.easeInOut(duration: 0.15), value: isPressed)
        )
        .overlay(
            VStack {
                Spacer()
                if !isLast {
                    Rectangle()
                        .fill(Color.Theme.textSecondary.opacity(0.15))
                        .frame(height: 0.5)
                        .padding(.leading, 60)
                }
            }
        )
        .contentShape(Rectangle())
        .pressable($isPressed, scaleEffect: 0.98, minimumDistance: 50)
        .animation(.spring(response: 0.4, dampingFraction: 0.8), value: isPressed)
    }
}

/// 設定頁動作按鈕
struct SettingsActionButton: View {
    let titleContent: LocalizedText
    let isFirst: Bool
    let isLast: Bool
    let action: () -> Void
    @State private var isPressed = false

    init(
        titleContent: LocalizedText,
        isFirst: Bool = false,
        isLast: Bool = false,
        action: @escaping () -> Void
    ) {
        self.titleContent = titleContent
        self.isFirst = isFirst
        self.isLast = isLast
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 16) {
                LocalizedTextView(titleContent)
                    .font(Font.Theme.body)
                    .foregroundColor(Color.Theme.textPrimary)
                    .frame(maxWidth: .infinity, alignment: .leading)

                Image(systemName: "chevron.right")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundColor(Color.Theme.textSecondary)
                    .opacity(0.5)
                    .offset(x: isPressed ? 2 : 0)
                    .animation(.spring(response: 0.4, dampingFraction: 0.8), value: isPressed)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .frame(minHeight: 48)
            .background(
                Rectangle()
                    .fill(isPressed ? Color.Theme.accent.opacity(0.08) : Color.clear)
                    .animation(.easeInOut(duration: 0.15), value: isPressed)
            )
            .overlay(
                VStack {
                    Spacer()
                    if !isLast {
                        Rectangle()
                            .fill(Color.Theme.textSecondary.opacity(0.15))
                            .frame(height: 0.5)
                            .padding(.leading, 60)
                    }
                }
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(PlainButtonStyle())
        .pressable($isPressed, scaleEffect: 0.98, minimumDistance: 50)
        .animation(.spring(response: 0.4, dampingFraction: 0.8), value: isPressed)
    }
}
