// 中文: 外觀控制列共用元件 — 顏色 row 與 slider row。外觀編輯器(自訂主題,draft)
// 中文: 與「自訂外觀設定」(預設 buffer,live-write)兩處共用,避免兩份列版面。

import SwiftUI

/// Shared slider ranges/steps for both appearance editors (default buffer +
/// user-theme draft), so the two never drift.
// 中文: 兩個外觀編輯器共用的 slider 範圍/步進,避免兩處各寫一份漂移。
enum ThemeSliderRanges {
    static let scale: ClosedRange<Double> = 0.85 ... 1.15
    static let scaleStep: Double = 0.01
    static let radius: ClosedRange<Double> = 0 ... 15
    static let radiusStep: Double = 0.5
    static let borderWidth: ClosedRange<Double> = 0 ... 3
    static let borderWidthStep: Double = 0.5
    static let shadow: ClosedRange<Double> = 0 ... 4
    static let shadowStep: Double = 0.5
}

/// A labeled `ColorPicker` row with a trailing reset button.
///
/// Shared by the default-buffer editor (`AppearanceSettingsView`, where the
/// closures write `SharedSettings` live) and the user-theme editor
/// (`ThemeEditorView`, where the binding mutates an in-memory draft). The row is
/// agnostic to persistence: it surfaces the picked color, fires `onChange`, and
/// shows the reset affordance when `isCustomized`.
// 中文: 具標籤的顏色 row + reset 鈕。不管持久化:顯示色、丟出 onChange、isCustomized 時顯示 reset。
struct ThemeColorRow: View {
    let label: String
    @Binding var color: Color
    let defaultColor: Color
    let isCustomized: Bool
    let onChange: (Color) -> Void
    let onReset: () -> Void

    var body: some View {
        HStack {
            ColorPicker(label, selection: $color, supportsOpacity: false)
                .onChange(of: color) { _, newValue in
                    onChange(newValue)
                }

            if isCustomized {
                Button {
                    color = defaultColor
                    onReset()
                } label: {
                    Image(latinSystemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

/// A labeled `Slider` row with a trailing reset button shown when the value
/// differs from `defaultValue`. Shared by both appearance editors.
// 中文: 具標籤的 slider row + 偏離預設時顯示 reset 鈕。兩個外觀編輯器共用。
struct ThemeSliderRow: View {
    let label: String
    @Binding var value: Double
    let range: ClosedRange<Double>
    let step: Double
    let defaultValue: Double
    let onChanged: (Double) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                Spacer()
                if value != defaultValue {
                    Button {
                        value = defaultValue
                        onChanged(defaultValue)
                    } label: {
                        Image(latinSystemName: "arrow.counterclockwise")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            Slider(value: $value, in: range, step: step)
                .onChange(of: value) { _, newValue in
                    onChanged(newValue)
                }
        }
    }
}

// MARK: - Font Picker

/// Font-selection subpage listing `FontType.allCases`. Shared by both appearance
/// editors (default buffer + user-theme draft).
// 中文: 字型挑選子頁。兩個外觀編輯器共用(預設 buffer + 自訂主題草稿)。
struct ThemeFontPickerView: View {
    @Binding var selectedFont: FontType
    var onChange: (FontType) -> Void

    var body: some View {
        Form {
            Section {
                ForEach(FontType.allCases, id: \.self) { font in
                    Button {
                        selectedFont = font
                        onChange(font)
                    } label: {
                        HStack {
                            Text(font.displayName)
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedFont == font {
                                Image(latinSystemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(ThemeTexts.customFont)
        .navigationBarTitleDisplayMode(.inline)
    }
}
