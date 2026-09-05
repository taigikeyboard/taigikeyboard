import SwiftUI

/// Theme tab.
///
/// Root is the theme-picker gallery (`ThemePickerView`): a global font entry, a
/// `Custom Themes` shelf whose `Create New…` card opens `ThemeEditorView`, plus
/// the built-in family shelves. The built-in catalog is layout-only for now;
/// real palettes are authored in a later pass.
struct ThemeTab: View {
    var body: some View {
        NavigationStack {
            ThemePickerView()
        }
    }
}
