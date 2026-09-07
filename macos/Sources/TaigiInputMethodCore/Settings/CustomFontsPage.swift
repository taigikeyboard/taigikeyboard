// 自訂字型: the typefaces the user added themselves.

import SwiftUI
import UniformTypeIdentifiers

/// The library, as the list macOS states a user-managed set of things with: an
/// inset table, and the `+` / `−` pair under it — the shape 自訂詞庫 already
/// uses (`CustomDictionaryPage.entryTable`).
///
/// Its own pane, beside 自訂詞庫, rather than a sheet or a table inside 外觀:
///
/// - A settings window is "a toolbar with buttons to switch between panes, each
///   containing related settings" (HIG, Settings > macOS), and a library the
///   user keeps adding to and removing from is content, not a task. A sheet is
///   for "a simple task that must be completed before returning to the parent
///   view" (HIG, Sheets), which this is not — and the Mac guidance is for
///   "fewer nested levels and less reliance on modality"
///   (HIG, Designing for macOS).
/// - A table inside 外觀 would put TWO selections on one pane: the typeface
///   picker's (which face draws) and the table's (which row `−` acts on), two
///   highlights meaning different things.
///
/// Choosing the typeface stays in 外觀, where the rest of the candidate
/// window's look is chosen. This pane writes that selection only to move it OFF
/// a typeface it is about to delete, and onto one it has just added.
struct CustomFontsPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.fontType.name)
    private var fontTypeRawValue = SettingsStore.Keys.fontType.defaultValue.rawValue

    @AppStorage(SettingsStore.Keys.customFontFile.name)
    private var customFontFile = SettingsStore.Keys.customFontFile.defaultValue

    @State private var fonts: [CustomFont] = []
    @State private var selectedFontID: CustomFont.ID?
    @State private var message: UserDataPageMessage?

    /// Six rows: a library is a handful of typefaces rather than a dictionary's
    /// thousands, and a table much taller than its contents reads as a list
    /// that failed to load.
    private static let visibleRowCount = 6

    /// An empty tray, the symbol 自訂詞庫 draws for the same state.
    private static let emptyStateSymbolName = "tray"

    var body: some View {
        Form {
            Section {
                fontTable
                UserDataListControls(
                    addLabelKey: .desktopCustomFontAdd,
                    isRemoveEnabled: selectedFont != nil,
                    onAdd: add,
                    onRemove: {
                        if let selectedFont {
                            remove(selectedFont)
                        }
                    },
                )
            } header: {
                HStack {
                    Text(language.string(.desktopCustomFontSection))
                    Spacer()
                    Text(verbatim: "\(fonts.count)")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .onAppear(perform: reload)
        .userDataPageChrome(activity: .idle, message: $message)
    }

    /// The typefaces, by the name each file declares — text out of a file the
    /// user chose, shown and never trusted.
    private var fontTable: some View {
        Table(fonts, selection: $selectedFontID) {
            TableColumn(language.string(.themeCustomFont)) { font in
                Text(font.displayName)
            }
        }
        .tableStyle(.inset)
        // Every row the same colour, as on the 自訂詞庫 table: a short list of
        // settings content, not a spreadsheet.
        .alternatingRowBackgrounds(.disabled)
        .frame(height: UserDataListMetrics.tableHeight(rows: Self.visibleRowCount))
        .overlay {
            if fonts.isEmpty {
                UserDataListEmptySymbol(
                    symbolName: Self.emptyStateSymbolName,
                    accessibilityLabelKey: .desktopCustomFontAdd,
                )
            }
        }
        .contextMenu(forSelectionType: CustomFont.ID.self) { ids in
            if let font = fonts.first(where: { $0.id == ids.first }) {
                Button(language.string(.commonDelete), role: .destructive) { remove(font) }
            }
        }
    }

    /// The row `−` acts on, or nil when the selection names a row the list no
    /// longer holds.
    private var selectedFont: CustomFont? {
        fonts.first { $0.id == selectedFontID }
    }

    /// Asks for a font file, takes it in, and selects it.
    ///
    /// Selecting it is the point of adding it: a user who just chose a typeface
    /// wants to see it, and the alternative — going to 外觀 to pick the row
    /// they just created — is a second step for nothing.
    private func add() {
        Task {
            await UserDataFilePanels.withSettingsWindow { window in
                guard let url = await UserDataFilePanels.chooseFileToOpen(
                    contentTypes: [.font],
                    in: window,
                ) else { return }
                do {
                    let font = try CustomFontLibrary.shared.addFont(from: url)
                    CustomFontLibrary.shared.activate(fileName: font.fileName)
                    fontTypeRawValue = CandidateFontSelection.customRawValue
                    customFontFile = font.fileName
                } catch {
                    message = .failure(.commonImportFailed, error)
                }
                reload()
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
        fonts = CustomFontLibrary.shared.installedFonts()
        // A selection the list no longer holds would leave `−` enabled over
        // nothing.
        if !fonts.contains(where: { $0.id == selectedFontID }) {
            selectedFontID = nil
        }
    }
}
