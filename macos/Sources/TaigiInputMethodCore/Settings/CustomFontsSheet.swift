// 自訂字型: the typefaces the user added themselves, added and removed.

import SwiftUI
import UniformTypeIdentifiers

/// The library, as the list macOS states a user-managed set of things with: an
/// inset table, and the `+` / `−` pair under it — the shape 自訂詞庫 already
/// uses (`CustomDictionaryPage.entryTable`), and the one System Settings gives
/// its own managed lists.
///
/// A sheet rather than a sidebar pane or a section in 外觀. Two reasons, and
/// they are the same reason twice:
///
/// - A pane would be a sixth sidebar row for something a user opens once and
///   leaves alone, against the direction that pane roster was trimmed in.
/// - A table in 外觀 itself would put TWO selections on one pane — the picker's
///   (which typeface draws) and the table's (which row `−` acts on) — showing
///   two highlights that mean different things. In a sheet only one of them is
///   on screen at a time.
///
/// The sheet does not choose the typeface: the picker in 外觀 does, and this
/// hands its changes back through `onAdded` / `onRemoving` because the selection
/// lives in two defaults keys the pane owns.
struct CustomFontsSheet: View {
    @Environment(DisplayLanguageStore.self) private var language
    @Environment(\.dismiss) private var dismiss

    /// Called with a typeface that just joined the library — the pane selects
    /// it, because a user who just added a typeface wants to see it.
    let onAdded: (CustomFont) -> Void
    /// Called BEFORE a typeface is unregistered, so the pane can move the
    /// selection off it and release the panels drawn in it.
    let onRemoving: (CustomFont) -> Void
    /// Called after the library changed, so the pane's picker does not go on
    /// offering a typeface that is no longer there.
    let onChanged: () -> Void

    @State private var fonts: [CustomFont] = []
    @State private var selectedFontID: CustomFont.ID?
    @State private var message: UserDataPageMessage?
    /// This sheet's own window, which the file panel hangs off — see
    /// `HostWindowReader` for why it cannot be the settings window.
    @State private var sheetWindow: NSWindow?

    /// Four rows: a library is a handful of typefaces, not a dictionary's
    /// thousands, and a table taller than its contents reads as a list that
    /// failed to load.
    private static let visibleRowCount = 4

    /// An empty tray, the symbol 自訂詞庫 draws for the same state.
    private static let emptyStateSymbolName = "tray"

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(language.string(.desktopCustomFontSection))
                .font(.headline)
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
            HStack {
                Spacer()
                Button(language.string(.commonOk)) { dismiss() }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 420)
        .background(HostWindowReader(window: $sheetWindow))
        .onAppear(perform: reload)
        // The sheet's own alert: a failure raised while this sheet is up has to
        // be readable over the sheet that caused it.
        .alert(item: $message) { message in
            Alert(
                title: Text(message.title(language.resolver)),
                message: message.detail(language.resolver).map(Text.init),
                dismissButton: .default(Text(language.string(.commonOk))),
            )
        }
    }

    /// The typefaces, by the name each file declares — which is text out of a
    /// file the user chose, shown and never trusted.
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

    private func add() {
        Task {
            await UserDataFilePanels.withSettingsWindow { window in
                guard let url = await UserDataFilePanels.chooseFileToOpen(
                    contentTypes: [.font],
                    in: window,
                ) else { return }
                do {
                    try onAdded(CustomFontLibrary.shared.addFont(from: url))
                } catch {
                    message = .failure(.commonImportFailed, error)
                }
                reload()
            }
        }
    }

    /// Takes `font` out of the library.
    ///
    /// `onRemoving` first, and the order is the whole of it: the pane moves the
    /// selection off the typeface and releases the panels built in it, because
    /// Core Text refuses to unregister a font that is still in use — and
    /// deleting under a live registration would leave this process drawing from
    /// a file that is gone.
    private func remove(_ font: CustomFont) {
        onRemoving(font)
        do {
            try CustomFontLibrary.shared.remove(font)
        } catch {
            message = .failure(.desktopCustomFontRemoveFailed, error)
        }
        reload()
        onChanged()
    }

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
