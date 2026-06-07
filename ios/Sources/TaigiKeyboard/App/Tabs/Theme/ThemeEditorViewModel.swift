// 中文: 自訂主題編輯器的 ViewModel。持有單一 ThemeAppearance 草稿 + 名稱,
// 中文: Save 才寫入(addUserTheme / updateUserTheme)並自動套用;Cancel 直接丟棄。

import Foundation
import SwiftUI

/// Draft state for the user-theme editor.
///
/// Holds a single `@Published appearance` (the whole bundle) plus the name, and
/// the optional identity of the theme being edited (`nil` = creating a new one).
/// Nothing is persisted until `save()`. New themes auto-apply on save.
///
/// Bindings into `appearance` always publish via copy-back
/// (`var next = appearance; next.x = v; appearance = next`) so SwiftUI re-renders
/// the live preview on every edit.
// 中文: 草稿狀態。單一 @Published appearance + name + 編輯中身分(nil = 新增)。Save 才落盤,新主題存後自動套用。
@MainActor
final class ThemeEditorViewModel: ObservableObject {
    @Published var name: String
    @Published var appearance: ThemeAppearance

    /// `nil` when creating a new theme; the existing id when editing.
    private let editingId: UUID?
    private let createdAt: Date
    private let settings = SharedSettings.shared

    /// Creates a fresh draft (new theme) or a prefilled draft (editing `existing`).
    init(editing existing: UserTheme? = nil) {
        if let existing {
            editingId = existing.id
            name = existing.name
            appearance = existing.appearance
            createdAt = existing.createdAt
        } else {
            editingId = nil
            name = ""
            appearance = .default
            createdAt = Date()
        }
    }

    /// Whether the title is for an edit (vs. a new theme).
    var isEditing: Bool { editingId != nil }

    /// Non-empty trimmed name is the only save gate.
    var isSaveEnabled: Bool {
        !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// Persists the draft and auto-applies it. Returns `false` only when adding a
    /// NEW theme fails the cap (`UserThemeStore.maxUserThemes`) — the caller then
    /// surfaces the cap alert. Editing never hits the cap.
    func save() -> Bool {
        let id = editingId ?? UUID()
        let now = Date()
        let theme = UserTheme(
            id: id,
            name: name.trimmingCharacters(in: .whitespacesAndNewlines),
            appearance: appearance,
            createdAt: createdAt,
            updatedAt: now,
        )
        if editingId == nil {
            guard settings.addUserTheme(theme) else { return false }
        } else {
            settings.updateUserTheme(theme)
        }
        settings.selectedThemeId = id.uuidString
        return true
    }

    // MARK: - Publish-safe bindings into the draft

    func setFontType(_ font: FontType) {
        var next = appearance
        next.fontType = font
        appearance = next
    }

    /// Binding for a 6-role color; `nil` field shows `defaultColor` (adaptive).
    func colorBinding(
        _ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>,
        default defaultColor: Color,
    ) -> Binding<Color> {
        Binding(
            get: { self.appearance.colors[keyPath: keyPath]?.color ?? defaultColor },
            set: { newColor in
                var next = self.appearance
                next.colors[keyPath: keyPath] = CodableColor(newColor)
                self.appearance = next
            },
        )
    }

    func isColorCustomized(_ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>) -> Bool {
        appearance.colors[keyPath: keyPath] != nil
    }

    func resetColor(_ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>) {
        var next = appearance
        next.colors[keyPath: keyPath] = nil
        appearance = next
    }

    /// Binding for a scalar appearance field (sliders).
    func scalarBinding(_ keyPath: WritableKeyPath<ThemeAppearance, Double>) -> Binding<Double> {
        Binding(
            get: { self.appearance[keyPath: keyPath] },
            set: { newValue in
                var next = self.appearance
                next[keyPath: keyPath] = newValue
                self.appearance = next
            },
        )
    }
}
