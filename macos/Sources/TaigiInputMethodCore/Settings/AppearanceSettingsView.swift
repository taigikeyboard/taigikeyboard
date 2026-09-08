// The 外觀 pane: how the candidate window looks — mode, layout, size.

import SwiftUI

/// The 外觀 pane of the settings window: an 外觀 pop-up of light/dark/auto,
/// then the candidate window's own pickers — layout, what each cell shows and
/// the two size steps. Every row is the same pop-up menu in one group, so the
/// pane reads as one list rather than a drawn selector fenced off above a
/// stack of menus (USER 2026-09-02).
///
/// Three rows are deliberately absent, each argued where it lives: no
/// accent-colour swatch (see `CandidateAccentColor`), no chrome-generation
/// picker (see `CandidateWindowStyle`) — both follow the system — and no
/// typeface. The typeface moved to 字型管理 (`FontManagementPage`, USER
/// 2026-09-08): the roster grows with what the user installs, so choosing one
/// and managing the list is one table there rather than a pop-up here beside
/// a list of files.
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

    @AppStorage(SettingsStore.Keys.candidateDisplayMode.name)
    private var candidateDisplayMode = SettingsStore.Keys.candidateDisplayMode.defaultValue

    var body: some View {
        Form {
            // The mode selector leads the pane's one group, drawn like every
            // row under it: three named, mutually exclusive choices are a
            // pop-up menu (Apple HIG, Pop-up Buttons).
            Section {
                Picker(language.string(.desktopAppearanceTab), selection: $appearanceMode) {
                    Text(language.string(AppearanceMode.light.labelKey)).tag(AppearanceMode.light)
                    Text(language.string(AppearanceMode.dark.labelKey)).tag(AppearanceMode.dark)
                    Text(language.string(AppearanceMode.auto.labelKey)).tag(AppearanceMode.auto)
                }
                Picker(language.string(.desktopCandidateWindowLayout), selection: $candidateLayout) {
                    Text(language.string(.desktopCandidateLayoutExpandable)).tag(CandidateLayout.expandable)
                    Text(language.string(.desktopCandidateLayoutHorizontal)).tag(CandidateLayout.horizontal)
                    Text(language.string(.desktopCandidateLayoutVertical)).tag(CandidateLayout.vertical)
                }
                // What each cell shows, beside how the cells are arranged: both
                // scripts side by side (today's rendering), each script as its
                // own adjacent cell, or the romanization alone. Bound like the rows around
                // it — the open bar re-renders on the write
                // (`TaigiInputController`), the next one reads it live.
                Picker(language.string(.settingsCandidateDisplayMode), selection: $candidateDisplayMode) {
                    Text(language.string(.settingsCandidateDisplayModeSideBySide))
                        .tag(CandidateDisplayMode.sideBySide)
                    Text(language.string(.settingsCandidateDisplayModeCombined))
                        .tag(CandidateDisplayMode.combined)
                    Text(language.string(.settingsCandidateDisplayModeRomanOnly))
                        .tag(CandidateDisplayMode.romanOnly)
                }
                // The two size rows are named steps, not continuous values, so
                // they are pop-up menus like the rows above rather than
                // sliders (Apple HIG, Pop-up Buttons: a flat list of mutually
                // exclusive choices).
                Picker(language.string(.desktopCandidateWindowSize), selection: $candidateWindowSize) {
                    Text(language.string(.desktopSizeSmall)).tag(CandidateWindowSizeChoice.small)
                    Text(language.string(.desktopSizeMedium)).tag(CandidateWindowSizeChoice.medium)
                    Text(language.string(.desktopSizeLarge)).tag(CandidateWindowSizeChoice.large)
                }
                Picker(language.string(.themeCandidateTextSize), selection: $candidateTextSize) {
                    Text(language.string(.desktopSizeSmall)).tag(CandidateTextSizeChoice.small)
                    Text(language.string(.desktopSizeMedium)).tag(CandidateTextSizeChoice.medium)
                    Text(language.string(.desktopSizeLarge)).tag(CandidateTextSizeChoice.large)
                }
            }

            // Its own section, at the end, drawn the way the 快捷鍵 pane draws
            // its own: it acts on every row above it rather than on any one of
            // them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
    }

    /// Puts the whole pane back to what a fresh install renders with.
    ///
    /// The `@AppStorage` bindings above repaint on their own: removing a key
    /// is a `UserDefaults` change like any other, and each binding falls back
    /// to the default it was declared with.
    private func restoreDefaults() {
        SettingsStore().resetAppearanceSettings()
    }
}
