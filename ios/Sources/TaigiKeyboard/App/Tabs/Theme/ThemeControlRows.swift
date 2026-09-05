// 外觀控制列共用元件 — 顏色 row 與 slider row,供自訂主題編輯器(ThemeEditorView,draft)使用。

import SwiftUI

/// Shared slider ranges/steps for the user-theme editor draft.
// 自訂主題編輯器(draft)的 slider 範圍/步進。
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
/// Used by the user-theme editor (`ThemeEditorView`, where the binding mutates
/// an in-memory draft). The row is agnostic to persistence: it surfaces the
/// picked color, fires `onChange`, and shows the reset affordance when
/// `isCustomized`.
// 具標籤的顏色 row + reset 鈕。不管持久化:顯示色、丟出 onChange、isCustomized 時顯示 reset。
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
/// differs from `defaultValue`. Used by the user-theme editor.
// 具標籤的 slider row + 偏離預設時顯示 reset 鈕。兩個外觀編輯器共用。
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

/// Font-selection subpage listing `FontType.allCases`. Opened from the theme
/// page's font entry to pick the GLOBAL keyboard font (font is not part of a theme).
// 字型挑選子頁。從主題頁的字型入口進入,挑選全域鍵盤字型(字型非主題的一部分)。
struct ThemeFontPickerView: View {
    @Environment(DisplayLanguageStore.self) private var lang
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
                            Text(lang.string(font.displayNameKey))
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
        .navigationTitle(lang.string(.themeCustomFont))
        .navigationBarTitleDisplayMode(.inline)
    }
}
