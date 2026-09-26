import Foundation
import SwiftUI

/// Draft state for the user-theme editor.
///
/// Holds a single `@Published appearance` (the whole bundle) plus the name, and
/// the optional identity of the theme being edited (`nil` = creating a new one).
/// Nothing is persisted until `save()`. New themes auto-apply on save.
///
/// A draft always carries concrete colors: a new theme starts from
/// `ThemeAppearance.userThemeSeed`, an edited theme was seeded on load
/// (`UserThemeStore.load`). So a user theme never follows light / dark mode and
/// every color row has a concrete value to show and to reset to.
///
/// Bindings into `appearance` always publish via copy-back
/// (`var next = appearance; next.x = v; appearance = next`) so SwiftUI re-renders
/// the live preview on every edit.
@MainActor
final class ThemeEditorViewModel: ObservableObject {
    @Published var name: String
    @Published var appearance: ThemeAppearance
    /// The Background segmented choice. Normally `background.kind`; it can run ahead of the
    /// background while Photo is chosen but no photo has been picked yet, so the picker
    /// row shows without the surface changing (the draft keeps its solid / gradient
    /// until a photo lands).
    @Published private(set) var backgroundKind: ThemeBackground.Kind

    /// `nil` when creating a new theme; the existing id when editing.
    private let editingId: UUID?
    private let createdAt: Date
    private let settings = SharedSettings.shared

    /// Creates a fresh draft (new theme) or a prefilled draft (editing `existing`).
    init(editing existing: UserTheme? = nil) {
        let initial = existing?.appearance ?? .userThemeSeed
        editingId = existing?.id
        name = existing?.name ?? ""
        appearance = initial
        createdAt = existing?.createdAt ?? Date()
        backgroundKind = (initial.colors.background ?? UserThemeSeed.background).kind
    }

    /// Whether the title is for an edit (vs. a new theme).
    var isEditing: Bool {
        editingId != nil
    }

    /// For a NEW theme, whether the store is below the cap; always `true` when
    /// editing (updates never hit the cap). The Save flow checks this before
    /// prompting for a name so the cap alert and the name alert never chain.
    var canSaveNewTheme: Bool {
        editingId != nil || settings.loadUserThemes().count < UserThemeStore.maxUserThemes
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

    // MARK: - Background

    /// The draft's background; the seed is the last-resort read so the editor never
    /// shows "adaptive" (a seeded draft always has one).
    private var background: ThemeBackground {
        appearance.colors.background ?? UserThemeSeed.background
    }

    private func setBackground(_ background: ThemeBackground) {
        var next = appearance
        next.colors.background = background
        appearance = next
    }

    /// Segmented Solid / Gradient / Photo choice. Switching keeps the current hue: solid → gradient
    /// runs the solid color into a lighter tint of it; gradient → solid keeps the first
    /// stop; leaving a photo lands on the seed colour. Choosing Photo changes nothing until
    /// a photo is picked (`setPhoto`).
    var backgroundKindBinding: Binding<ThemeBackground.Kind> {
        Binding(
            get: { self.backgroundKind },
            set: { kind in
                self.backgroundKind = kind
                guard kind != self.background.kind else { return }
                let solidColor = self.background.solidColor ?? self.background.gradient?.stops[0] ?? UserThemeSeed.solidColor
                switch kind {
                case .solid: self.setBackground(.solid(solidColor))
                case .gradient: self.setBackground(.gradient(.seeded(from: solidColor)))
                case .image: return
                }
            },
        )
    }

    /// The draft photo, or nil while Photo is chosen but nothing has been picked.
    var photo: ThemeImageBackground? {
        background.image
    }

    /// Makes the stored photo `file` (see `ThemeImageStore.save`) the background, keeping
    /// the current Fade when replacing a photo; a new photo starts centred and unzoomed.
    func setPhoto(file: String) {
        setBackground(.image(ThemeImageBackground(file: file, dim: photo?.dim ?? ThemeImageBackground.defaultDim)))
    }

    var photoDimBinding: Binding<Double> {
        Binding(
            get: { self.photo?.dim ?? ThemeImageBackground.defaultDim },
            set: { dim in
                guard let photo = self.photo else { return }
                self.setBackground(.image(photo.with(dim: dim)))
            },
        )
    }

    /// The draft photo for `PhotoPositionOverlay` (which moves and zooms it), or nil while none is picked.
    var photoBinding: Binding<ThemeImageBackground>? {
        guard let photo else { return nil }
        return Binding(
            get: { self.photo ?? photo },
            set: { self.setBackground(.image($0)) },
        )
    }

    /// The solid background color (row shown only while the kind is Solid).
    var solidBackgroundBinding: Binding<Color> {
        Binding(
            get: { (self.background.solidColor ?? UserThemeSeed.solidColor).color },
            set: { self.setBackground(.solid(CodableColor($0))) },
        )
    }

    var isSolidBackgroundCustomized: Bool {
        background != UserThemeSeed.background
    }

    func resetSolidBackground() {
        setBackground(UserThemeSeed.background)
    }

    /// The current gradient (rows shown only while the kind is Gradient).
    private var gradient: ThemeGradient {
        background.gradient ?? .seeded(from: UserThemeSeed.solidColor)
    }

    /// Binding for one of the two gradient stops (`0` = start, `1` = end).
    func gradientStopBinding(_ index: Int) -> Binding<Color> {
        Binding(
            get: { self.gradient.stops[index].color },
            set: { newColor in
                var next = self.gradient
                next.stops[index] = CodableColor(newColor)
                self.setBackground(.gradient(next))
            },
        )
    }

    var gradientAngleBinding: Binding<Double> {
        Binding(
            get: { self.gradient.angle },
            set: { angle in
                // The preview drag writes at touch-sample rate; skip the no-op frames
                // (same whole degree / snapped preset) so the preview is not rebuilt for them.
                guard angle != self.gradient.angle else { return }
                var next = self.gradient
                next.angle = angle
                self.setBackground(.gradient(next))
            },
        )
    }

    // MARK: - Role colors (seeded, never nil)

    func colorBinding(_ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>) -> Binding<Color> {
        Binding(
            get: { (self.appearance.colors[keyPath: keyPath] ?? UserThemeSeed.color(keyPath)).color },
            set: { newColor in
                var next = self.appearance
                next.colors[keyPath: keyPath] = CodableColor(newColor)
                self.appearance = next
            },
        )
    }

    func isColorCustomized(_ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>) -> Bool {
        appearance.colors[keyPath: keyPath] != UserThemeSeed.color(keyPath)
    }

    func resetColor(_ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>) {
        var next = appearance
        next.colors[keyPath: keyPath] = UserThemeSeed.color(keyPath)
        appearance = next
    }

    /// Resets the whole draft appearance to the user-theme seed. Draft-only: the name
    /// is kept, nothing is persisted, and the applied theme stays untouched until
    /// `save()`. `ThemeAppearance` is a value type, so this cannot leak to the live theme.
    func resetToDefaults() {
        appearance = .userThemeSeed
        backgroundKind = background.kind
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
