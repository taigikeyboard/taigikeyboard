// Keyboard key definition used by TaigiLayouts to describe each layout;
// LayoutConverter turns [[KeyDef]] into a KeyboardKit KeyboardLayout.

enum KeyDef {
    // MARK: - Character keys

    /// `fullWidth` is the optional full-width form used in isTranslateSwapped mode.
    case char(String, fullWidth: String? = nil)

    // MARK: - Function keys

    case shift
    case backspace
    case space
    case `return`
    // Hanji <-> romanization toggle; switches the script candidates are shown in.
    case translate

    // MARK: - Keyboard switching

    case numeric
    case symbolic
    case alphabetic

    // MARK: - System keys

    case globe
    case emoji
}
