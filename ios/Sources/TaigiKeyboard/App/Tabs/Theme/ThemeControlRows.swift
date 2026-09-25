// Shared appearance-control-row components (color row, slider row, photo row) for the
// custom theme editor draft (ThemeEditorView).

import PhotosUI
import SwiftUI

/// Shared slider ranges/steps for the user-theme editor draft.
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

/// A labeled `ColorPicker` row with a trailing reset button shown while `onReset`
/// is non-nil (the caller passes nil when the value already equals its default).
/// Used by the user-theme editor (`ThemeEditorView`, where the binding mutates an
/// in-memory draft); `onReset` is the single writer of the reset value.
struct ThemeColorRow: View {
    let label: String
    @Binding var color: Color
    let onReset: (() -> Void)?

    var body: some View {
        HStack {
            ColorPicker(label, selection: $color, supportsOpacity: false)

            if let onReset {
                Button(action: onReset) {
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

// MARK: - Photo row

/// A `PhotosPicker` row: a thumbnail of the current theme photo (or a placeholder while
/// none is picked) beside the Choose Photo / Change Photo label. Tapping anywhere on the row opens
/// the system picker (images only; no photo-library permission is required).
struct ThemePhotoRow: View {
    let label: String
    /// The current theme photo's file name, or nil.
    let file: String?
    @Binding var selection: PhotosPickerItem?

    private static let thumbnailSize: CGFloat = 44

    var body: some View {
        PhotosPicker(selection: $selection, matching: .images) {
            HStack(spacing: 12) {
                Group {
                    // The thumbnail decode (off the main actor), not the 1280 px photo.
                    if let file {
                        ThemePhotoImage(file: file, variant: .thumbnail) { image in
                            Image(uiImage: image).resizable().scaledToFill()
                        } placeholder: {
                            Color(.tertiarySystemFill)
                        }
                    } else {
                        Color(.tertiarySystemFill)
                    }
                }
                .frame(width: Self.thumbnailSize, height: Self.thumbnailSize)
                .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
                Text(label)
                    .foregroundColor(.primary)
                Spacer()
                Image(latinSystemName: "photo.on.rectangle")
                    .foregroundColor(.secondary)
            }
        }
    }
}

// MARK: - Font Picker

/// Font-selection subpage listing `FontType.allCases`. Opened from the theme
/// page's font entry to pick the GLOBAL keyboard font (font is not part of a theme).
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
