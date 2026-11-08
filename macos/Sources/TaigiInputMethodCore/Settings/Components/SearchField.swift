// The AppKit search control, in SwiftUI dress.

import AppKit
import SwiftUI

/// An `NSSearchField` — the magnifier, the rounded well and the clear button
/// AppKit draws for a search box — usable from a SwiftUI form.
///
/// SwiftUI ships no search-field style. Its one built-in route to the control
/// is `.searchable`, which hoists the field into the window's toolbar; this
/// window's toolbar is shared by all five panes, so a field put there would
/// appear on panes with nothing to search. A pane-local search box has to be
/// the AppKit control, wrapped.
struct SearchField: NSViewRepresentable {
    let placeholder: String
    @Binding var text: String

    func makeNSView(context: Context) -> NSSearchField {
        let field = NSSearchField()
        field.delegate = context.coordinator
        return field
    }

    func updateNSView(_ field: NSSearchField, context: Context) {
        // Re-pointed every update: a `Binding` captured once at
        // `makeCoordinator` time would go on writing to the view value that
        // existed then.
        context.coordinator.text = $text
        // Both writes are guarded, and this runs on every keystroke: setting
        // either one marks the cell for display, and assigning `stringValue`
        // also moves the insertion point to the end of the field — echoing
        // back what the user just typed would fight their cursor.
        if field.placeholderString != placeholder {
            field.placeholderString = placeholder
        }
        if field.stringValue != text {
            field.stringValue = text
        }
        // A wrapped control does not take SwiftUI's `.controlSize` on its own:
        // the size and its font are handed over here.
        let size = Self.appKitSize(context.environment.controlSize)
        if field.controlSize != size {
            field.controlSize = size
            field.font = .systemFont(ofSize: NSFont.systemFontSize(for: size))
            field.invalidateIntrinsicContentSize()
        }
    }

    private static func appKitSize(_ size: ControlSize) -> NSControl.ControlSize {
        switch size {
        case .mini: .mini
        case .small: .small
        case .regular: .regular
        case .large, .extraLarge: .large
        @unknown default: .regular
        }
    }

    /// Width from the row it sits in, height from the control: a search field
    /// stretches like a text field but is never taller than AppKit draws it.
    func sizeThatFits(
        _ proposal: ProposedViewSize,
        nsView: NSSearchField,
        context _: Context,
    ) -> CGSize? {
        CGSize(
            width: proposal.width ?? nsView.intrinsicContentSize.width,
            height: nsView.intrinsicContentSize.height,
        )
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    /// Reports every keystroke rather than only what the field's own action
    /// sends: the list this filters reloads on a debounce of its own
    /// (`reloadWhenFilterSettles`), so the binding has to see the text as it
    /// is typed.
    final class Coordinator: NSObject, NSSearchFieldDelegate {
        var text: Binding<String>

        init(text: Binding<String>) {
            self.text = text
        }

        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSSearchField else { return }
            text.wrappedValue = field.stringValue
        }
    }
}
