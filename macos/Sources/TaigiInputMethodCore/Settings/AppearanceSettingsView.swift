// The 外觀 pane: how the candidate window looks — layout, chrome, accent.

import SwiftUI

/// The 外觀 pane of the settings window: everything about how the candidate
/// window presents itself, in one place — its layout, its chrome generation,
/// and its accent colour, the last presented as the same row of colour circles
/// MacishType's site demos (https://luke-chang.github.io/MacishType/).
///
/// `@AppStorage`-bound like `GeneralSettingsView`, and for the same reason:
/// the values are read live by the candidate-window router on every show, so
/// a change here applies from the next keystroke with nothing told about it.
struct AppearanceSettingsView: View {
    /// Same bound as the 一般 form — see `GeneralSettingsView.maximumFormWidth`.
    private static let maximumFormWidth: CGFloat = 640

    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.candidateLayout.name)
    private var candidateLayout = SettingsStore.Keys.candidateLayout.defaultValue

    @AppStorage(SettingsStore.Keys.candidateWindowStyle.name)
    private var candidateWindowStyle = SettingsStore.Keys.candidateWindowStyle.defaultValue

    @AppStorage(SettingsStore.Keys.candidateAccentColor.name)
    private var candidateAccentColor = SettingsStore.Keys.candidateAccentColor.defaultValue

    var body: some View {
        Form {
            Section {
                Picker(language.string(.macosCandidateWindowLayout), selection: $candidateLayout) {
                    Text(language.string(.macosCandidateLayoutExpandable)).tag(CandidateLayout.expandable)
                    Text(language.string(.macosCandidateLayoutHorizontal)).tag(CandidateLayout.horizontal)
                    Text(language.string(.macosCandidateLayoutVertical)).tag(CandidateLayout.vertical)
                }
                Picker(language.string(.macosCandidateWindowAppearance), selection: $candidateWindowStyle) {
                    // The shared Automatic label — same word, same picker role
                    // as the display-language row's.
                    Text(language.string(.settingsDisplayLanguageAutomatic)).tag(CandidateWindowStyleChoice.auto)
                    Text(language.string(.macosCandidateStyleSequoia)).tag(CandidateWindowStyleChoice.sequoia)
                    Text(language.string(.macosCandidateStyleTahoe)).tag(CandidateWindowStyleChoice.tahoe)
                }
            }

            Section {
                AccentSwatchRow(selection: $candidateAccentColor)
            } header: {
                Text(language.string(.macosCandidateAccentColor))
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: Self.maximumFormWidth)
    }
}

/// The accent choices as a row of colour circles: 自動 first — drawn as a
/// conic sweep of all eight, the way System Settings draws Multicolour — then
/// the eight accents, each a plain disc of its `overrideColor`.
private struct AccentSwatchRow: View {
    @Binding var selection: CandidateAccentChoice

    @Environment(DisplayLanguageStore.self) private var language

    /// MacishType's site metrics: 28pt discs, 12pt apart, ring offset ~3pt.
    private static let swatchDiameter: CGFloat = 28
    private static let swatchSpacing: CGFloat = 12
    private static let selectionRingPadding: CGFloat = 3

    var body: some View {
        HStack(spacing: Self.swatchSpacing) {
            ForEach(CandidateAccentChoice.allCases, id: \.self) { choice in
                swatch(for: choice)
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
    }

    private func swatch(for choice: CandidateAccentChoice) -> some View {
        let name = language.string(choice.labelKey)
        return Button {
            selection = choice
        } label: {
            swatchFill(for: choice)
                .frame(width: Self.swatchDiameter, height: Self.swatchDiameter)
                .clipShape(Circle())
                .overlay(
                    // The ring floats a little outside the disc, so the colour
                    // stays a full circle rather than gaining a border.
                    Circle()
                        .strokeBorder(.secondary, lineWidth: 2)
                        .padding(-Self.selectionRingPadding)
                        .opacity(selection == choice ? 1 : 0),
                )
        }
        .buttonStyle(.plain)
        .help(name)
        .accessibilityLabel(name)
        .accessibilityAddTraits(selection == choice ? .isSelected : [])
    }

    @ViewBuilder
    private func swatchFill(for choice: CandidateAccentChoice) -> some View {
        if let color = choice.overrideColor {
            Color(nsColor: color)
        } else {
            // 自動: all eight accents in one disc. The stops repeat the first
            // colour at the end so the sweep closes without a seam.
            AngularGradient(
                colors: (CandidateAccentChoice.allCases.compactMap(\.overrideColor)
                    + [CandidateAccentChoice.blue.overrideColor!])
                    .map(Color.init(nsColor:)),
                center: .center,
            )
        }
    }
}
