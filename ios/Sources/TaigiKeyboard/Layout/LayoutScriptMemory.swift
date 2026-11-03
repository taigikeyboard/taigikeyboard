import Foundation

// The layout each romanization script last used, so switching Tâi-lô ↔ Pe̍h-ōe-jī brings it back.

/// A layout selection: a base and, while a custom layout is selected, its id.
struct LayoutSelection: Equatable {
    let base: KeyboardLayoutType
    let customId: UUID?

    var storage: String {
        customId.map { "\(base.rawValue)|\($0.uuidString)" } ?? base.rawValue
    }
}

extension LayoutSelection {
    /// `"<base>"` or `"<base>|<custom id>"`; the same spelling on Android.
    init?(storage: String) {
        let parts = storage.split(separator: "|", omittingEmptySubsequences: false)
        guard let base = parts.first.flatMap({ KeyboardLayoutType(rawValue: String($0)) }),
              KeyboardLayoutType.romanizationLayouts.contains(base), parts.count <= 2
        else {
            return nil
        }
        self.init(base: base, customId: parts.count == 2 ? UUID(uuidString: String(parts[1])) : nil)
    }
}

enum LayoutScriptMemory {
    /// The selection to show after entering `script`: the one `remembered` for it, else `live` —
    /// either one's custom layout only while it is still live over that base (`CustomLayout.active`)
    /// and shows in `script` (`CustomLayout.isUniversal`, or made for it), else its base.
    static func selection(
        entering script: LayoutScript,
        remembered: LayoutSelection?,
        live: LayoutSelection,
        layouts: [CustomLayout],
    ) -> LayoutSelection {
        let selection = remembered ?? live
        let layout = CustomLayout.active(in: layouts, selectedId: selection.customId, layoutType: selection.base)
        let fits = layout.map { $0.isUniversal || $0.script == script } ?? false
        return fits ? selection : LayoutSelection(base: selection.base, customId: nil)
    }
}
