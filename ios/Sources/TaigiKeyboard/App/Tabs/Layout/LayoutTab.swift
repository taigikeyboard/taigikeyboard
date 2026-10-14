import SwiftUI

/// Layout tab.
///
/// A gallery of horizontal shelves (`GalleryShelf`, shared with the Theme tab): Custom Layouts
/// (the user's layouts after a create card that opens `LayoutEditorView`), Universal (layouts
/// with one key table for both scripts), Tâi-lô and Pe̍h-ōe-jī (the layouts whose POJ keys
/// differ, once per script), and Phonetic Symbols. A Tâi-lô / Pe̍h-ōe-jī card applies its layout
/// and switches the input mode to that script.
struct LayoutTab: View {
    @Environment(DisplayLanguageStore.self) private var lang

    // Read-only observers of the App Group store, so a mode change on the Settings tab (or in the
    // keyboard) moves the checkmark; every write goes through `SharedSettings`.
    @AppStorage(SharedSettings.keyboardLayoutTypeKey.key, store: SharedSettings.sharedUserDefaults)
    private var keyboardLayoutType = SharedSettings.keyboardLayoutTypeKey.defaultValue
    @AppStorage(SharedSettings.inputModeKey.key, store: SharedSettings.sharedUserDefaults)
    private var inputMode = SharedSettings.inputModeKey.defaultValue
    @AppStorage(SharedSettings.selectedCustomLayoutIdKey.key, store: SharedSettings.sharedUserDefaults)
    private var selectedCustomLayoutId = SharedSettings.selectedCustomLayoutIdKey.defaultValue
    @AppStorage(SharedSettings.layoutRevisionKey.key, store: SharedSettings.sharedUserDefaults)
    private var layoutRevision = SharedSettings.layoutRevisionKey.defaultValue

    @State private var customLayouts: [CustomLayout] = []
    @State private var editorRoute: LayoutEditorRoute?

    var body: some View {
        let activeCustomLayoutId = CustomLayout.active(
            in: customLayouts, selectedId: UUID(uuidString: selectedCustomLayoutId), layoutType: keyboardLayoutType,
        )?.id
        NavigationStack {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: GalleryCardMetrics.shelfSpacing) {
                    GalleryShelf(title: lang.string(.layoutCustomLayoutsSection)) {
                        if customLayouts.count < UserLayoutStore.maxLayouts {
                            GalleryCreateCard(title: lang.string(.layoutCreateNewLayout)) { editorRoute = .create }
                        }
                        ForEach(customLayouts) { layout in
                            customLayoutCard(layout, isSelected: activeCustomLayoutId == layout.id)
                        }
                    }

                    ForEach(LayoutChoice.shelves, id: \.titleKey) { shelf in
                        GalleryShelf(title: lang.string(shelf.titleKey)) {
                            ForEach(shelf.choices, id: \.self) { choice in
                                let title = lang.string(choice.layout.displayNameKey)
                                GalleryCard(
                                    title: title,
                                    isSelected: activeCustomLayoutId == nil
                                        && choice.isSelected(layout: keyboardLayoutType, inputMode: inputMode),
                                    onTap: { select(choice) },
                                ) {
                                    GalleryScreenshot(imageName: choice.previewImageName, title: title)
                                }
                            }
                        }
                    }
                }
                .padding(.vertical, AppStyle.horizontalPadding)
            }
            // Matches the Form-backed tabs (systemGroupedBackground); a bare ScrollView defaults to systemBackground.
            .background(Color(.systemGroupedBackground))
            .navigationTitle(lang.string(TabType.layout.titleKey))
            .navigationBarTitleDisplayMode(.large)
            .navigationDestination(item: $editorRoute) { route in
                switch route {
                case .create: LayoutEditorView()
                case let .edit(layout): LayoutEditorView(editing: layout)
                }
            }
            // Every store write bumps layoutRevision; onAppear covers popping back from the editor.
            .onAppear(perform: reloadCustomLayouts)
            .onChange(of: layoutRevision) { _, _ in reloadCustomLayouts() }
        }
    }

    private func customLayoutCard(_ layout: CustomLayout, isSelected: Bool) -> some View {
        GalleryCard(title: layout.name, isSelected: isSelected, onTap: { apply(layout) }) {
            LayoutKeyGrid(rows: layout.rows(withGlobe: false))
        } accessory: {
            GalleryCardMenu(title: layout.name) {
                Button(lang.string(.themeCardMenuApply)) { apply(layout) }
                Button(lang.string(.commonEdit)) { editorRoute = .edit(layout) }
                Button(lang.string(.commonDelete), role: .destructive) {
                    SharedSettings.shared.deleteCustomLayout(id: layout.id)
                }
            }
        }
    }

    private func select(_ choice: LayoutChoice) {
        withAnimation(.easeInOut(duration: 0.15)) {
            choice.apply(to: SharedSettings.shared)
        }
    }

    private func apply(_ layout: CustomLayout) {
        withAnimation(.easeInOut(duration: 0.15)) {
            SharedSettings.shared.applyCustomLayout(layout)
        }
    }

    private func reloadCustomLayouts() {
        customLayouts = SharedSettings.shared.loadCustomLayouts()
    }
}

// MARK: - Editor route

/// Navigation route for the layout editor: a new layout, or an edit of a saved one.
enum LayoutEditorRoute: Hashable {
    case create
    case edit(CustomLayout)
}

// MARK: - Layout choice

/// One Layout-tab card: a layout plus, on the Tâi-lô / Pe̍h-ōe-jī shelves, the script it applies.
struct LayoutChoice: Hashable {
    struct Shelf {
        let titleKey: StringKey
        let choices: [LayoutChoice]
    }

    let layout: KeyboardLayoutType
    /// nil on the Universal / Phonetic Symbols shelves: the card writes the layout only.
    let script: LayoutScript?

    static let shelves: [Shelf] = {
        let universal = KeyboardLayoutType.romanizationLayouts.filter { $0.pojPreviewImageName == nil }
        let perScript = KeyboardLayoutType.romanizationLayouts.filter { $0.pojPreviewImageName != nil }
        return [
            Shelf(titleKey: .layoutCommonLayoutsSection, choices: universal.map { LayoutChoice(layout: $0, script: nil) }),
            Shelf(titleKey: .settingsTlMode, choices: perScript.map { LayoutChoice(layout: $0, script: .tl) }),
            Shelf(titleKey: .settingsPojMode, choices: perScript.map { LayoutChoice(layout: $0, script: .poj) }),
            Shelf(titleKey: .settingsTpsMode, choices: [LayoutChoice(layout: .tps, script: nil)]),
        ]
    }()

    var previewImageName: String {
        layout.previewImageName(for: script?.inputMode ?? .tl)
    }

    /// A script card is selected only in its own input mode (English or TPS selects neither);
    /// a layout-only card follows the stored layout.
    func isSelected(layout selectedLayout: KeyboardLayoutType, inputMode: InputMode) -> Bool {
        guard selectedLayout == layout else { return false }
        guard let script else { return true }
        return inputMode == script.inputMode
    }

    /// Writes the layout, then the script's input mode: leaving TPS restores the pre-TPS mode,
    /// which the card's script then overrides.
    func apply(to settings: SharedSettings) {
        settings.keyboardLayoutType = layout
        if let script {
            settings.inputMode = script.inputMode
        }
    }
}
