import SwiftUI

/// Layout tab.
///
/// A gallery of horizontal shelves (`GalleryShelf`, shared with the Theme tab): Custom Layouts
/// (a create card, not available yet), Universal (layouts with one key table for both scripts),
/// Tâi-lô and Pe̍h-ōe-jī (the layouts whose POJ keys differ, once per script), and Phonetic
/// Symbols. A Tâi-lô / Pe̍h-ōe-jī card applies its layout and switches the input mode to that
/// script.
struct LayoutTab: View {
    @Environment(DisplayLanguageStore.self) private var lang

    // Read-only observers of the App Group store, so a mode change on the Settings tab (or in the
    // keyboard) moves the checkmark; every write goes through `LayoutChoice.apply(to:)`.
    @AppStorage(SharedSettings.keyboardLayoutTypeKey.key, store: SharedSettings.sharedUserDefaults)
    private var keyboardLayoutType = SharedSettings.keyboardLayoutTypeKey.defaultValue
    @AppStorage(SharedSettings.inputModeKey.key, store: SharedSettings.sharedUserDefaults)
    private var inputMode = SharedSettings.inputModeKey.defaultValue

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: GalleryCardMetrics.shelfSpacing) {
                    GalleryShelf(title: lang.string(.layoutCustomLayoutsSection)) {
                        GalleryCreateCard(
                            title: lang.string(.layoutCreateNewLayout),
                            comingSoonBadge: lang.string(.layoutComingSoon),
                        )
                    }

                    ForEach(LayoutChoice.shelves, id: \.titleKey) { shelf in
                        GalleryShelf(title: lang.string(shelf.titleKey)) {
                            ForEach(shelf.choices, id: \.self) { choice in
                                let title = lang.string(choice.layout.displayNameKey)
                                GalleryCard(
                                    title: title,
                                    isSelected: choice.isSelected(layout: keyboardLayoutType, inputMode: inputMode),
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
        }
    }

    private func select(_ choice: LayoutChoice) {
        withAnimation(.easeInOut(duration: 0.15)) {
            choice.apply(to: SharedSettings.shared)
        }
    }
}

// MARK: - Layout choice

/// One Layout-tab card: a layout plus, on the Tâi-lô / Pe̍h-ōe-jī shelves, the script it applies.
struct LayoutChoice: Hashable {
    /// The script of a Tâi-lô / Pe̍h-ōe-jī shelf card.
    enum Script {
        case tl
        case poj

        var inputMode: InputMode {
            switch self {
            case .tl: .tl
            case .poj: .poj
            }
        }
    }

    struct Shelf {
        let titleKey: StringKey
        let choices: [LayoutChoice]
    }

    let layout: KeyboardLayoutType
    /// nil on the Universal / Phonetic Symbols shelves: the card writes the layout only.
    let script: Script?

    private static let romanizationLayouts = KeyboardLayoutType.allCases.filter { $0 != .tps }

    static let shelves: [Shelf] = {
        let universal = romanizationLayouts.filter { $0.pojPreviewImageName == nil }
        let perScript = romanizationLayouts.filter { $0.pojPreviewImageName != nil }
        return [
            Shelf(titleKey: .layoutCommonLayoutsSection, choices: universal.map { LayoutChoice(layout: $0, script: nil) }),
            Shelf(titleKey: .settingsTlMode, choices: perScript.map { LayoutChoice(layout: $0, script: .tl) }),
            Shelf(titleKey: .settingsPojMode, choices: perScript.map { LayoutChoice(layout: $0, script: .poj) }),
            Shelf(titleKey: .settingsTpsMode, choices: [LayoutChoice(layout: .tps, script: nil)]),
        ]
    }()

    var previewImageName: String {
        script == .poj ? layout.pojPreviewImageName ?? layout.previewImageName : layout.previewImageName
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
