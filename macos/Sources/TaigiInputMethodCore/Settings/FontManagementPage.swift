// 字型管理: which typeface the candidate window is set in — bundled, added by the user, or installed on this Mac.

import SwiftUI
import UniformTypeIdentifiers

/// One row of the typeface list: a bundled face, one the user added, or a
/// family the OS has installed.
///
/// The three kinds are rows of ONE list on purpose. The list's selection is
/// the typeface the candidate window draws in, whichever kind it is — so there
/// is one selection with one meaning, which is what a picker in 外觀 beside a
/// table here could not have had (the picker's selection and the table's would
/// have been two highlights meaning different things).
private struct FontRow: Identifiable {
    /// What selecting this row stores.
    let stored: StoredFontSelection
    let title: String
    /// Only a typeface the user added can leave the list; `−` is disabled on
    /// the bundled five and on the installed families, which are the OS's to
    /// remove.
    let customFont: CustomFont?

    var id: String {
        "\(stored.fontType).\(stored.customFontFile)\(stored.installedFontFamily)"
    }
}

/// The typeface pane: the whole roster in one table, a search field above it,
/// `+` / `−` under it.
///
/// A table whose selection is the live typeface, rather than a pop-up menu:
/// this list grows and shrinks with what the user installs, and System Settings
/// states such a list the same way — 聲音's output devices, 顯示器's displays,
/// where selecting a row is what makes it the one in use.
///
/// The list's SHAPE is 自訂詞庫's (`CustomDictionaryPage.entryTable`): a search
/// field, an inset table with no striping, one page of rows at a time, and the
/// `+` / `−` pair under it with the pager at its trailing end. The search field
/// and the pager are what make a few hundred installed families usable in one
/// table (USER 2026-09-11) — the alternative, a sub-page or a sheet for them,
/// is the popup this pane's shape was chosen to avoid.
///
/// No 回復預設 row (USER 2026-09-08): every other pane's reset restores rows the
/// user cannot otherwise put back one by one, while this list's default is a row
/// in it — 系統, first in the table, one click away.
struct FontManagementPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.fontType.name)
    private var fontTypeRawValue = SettingsStore.Keys.fontType.defaultValue.rawValue

    @AppStorage(SettingsStore.Keys.customFontFile.name)
    private var customFontFile = SettingsStore.Keys.customFontFile.defaultValue

    @AppStorage(SettingsStore.Keys.installedFontFamily.name)
    private var installedFontFamily = SettingsStore.Keys.installedFontFamily.defaultValue

    @State private var customFonts: [CustomFont] = []
    @State private var installedFamilies: [String] = []
    @State private var filter = ""
    /// Which page of the filtered rows is on screen, zero-based. Clamped when
    /// read: the rows under it change with the filter and with what the OS
    /// has, and a page past the end shows the last one rather than nothing.
    @State private var page = 0
    @State private var message: UserDataPageMessage?

    /// How many rows one page holds — the table's height, exactly, as 自訂詞庫
    /// does it (`CustomDictionaryPageModel.pageSize`): a page that fits the
    /// table never needs a scroller of its own, which a `Table` inside a
    /// `Form` cannot have (`UserDataListPager`). The installed families make
    /// this list a few hundred rows; the search field finds one, the pager
    /// reaches every one.
    private static let pageSize = 10

    var body: some View {
        // Resolved once per render: the list is a few hundred rows and the
        // search field re-renders on every keystroke, so the controls, the
        // table and the binding below all read these locals.
        let rows = rows
        let matchingRows = filter.isEmpty ? rows : rows.filter { $0.title.localizedStandardContains(filter) }
        let pageCount = max(1, (matchingRows.count + Self.pageSize - 1) / Self.pageSize)
        let currentPage = min(page, pageCount - 1)
        let visibleRows = Array(matchingRows.dropFirst(currentPage * Self.pageSize).prefix(Self.pageSize))
        let selectedRow = rows.first { $0.stored == stored }
        let visibleSelectedRow = visibleRows.contains { $0.id == selectedRow?.id } ? selectedRow : nil
        Form {
            Section {
                if selectedRow == nil {
                    // The file or family the preference names is gone or will
                    // not activate. Said rather than silently corrected: the
                    // preference is kept (`SettingsStore.candidateFontSelection`),
                    // and an external volume, a restore or a reinstall may bring
                    // it back.
                    Text(language.string(.desktopCustomFontMissing))
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                UserDataFilterField(text: $filter)
                fontTable(rows: visibleRows, selectedID: visibleSelectedRow?.id)
                UserDataListControls(
                    addLabelKey: .desktopCustomFontAdd,
                    // The VISIBLE selected row: a custom typeface the search
                    // has hidden must not be deletable by a button beside a
                    // table that shows no selection.
                    isRemoveEnabled: visibleSelectedRow?.customFont != nil,
                    onAdd: add,
                    onRemove: {
                        if let font = visibleSelectedRow?.customFont {
                            remove(font)
                        }
                    },
                ) {
                    UserDataListPager(
                        page: currentPage,
                        pageCount: pageCount,
                        onBackward: { page = currentPage - 1 },
                        onForward: { page = currentPage + 1 },
                    )
                }
            }
        }
        .formStyle(.grouped)
        // A new search starts from its first page; the old page number was
        // about rows that may no longer match.
        .onChange(of: filter) { page = 0 }
        .onAppear(perform: reload)
        .onReceive(FontRegistryObserver.registrationListMoved) { reload() }
        .userDataPageChrome(activity: .idle, message: $message)
    }

    /// The table's selection IS the setting. A `Table`'s selection is optional
    /// (it can be cleared); this pane's is not, because a typeface is always in
    /// use. A cleared selection is therefore ignored rather than written —
    /// including the one the search field clears by hiding the selected row,
    /// which is why the table is handed the VISIBLE selection while the
    /// setting itself stays put.
    private func fontTable(rows: [FontRow], selectedID: FontRow.ID?) -> some View {
        Table(rows, selection: Binding(
            get: { selectedID },
            set: { newValue in
                if let row = rows.first(where: { $0.id == newValue }) {
                    select(row)
                }
            },
        )) {
            TableColumn(language.string(.themeCustomFont)) { row in
                Text(row.title)
            }
        }
        .tableStyle(.inset)
        // Every row the same colour, as on the 自訂詞庫 table: settings
        // content, not a spreadsheet.
        .alternatingRowBackgrounds(.disabled)
        .frame(height: UserDataListMetrics.tableHeight(rows: Self.pageSize))
        .contextMenu(forSelectionType: FontRow.ID.self) { ids in
            if let font = rows.first(where: { $0.id == ids.first })?.customFont {
                Button(language.string(.commonDelete), role: .destructive) { remove(font) }
            }
        }
    }

    /// The bundled roster first, in the order the four platforms share, then
    /// what the user added, then what the OS has — a list that reads as "what
    /// ships", "what I added" and "what my Mac has", rather than one sorted
    /// order hiding the difference.
    private var rows: [FontRow] {
        CandidateFontChoice.allCases.map {
            FontRow(stored: .builtIn($0), title: language.string($0.labelKey), customFont: nil)
        }
            + customFonts.map {
                // The font's own name, which is text out of a file the user
                // chose — shown, never trusted.
                FontRow(stored: .customFile($0.fileName), title: $0.displayName, customFont: $0)
            }
            + installedFamilies.map {
                FontRow(stored: .installedFamily($0), title: $0, customFont: nil)
            }
    }

    /// The three stored keys, read and written as one (`StoredFontSelection`),
    /// so a row can never leave `fontType` naming a custom or installed
    /// typeface with nothing beside it.
    private var stored: StoredFontSelection {
        get {
            StoredFontSelection(
                fontType: fontTypeRawValue, customFontFile: customFontFile, installedFontFamily: installedFontFamily,
            )
        }
        nonmutating set {
            fontTypeRawValue = newValue.fontType
            customFontFile = newValue.customFontFile
            installedFontFamily = newValue.installedFontFamily
        }
    }

    private func select(_ row: FontRow) {
        stored = row.stored
        // Selecting is what activates: the candidate window reads the
        // selection on every show and registers nothing itself
        // (`SettingsStore.candidateFontSelection`).
        if let font = row.customFont {
            CustomFontLibrary.shared.activate(fileName: font.fileName)
        }
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
                    show(FontRow(stored: .customFile(font.fileName), title: font.displayName, customFont: font))
                } catch let CustomFontLibrary.ImportFailure.nameAlreadyResolves(postScriptName) {
                    // The face is already on this Mac — installed, or bundled.
                    // The user asked to type in it, not to own a copy of it, so
                    // the row that already draws it is selected (USER
                    // 2026-09-11 「跳出提示,並且跳轉到那個字型」). The file
                    // name they gave it is irrelevant: the face is known by
                    // the name inside the file.
                    reload()
                    if let row = rowDrawing(postScriptName) {
                        show(row)
                        message = .done(.desktopCustomFontAlreadyInstalled)
                    } else {
                        message = .failure(.commonImportFailed, CustomFontLibrary.ImportFailure.nameAlreadyResolves(postScriptName))
                    }
                } catch {
                    message = .failure(.commonImportFailed, error)
                    reload()
                }
            }
        }
    }

    /// Selects `row` and turns to its page with the search cleared: a row
    /// the user just added or was sent to lands after the bundled five and
    /// the other imports, which may be past the first page, and a search
    /// would hide it.
    private func show(_ row: FontRow) {
        select(row)
        filter = ""
        if let index = rows.firstIndex(where: { $0.id == row.id }) {
            page = index / Self.pageSize
        }
    }

    /// The row that already draws the face named `postScriptName`: the bundled
    /// row carrying that name, the import that carries it, or the installed
    /// row of its family.
    private func rowDrawing(_ postScriptName: String) -> FontRow? {
        let rows = rows
        if let bundled = CandidateFontChoice.allCases.first(where: { $0.postScriptName == postScriptName }) {
            return rows.first { $0.stored == .builtIn(bundled) }
        }
        if let imported = customFonts.first(where: { $0.postScriptName == postScriptName }) {
            return rows.first { $0.stored == .customFile(imported.fileName) }
        }
        guard let family = RegisteredFace.font(.postScript(postScriptName), ofSize: 12)?.familyName else { return nil }
        return rows.first { $0.stored == .installedFamily(family) }
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
        if stored == .customFile(font.fileName) {
            stored = .builtIn(.system)
        }
        for panel in CandidatePanel.allInstances {
            panel.releaseCachedPanels(drawing: font)
        }
        do {
            try CustomFontLibrary.shared.remove(font)
        } catch {
            message = .failure(.desktopCustomFontRemoveFailed, error)
        }
        reload()
    }

    /// Re-reads the library and the OS's families.
    ///
    /// Forced rather than asked: the scan is invalidated by whoever changed the
    /// directory, but the user may have been in Finder — or Font Book — since
    /// this pane was last on screen. The families this process registered
    /// itself are left out of the installed list — they are rows already.
    private func reload() {
        CustomFontLibrary.shared.invalidateCache()
        customFonts = CustomFontLibrary.shared.installedFonts()
        installedFamilies = RegisteredFace.installedFamilies(
            excluding: Self.bundledFamilies.union(CustomFontLibrary.shared.registeredFamilies()),
        )
    }

    /// The families of the bundled roster — registered at launch and never
    /// withdrawn, so resolved once. Left out of the installed list for the same
    /// reason the library's own registrations are (`CustomFontLibrary
    /// .registeredFamilies`).
    private static let bundledFamilies: Set<String> = Set(
        CandidateFontChoice.allCases
            .compactMap(\.postScriptName)
            .compactMap { RegisteredFace.font(.postScript($0), ofSize: 12)?.familyName },
    )
}
