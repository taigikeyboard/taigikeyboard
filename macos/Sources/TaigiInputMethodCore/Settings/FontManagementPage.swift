// 字型管理: which typeface the candidate window is set in, and the typefaces the user added.

import SwiftUI
import UniformTypeIdentifiers

/// One row of the typeface list: a bundled face or one the user added.
///
/// The two kinds are rows of ONE list on purpose. The list's selection is the
/// typeface the candidate window draws in, whichever kind it is — so there is
/// one selection with one meaning, which is what a picker in 外觀 beside a
/// table here could not have had (the picker's selection and the table's would
/// have been two highlights meaning different things).
private struct FontRow: Identifiable {
    let selection: CandidateFontSelection
    let title: String
    /// Only a typeface the user added can leave the list; `−` is disabled on
    /// the bundled five.
    let customFont: CustomFont?

    var id: String {
        switch selection {
        case let .builtIn(choice): "builtIn.\(choice.rawValue)"
        case let .custom(font): "custom.\(font.fileName)"
        }
    }
}

/// The typeface pane: the whole roster in one table, with `+` / `−` under it.
///
/// A table whose selection is the live typeface, rather than a pop-up menu:
/// this list grows and shrinks with what the user installs, and System Settings
/// states such a list the same way — 聲音's output devices, 顯示器's displays,
/// where selecting a row is what makes it the one in use.
///
/// The list's SHAPE is 自訂詞庫's (`CustomDictionaryPage.entryTable`): an inset
/// table with no striping, and the `+` / `−` pair under it that macOS gives an
/// editable list.
struct FontManagementPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.fontType.name)
    private var fontTypeRawValue = SettingsStore.Keys.fontType.defaultValue.rawValue

    @AppStorage(SettingsStore.Keys.customFontFile.name)
    private var customFontFile = SettingsStore.Keys.customFontFile.defaultValue

    @State private var customFonts: [CustomFont] = []
    @State private var message: UserDataPageMessage?

    /// Tall enough for the bundled five plus a couple the user added, without
    /// leaving a stretch of empty rows under a fresh install's list.
    private static let visibleRowCount = 7

    var body: some View {
        Form {
            Section {
                if isSelectedCustomFontUnavailable {
                    // The file named by the preference is gone or will not
                    // activate. Said rather than silently corrected: the
                    // preference is kept (`SettingsStore.candidateFontSelection`),
                    // and an external volume or a restore may bring it back.
                    Text(language.string(.desktopCustomFontMissing))
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                fontTable
                UserDataListControls(
                    addLabelKey: .desktopCustomFontAdd,
                    isRemoveEnabled: selectedRow?.customFont != nil,
                    onAdd: add,
                    onRemove: {
                        if let font = selectedRow?.customFont {
                            remove(font)
                        }
                    },
                )
            }

            // Its own section at the end, the way 外觀 draws its own: it acts
            // on every row above it rather than on any one of them.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll) {
                    SettingsStore().resetFontSettings()
                }
            }
        }
        .formStyle(.grouped)
        .onAppear(perform: reload)
        .userDataPageChrome(activity: .idle, message: $message)
    }

    private var fontTable: some View {
        Table(rows, selection: selectionBinding) {
            TableColumn(language.string(.themeCustomFont)) { row in
                Text(row.title)
            }
        }
        .tableStyle(.inset)
        // Every row the same colour, as on the 自訂詞庫 table: settings
        // content, not a spreadsheet.
        .alternatingRowBackgrounds(.disabled)
        .frame(height: UserDataListMetrics.tableHeight(rows: Self.visibleRowCount))
        .contextMenu(forSelectionType: FontRow.ID.self) { ids in
            if let font = rows.first(where: { $0.id == ids.first })?.customFont {
                Button(language.string(.commonDelete), role: .destructive) { remove(font) }
            }
        }
    }

    /// The bundled roster first, in the order the four platforms share, then
    /// what the user added — a list that reads as "what ships" and "what I
    /// added", rather than one sorted order hiding the difference.
    private var rows: [FontRow] {
        CandidateFontChoice.allCases.map {
            FontRow(selection: .builtIn($0), title: language.string($0.labelKey), customFont: nil)
        }
            + customFonts.map {
                // The font's own name, which is text out of a file the user
                // chose — shown, never trusted.
                FontRow(selection: .custom($0), title: $0.displayName, customFont: $0)
            }
    }

    /// The table's selection IS the setting, resolved across the two keys that
    /// store it and written back to both — so a row can never leave `fontType`
    /// naming a custom typeface with no file name beside it.
    ///
    /// A `Table`'s selection is optional (it can be cleared); this pane's is
    /// not, because a typeface is always in use. A cleared selection is
    /// therefore ignored rather than written.
    private var selectionBinding: Binding<FontRow.ID?> {
        Binding(
            get: { selectedRow?.id },
            set: { newValue in
                guard let row = rows.first(where: { $0.id == newValue }) else { return }
                switch row.selection {
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
            },
        )
    }

    /// The row the two stored keys name — the bundled face, or the custom one
    /// while its file is in the library.
    private var selectedRow: FontRow? {
        guard fontTypeRawValue == CandidateFontSelection.customRawValue else {
            let choice = CandidateFontChoice(rawValue: fontTypeRawValue) ?? .system
            return rows.first { $0.selection == .builtIn(choice) }
        }
        return rows.first { $0.customFont?.fileName == customFontFile }
    }

    /// A custom typeface is selected and the library cannot produce it.
    private var isSelectedCustomFontUnavailable: Bool {
        fontTypeRawValue == CandidateFontSelection.customRawValue && selectedRow == nil
    }

    /// Asks for a font file, takes it in, and selects it.
    ///
    /// Selecting it is the point of adding it: a user who just chose a typeface
    /// wants to see it, and the alternative — finding the row they just created
    /// and clicking it — is a second step for nothing.
    private func add() {
        Task {
            await UserDataFilePanels.withSettingsWindow { window in
                guard let url = await UserDataFilePanels.chooseFileToOpen(
                    contentTypes: [.font],
                    in: window,
                ) else { return }
                do {
                    let font = try CustomFontLibrary.shared.addFont(from: url)
                    reload()
                    selectionBinding.wrappedValue = FontRow(
                        selection: .custom(font), title: font.displayName, customFont: font,
                    ).id
                } catch {
                    message = .failure(.commonImportFailed, error)
                    reload()
                }
            }
        }
    }

    /// Takes `font` out of the library.
    ///
    /// In this order, and the order is the whole of it: the selection moves off
    /// the typeface first, then every panel built in it is dropped, and only
    /// then is the font unregistered and its file deleted. A panel built in a
    /// face is a use of it, and Core Text refuses to unregister a font that is
    /// in use — which would otherwise leave this process drawing from a deleted
    /// file. Only the panels drawn in THIS face: dropping the others would
    /// rebuild a window's worth of cells to delete a typeface they were never
    /// set in.
    private func remove(_ font: CustomFont) {
        if fontTypeRawValue == CandidateFontSelection.customRawValue, customFontFile == font.fileName {
            fontTypeRawValue = CandidateFontChoice.system.rawValue
            customFontFile = ""
        }
        CandidatePanel.shared.releaseCachedPanels(drawing: font)
        do {
            try CustomFontLibrary.shared.remove(font)
        } catch {
            message = .failure(.desktopCustomFontRemoveFailed, error)
        }
        reload()
    }

    /// Re-reads the library.
    ///
    /// Forced rather than asked: the scan is invalidated by whoever changed the
    /// directory, but the user may have been in Finder since this pane was last
    /// on screen.
    private func reload() {
        CustomFontLibrary.shared.invalidateCache()
        customFonts = CustomFontLibrary.shared.installedFonts()
    }
}
