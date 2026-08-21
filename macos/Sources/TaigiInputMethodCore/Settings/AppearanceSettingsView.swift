// The 外觀 pane: how the candidate window looks — mode, accent, layout, chrome.

import SwiftUI

/// The 外觀 pane of the settings window, shaped like System Settings'
/// Appearance pane: an 外觀 row of light/dark/auto thumbnails, a 強調色 row
/// of colour circles (the same eight MacishType's site demos,
/// https://luke-chang.github.io/MacishType/), then the candidate window's own
/// two pickers — layout and chrome generation.
///
/// `@AppStorage`-bound like `GeneralSettingsView`, and for the same reason:
/// the values are read live by the candidate-window router on every show, so
/// a change here applies from the next keystroke with nothing told about it.
struct AppearanceSettingsView: View {
    /// Same bound as the 一般 form — see `GeneralSettingsView.maximumFormWidth`.
    private static let maximumFormWidth: CGFloat = 640

    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.candidateAppearanceMode.name)
    private var candidateAppearanceMode = SettingsStore.Keys.candidateAppearanceMode.defaultValue

    @AppStorage(SettingsStore.Keys.candidateAccentColor.name)
    private var candidateAccentColor = SettingsStore.Keys.candidateAccentColor.defaultValue

    @AppStorage(SettingsStore.Keys.candidateLayout.name)
    private var candidateLayout = SettingsStore.Keys.candidateLayout.defaultValue

    @AppStorage(SettingsStore.Keys.candidateWindowStyle.name)
    private var candidateWindowStyle = SettingsStore.Keys.candidateWindowStyle.defaultValue

    @AppStorage(SettingsStore.Keys.candidateWindowSize.name)
    private var candidateWindowSize = SettingsStore.Keys.candidateWindowSize.defaultValue

    @AppStorage(SettingsStore.Keys.candidateTextSize.name)
    private var candidateTextSize = SettingsStore.Keys.candidateTextSize.defaultValue

    var body: some View {
        Form {
            // The System Settings shape: the mode selector and the accent row
            // share the first group, labels leading like every other row.
            Section {
                LabeledContent(language.string(.macosAppearanceTab)) {
                    AppearanceModeRow(selection: $candidateAppearanceMode)
                }
                LabeledContent(language.string(.macosCandidateAccentColor)) {
                    AccentSwatchRow(selection: $candidateAccentColor)
                }
            }

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
                    Text(language.string(.macosSizeExtraLarge)).tag(CandidateTextSizeChoice.extraLarge)
                }
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: Self.maximumFormWidth)
    }
}

/// The 淺色 / 深色 / 自動 selector, drawn the way System Settings draws its
/// Appearance row: a thumbnail per mode with a caption under it, the selected
/// one ringed in the accent colour. The thumbnails are miniature candidate
/// windows rather than Apple's desktop artwork — they depict the thing this
/// setting changes.
private struct AppearanceModeRow: View {
    @Binding var selection: CandidateAppearanceMode

    @Environment(DisplayLanguageStore.self) private var language

    /// System Settings' order: light, dark, then auto.
    private static let modes: [CandidateAppearanceMode] = [.light, .dark, .auto]

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

    private func thumbnailButton(for mode: CandidateAppearanceMode) -> some View {
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
    private func thumbnail(for mode: CandidateAppearanceMode) -> some View {
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
                    .fill(Color(nsColor: CandidateAccentChoice.blue.overrideColor ?? .controlAccentColor))
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

/// The accent choices as a row of colour circles: 自動 first — drawn as a
/// conic sweep of all eight, the way System Settings draws Multicolour — then
/// the eight accents, each a plain disc of its `overrideColor`.
private struct AccentSwatchRow: View {
    @Binding var selection: CandidateAccentChoice

    @Environment(DisplayLanguageStore.self) private var language

    /// Larger than System Settings' accent row (USER 2026-08-18 「可以大一點」),
    /// between it and the 28pt discs MacishType's site demos; the ring floats
    /// just outside the disc.
    private static let swatchDiameter: CGFloat = 22
    private static let swatchSpacing: CGFloat = 10
    private static let selectionRingPadding: CGFloat = 2.5

    var body: some View {
        HStack(spacing: Self.swatchSpacing) {
            ForEach(CandidateAccentChoice.allCases, id: \.self) { choice in
                swatch(for: choice)
            }
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
