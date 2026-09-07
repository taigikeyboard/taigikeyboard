// The 外觀 pane: how the candidate window looks — mode, layout, size, font.

import SwiftUI

/// The 外觀 pane of the settings window: an 外觀 pop-up of light/dark/auto,
/// then the candidate window's own pickers — layout, what each cell shows,
/// the two size steps, and the typeface. Every row is the same pop-up menu in
/// one group, so the pane reads as one list rather than a drawn selector
/// fenced off above a stack of menus (USER 2026-09-02).
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

    /// The typeface key, read as its raw string rather than as a
    /// `CandidateFontChoice`: it also holds `CandidateFontSelection
    /// .customRawValue`, which is deliberately not a case of that roster.
    @AppStorage(SettingsStore.Keys.fontType.name)
    private var fontTypeRawValue = SettingsStore.Keys.fontType.defaultValue.rawValue

    @AppStorage(SettingsStore.Keys.customFontFile.name)
    private var customFontFile = SettingsStore.Keys.customFontFile.defaultValue

    /// The user's own typefaces, re-read when the pane appears and after every
    /// change this pane makes. Nothing else in the app writes the library, so
    /// there is nothing to watch for.
    @State private var customFonts: [CustomFont] = []
    /// Whether the selected custom typeface is the one actually drawing. A row
    /// can be in the roster and still not draw — its file may have stopped
    /// activating since the session began — and the two questions have
    /// different answers, so the pane asks the one it means.
    @State private var isSelectedCustomFontDrawing = false
    @State private var isManagingFonts = false

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
                // The roster comes from the type rather than being spelled out
                // row by row like the pickers above: those name three fixed
                // steps each, while the fonts are a list the bundle can grow.
                Picker(language.string(.themeCustomFont), selection: selection) {
                    ForEach(CandidateFontChoice.allCases, id: \.self) { font in
                        Text(language.string(font.labelKey)).tag(CandidateFontSelection.builtIn(font))
                    }
                    // The user's own typefaces continue the same list rather
                    // than sitting in a picker of their own: one row is one
                    // typeface the candidate window can draw in, whoever it
                    // came from. Named by the font's own name, which is text
                    // out of a file the user chose — shown, never trusted.
                    ForEach(customFonts) { font in
                        Text(font.displayName).tag(CandidateFontSelection.custom(font))
                    }
                }
            }

            // The library the picker spends, one row: adding and removing
            // typefaces is a list to manage, and a managed list is a table with
            // a `+` / `−` pair (`CustomFontsSheet`) rather than a run of form
            // rows. In a sheet, so this pane keeps ONE selection on screen —
            // the picker's — and stays the single column of pop-up menus it
            // reads as (USER 2026-09-02).
            Section {
                if isSelectedCustomFontUnavailable {
                    // The file named by the preference is gone or will not
                    // activate. Said here rather than silently corrected: the
                    // preference is kept (`SettingsStore.candidateFontSelection`),
                    // and an external volume or a restore may bring it back.
                    Text(language.string(.desktopCustomFontMissing))
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                WideActionRow(titleKey: .desktopCustomFontManage) { isManagingFonts = true }
            }

            // Its own section, at the end, drawn the way the 快捷鍵 pane draws
            // its own: it acts on every row above it rather than on any one of
            // them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
        .onAppear(perform: reload)
        .sheet(isPresented: $isManagingFonts, onDismiss: reload) {
            CustomFontsSheet(onAdded: select, onRemoving: releaseBeforeRemoving, onChanged: reload)
        }
    }

    /// The picker's selection, resolved across the two keys that store it and
    /// written back to both — so a row can never leave `fontType` naming a
    /// custom font with no file name beside it.
    private var selection: Binding<CandidateFontSelection> {
        Binding(
            get: {
                guard let font = selectedCustomFont
                else { return .builtIn(CandidateFontChoice(rawValue: fontTypeRawValue) ?? .system) }
                return .custom(font)
            },
            set: { newValue in
                switch newValue {
                case let .builtIn(choice):
                    fontTypeRawValue = choice.rawValue
                    customFontFile = ""
                case let .custom(font):
                    fontTypeRawValue = CandidateFontSelection.customRawValue
                    customFontFile = font.fileName
                    // Selecting is what activates: the candidate window reads
                    // the selection on every show and registers nothing itself
                    // (`SettingsStore.candidateFontSelection`).
                    CustomFontLibrary.shared.activate(fileName: font.fileName)
                }
                refreshSelectedFontState()
            },
        )
    }

    /// The row the selection names, or nil because a bundled face is selected —
    /// or because the file the preference names is not in the library. The one
    /// spelling of that lookup; the picker and the note below both read it.
    private var selectedCustomFont: CustomFont? {
        guard fontTypeRawValue == CandidateFontSelection.customRawValue else { return nil }
        return customFonts.first { $0.fileName == customFontFile }
    }

    /// A custom typeface is selected and the window is NOT drawing in it —
    /// either the file is gone, or it is there and would not activate.
    private var isSelectedCustomFontUnavailable: Bool {
        fontTypeRawValue == CandidateFontSelection.customRawValue && !isSelectedCustomFontDrawing
    }

    /// Selects a typeface the sheet just added.
    ///
    /// Selecting it is the point of adding it: a user who just chose a typeface
    /// wants to see it, and the alternative — a new row they have to find and
    /// pick — is a second step for nothing.
    private func select(_ font: CustomFont) {
        reload()
        selection.wrappedValue = .custom(font)
    }

    /// What the sheet asks for before it takes `font` out of the library.
    ///
    /// The selection moves off the typeface first, then every panel built in it
    /// is dropped: a panel in a face is a use of it, and Core Text refuses to
    /// unregister a font that is in use. Only the panels drawn in THIS face —
    /// dropping the others would rebuild a window's worth of cells and
    /// constraints to delete a typeface they were never set in.
    private func releaseBeforeRemoving(_ font: CustomFont) {
        if selectedCustomFont == font {
            selection.wrappedValue = .builtIn(.system)
        }
        CandidatePanel.shared.releaseCachedPanels(drawing: font)
    }

    private func reload() {
        CustomFontLibrary.shared.invalidateCache()
        customFonts = CustomFontLibrary.shared.installedFonts()
        refreshSelectedFontState()
    }

    /// Asks the library whether the selected typeface is the one drawing —
    /// which is what activating it answers, and what the roster cannot.
    private func refreshSelectedFontState() {
        let fileName = customFontFile
        isSelectedCustomFontDrawing = fontTypeRawValue == CandidateFontSelection.customRawValue
            && !fileName.isEmpty
            && CustomFontLibrary.shared.activate(fileName: fileName) != nil
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
