// 中文: 候選詞列上的工具快捷鍵 toolbar — `+` 切換按鈕與 9 個等寬功能按鈕(輸入模式 / 符號 / 佈局 / globe / 收合 / 設定)。

import KeyboardKit
import SwiftUI

/// 候選詞列工具快捷鍵工具列
///
/// 永遠顯示 `+` toggle 按鈕；展開時顯示 9 個等寬快捷按鈕
/// （4 個輸入模式切換 + 符號 / 佈局 / globe / 收合鍵盤 / 設定）。
struct ToolShortcutsToolbar: View {
    @Binding var isExpanded: Bool
    let currentInputMode: InputMode
    let onInputModeChange: (InputMode) -> Void
    let onSymbolTap: () -> Void
    let onLayoutTap: () -> Void
    let onDismissKeyboard: () -> Void
    let onSettingsTap: () -> Void

    @Environment(\.candidateTheme) private var theme
    // Resolves smartbar a11y labels under the picker's display language (mirrors Android
    // InputView.applyAccessibilityStrings). Reading lang.string(_:) in body registers the live-switch.
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        HStack(spacing: 0) {
            toggleButton

            if isExpanded {
                expandedButtons
                    .transition(.move(edge: .bottom))
            }
        }
    }

    /// `+` 展開/收合按鈕（展開時旋轉 45° 變 `×`）
    private var toggleButton: some View {
        Button(action: {
            withAnimation(.easeInOut(duration: 0.2)) {
                isExpanded.toggle()
            }
        }) {
            Image(latinSystemName: "plus")
                .font(KeyboardFonts.globalFont(size: 16))
                .fontWeight(.light)
                .foregroundColor(theme.primaryTextColor)
                .rotationEffect(.degrees(isExpanded ? 45 : 0))
                .animation(.easeInOut(duration: 0.2), value: isExpanded)
                .frame(width: 36, height: theme.height)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .offset(y: 7)
        .accessibilityLabel(isExpanded ? "收合工具列" : "展開工具列")
    }

    /// 9 個等寬快捷按鈕
    private var expandedButtons: some View {
        HStack(spacing: 0) {
            inputModeButton(mode: .poj, label: "POJ")
            inputModeButton(mode: .tl, label: "TL")
            inputModeButton(mode: .english, label: "EN")
            inputModeButton(mode: .tps, label: "TPS")

            ToolShortcutButton(
                systemName: "number",
                accessibilityLabel: lang.string(.keyboardSymbolPanel),
                accessibilityHint: "點擊以開啟符號選擇面板",
                action: onSymbolTap,
            )

            ToolShortcutButton(
                systemName: "photo",
                accessibilityLabel: lang.string(.keyboardSwitchLayout),
                accessibilityHint: "點擊以開啟佈局選擇面板",
                action: onLayoutTap,
            )

            globeButton

            ToolShortcutButton(
                systemName: "keyboard.chevron.compact.down",
                accessibilityLabel: lang.string(.keyboardDismissKeyboard),
                accessibilityHint: "Tap to dismiss keyboard",
                action: onDismissKeyboard,
            )

            ToolShortcutButton(
                systemName: "gearshape",
                accessibilityLabel: lang.string(.keyboardSettings),
                accessibilityHint: "點擊以開啟鍵盤設定",
                action: onSettingsTap,
            )
        }
        .offset(y: 7)
    }

    /// 單個輸入模式按鈕
    private func inputModeButton(mode: InputMode, label: String) -> some View {
        let isSelected = currentInputMode == mode
        return Button(action: { onInputModeChange(mode) }) {
            Text(label)
                .font(KeyboardFonts.globalFont(size: 16))
                .fontWeight(isSelected ? .semibold : .regular)
                .foregroundColor(isSelected ? .white : theme.primaryTextColor)
                .lineLimit(1)
                .fixedSize(horizontal: true, vertical: false)
                .padding(.horizontal, 6)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isSelected ? Color.accentColor : Color.clear),
                )
                .animation(nil, value: isSelected)
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
        .accessibilityLabel("\(label) 輸入模式")
        .accessibilityHint(isSelected ? "目前選擇" : "點擊切換至 \(label) 模式")
    }

    /// 切換鍵盤的 globe 按鈕（tap: 下一個鍵盤, long-press: 鍵盤選擇器）
    private var globeButton: some View {
        Keyboard.NextKeyboardButton {
            ToolShortcutIcon(systemName: "globe")
        }
        .frame(maxWidth: .infinity)
        .accessibilityLabel(lang.string(.keyboardSwitchInputMethod))
        .accessibilityHint("點擊切換下一個鍵盤，長按選取鍵盤")
    }
}

// MARK: - Icon 快捷按鈕

/// 符號 / 佈局 / 收合鍵盤 / 設定 四個 icon 按鈕共用模板
private struct ToolShortcutButton: View {
    let systemName: String
    let accessibilityLabel: String
    let accessibilityHint: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ToolShortcutIcon(systemName: systemName)
        }
        .buttonStyle(ToolShortcutButtonStyle())
        .frame(maxWidth: .infinity)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityHint(accessibilityHint)
    }
}

/// 共用 icon 排版：`ToolShortcutButton` 與 globe `NextKeyboardButton` 共用視覺。
private struct ToolShortcutIcon: View {
    let systemName: String

    @Environment(\.candidateTheme) private var theme

    var body: some View {
        Image(latinSystemName: systemName)
            .font(KeyboardFonts.globalFont(size: 18))
            .fontWeight(.light)
            .foregroundColor(theme.primaryTextColor)
            .scaleEffect(1.2)
            .frame(width: 30, height: theme.height)
            .contentShape(Rectangle())
    }
}

/// Tool shortcut button 按壓回饋（縮放 + 透明度）
private struct ToolShortcutButtonStyle: SwiftUI.ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.5 : 1.0)
            .scaleEffect(configuration.isPressed ? 0.85 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}
