// Theme Tab(主題)— 鍵盤外觀/配色主頁面。root = 主題選擇器 gallery(ThemePickerView);
// 自訂主題經 Custom Themes 的「Create New…」/ 卡片 Edit 進入 ThemeEditorView。

import SwiftUI

/// Theme tab.
///
/// Root is the theme-picker gallery (`ThemePickerView`): a global font entry, a
/// `Custom Themes` shelf whose `Create New…` card opens `ThemeEditorView`, plus
/// the built-in family shelves. The built-in catalog is layout-only for now;
/// real palettes are authored in a later pass.
// 主題 Tab View — 呈現主題選擇器 gallery。
struct ThemeTab: View {
    var body: some View {
        NavigationStack {
            ThemePickerView()
        }
    }
}
