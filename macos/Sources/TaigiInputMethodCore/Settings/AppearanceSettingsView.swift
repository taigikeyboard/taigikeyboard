// The 外觀 pane: how the candidate window looks — mode, layout, size, font.

import SwiftUI

/// The 外觀 pane of the settings window, shaped like System Settings'
/// Appearance pane: an 外觀 row of light/dark/auto thumbnails, then the
/// candidate window's own pickers — layout, the two size steps, and the
/// typeface.
///
/// Two rows are deliberately absent, each argued where its own type lives: no
/// accent-colour swatch (see `CandidateAccentColor`) and no chrome-generation
/// picker (see `CandidateWindowStyle`). Both follow the system instead.
///
/// `@AppStorage`-bound like `GeneralSettingsView`, and for the same reason:
/// the values are read live by the candidate-window router on every show, so
/// a change here applies from the next keystroke with nothing told about it.
struct AppearanceSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.appearanceMode.name)
    private var appearanceMode = SettingsStore.Keys.appearanceMode.defaultValue

    @AppStorage(SettingsStore.Keys.candidateLayout.name)
    private var candidateLayout = SettingsStore.Keys.candidateLayout.defaultValue

    @AppStorage(SettingsStore.Keys.candidateWindowSize.name)
    private var candidateWindowSize = SettingsStore.Keys.candidateWindowSize.defaultValue

    @AppStorage(SettingsStore.Keys.candidateTextSize.name)
    private var candidateTextSize = SettingsStore.Keys.candidateTextSize.defaultValue

    @AppStorage(SettingsStore.Keys.fontType.name)
    private var fontType = SettingsStore.Keys.fontType.defaultValue

    var body: some View {
        Form {
            // The System Settings shape: the mode selector leads its own
            // group, label leading like every other row.
            Section {
                LabeledContent(language.string(.macosAppearanceTab)) {
                    AppearanceModeRow(selection: $appearanceMode)
                }
            }

            Section {
                Picker(language.string(.macosCandidateWindowLayout), selection: $candidateLayout) {
                    Text(language.string(.macosCandidateLayoutExpandable)).tag(CandidateLayout.expandable)
                    Text(language.string(.macosCandidateLayoutHorizontal)).tag(CandidateLayout.horizontal)
                    Text(language.string(.macosCandidateLayoutVertical)).tag(CandidateLayout.vertical)
                }
                // The two size rows are named steps, not continuous values, so
                // they are pop-up menus like the rows above rather than
                // sliders (Apple HIG, Pop-up Buttons: a flat list of mutually
                // exclusive choices).
                Picker(language.string(.macosCandidateWindowSize), selection: $candidateWindowSize) {
                    Text(language.string(.macosSizeSmall)).tag(CandidateWindowSizeChoice.small)
                    Text(language.string(.macosSizeMedium)).tag(CandidateWindowSizeChoice.medium)
                    Text(language.string(.macosSizeLarge)).tag(CandidateWindowSizeChoice.large)
                }
                Picker(language.string(.themeCandidateTextSize), selection: $candidateTextSize) {
                    Text(language.string(.macosSizeSmall)).tag(CandidateTextSizeChoice.small)
                    Text(language.string(.macosSizeMedium)).tag(CandidateTextSizeChoice.medium)
                    Text(language.string(.macosSizeLarge)).tag(CandidateTextSizeChoice.large)
                }
                // The roster comes from the type rather than being spelled out
                // row by row like the pickers above: those name three fixed
                // steps each, while the fonts are a list the bundle can grow.
                Picker(language.string(.themeCustomFont), selection: $fontType) {
                    ForEach(CandidateFontChoice.allCases, id: \.self) { font in
                        Text(language.string(font.labelKey)).tag(font)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }
}

/// The 淺色 / 深色 / 自動 selector, drawn the way System Settings draws its
/// Appearance row: a thumbnail per mode with a caption under it, the selected
/// one ringed in the accent colour. The thumbnails are miniature candidate
/// windows rather than Apple's desktop artwork — they depict the thing this
/// setting changes.
private struct AppearanceModeRow: View {
    @Binding var selection: AppearanceMode

    @Environment(DisplayLanguageStore.self) private var language

    /// System Settings' order: light, dark, then auto.
    private static let modes: [AppearanceMode] = [.light, .dark, .auto]

    /// The highlight capsule's colour, fixed for the same reason the
    /// thumbnails' backgrounds are: each depicts ONE mode, so nothing in it may
    /// resolve against whatever appearance the form happens to render in. It
    /// stands for a highlight rather than previewing the resolved accent — the
    /// real one follows the system and the frontmost app (`CandidateAccentColor`).
    private static let thumbnailHighlightColor = Color(
        .sRGB, red: 0x00 / 255, green: 0x7A / 255, blue: 0xFF / 255,
    )

    private static let thumbnailSize = CGSize(width: 62, height: 40)
    private static let thumbnailCornerRadius: CGFloat = 8
    private static let thumbnailSpacing: CGFloat = 14
    private static let selectionRingPadding: CGFloat = 2

    var body: some View {
        HStack(alignment: .top, spacing: Self.thumbnailSpacing) {
            ForEach(Self.modes, id: \.self) { mode in
                thumbnailButton(for: mode)
            }
        }
        .padding(.vertical, 4)
    }

    private func thumbnailButton(for mode: AppearanceMode) -> some View {
        let name = language.string(mode.labelKey)
        return Button {
            selection = mode
        } label: {
            VStack(spacing: 5) {
                thumbnail(for: mode)
                    .frame(width: Self.thumbnailSize.width, height: Self.thumbnailSize.height)
                    .clipShape(RoundedRectangle(cornerRadius: Self.thumbnailCornerRadius))
                    .overlay(
                        // A hairline so the light thumbnail keeps an edge on a
                        // light form background.
                        RoundedRectangle(cornerRadius: Self.thumbnailCornerRadius)
                            .strokeBorder(.separator, lineWidth: 1),
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: Self.thumbnailCornerRadius + Self.selectionRingPadding)
                            .strokeBorder(Color.accentColor, lineWidth: 2)
                            .padding(-Self.selectionRingPadding)
                            .opacity(selection == mode ? 1 : 0),
                    )
                Text(name)
                    .font(.subheadline)
                    .foregroundStyle(selection == mode ? .primary : .secondary)
            }
        }
        .buttonStyle(.plain)
        .help(name)
        .accessibilityLabel(name)
        .accessibilityAddTraits(selection == mode ? .isSelected : [])
    }

    /// 自動 is the two fixed thumbnails split down the middle — light on the
    /// left, dark on the right — which is how System Settings depicts it.
    @ViewBuilder
    private func thumbnail(for mode: AppearanceMode) -> some View {
        switch mode {
        case .light:
            miniCandidateWindow(dark: false)
        case .dark:
            miniCandidateWindow(dark: true)
        case .auto:
            ZStack {
                miniCandidateWindow(dark: false)
                miniCandidateWindow(dark: true)
                    .mask(alignment: .trailing) {
                        Rectangle().frame(width: Self.thumbnailSize.width / 2)
                    }
            }
        }
    }

    /// A miniature of what the setting controls: a candidate bar — three
    /// cells, the first highlighted — on the mode's background. Fixed colours
    /// on purpose: each thumbnail depicts ONE mode, so it must not follow the
    /// appearance the form happens to render in.
    private func miniCandidateWindow(dark: Bool) -> some View {
        ZStack {
            (dark ? Color(white: 0.16) : Color(white: 0.94))
            HStack(spacing: 2.5) {
                Capsule()
                    .fill(Self.thumbnailHighlightColor)
                    .frame(width: 12, height: 7)
                Capsule()
                    .fill(dark ? Color(white: 0.38) : Color(white: 0.74))
                    .frame(width: 9, height: 7)
                Capsule()
                    .fill(dark ? Color(white: 0.38) : Color(white: 0.74))
                    .frame(width: 9, height: 7)
            }
        }
    }
}
