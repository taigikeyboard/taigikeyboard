// Executable shell. All behaviour lives in TaigiInputMethodCore so `swift test`
// can import it; NSApplicationMain instantiates the Info.plist NSPrincipalClass
// (`TaigiInputMethodApplication`), which installs the app delegate.

import AppKit
import TaigiInputMethodCore

// Referenced so the linker keeps the Objective-C classes the Info.plist looks
// up by name; nothing in Swift code calls them directly.
_ = TaigiInputMethodApplication.self
_ = TaigiInputController.self

exit(NSApplicationMain(CommandLine.argc, CommandLine.unsafeArgv))
