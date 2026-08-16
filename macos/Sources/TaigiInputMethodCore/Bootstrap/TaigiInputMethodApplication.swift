// NSApplication subclass named by `NSPrincipalClass` in the bundle's Info.plist.

import AppKit

/// Owns the application delegate. An input method bundle has no MainMenu.nib,
/// so nothing else would install one.
///
/// `@objc(TaigiInputMethodApplication)` pins the Objective-C runtime name that
/// `NSPrincipalClass` looks up, so the plist value stays correct regardless of
/// which Swift module this type ends up in.
@objc(TaigiInputMethodApplication)
public final class TaigiInputMethodApplication: NSApplication {
    private let inputMethodDelegate = AppDelegate()

    override public init() {
        super.init()
        delegate = inputMethodDelegate
    }

    @available(*, unavailable)
    public required init?(coder _: NSCoder) {
        fatalError("TaigiInputMethodApplication is not created from a nib")
    }
}
