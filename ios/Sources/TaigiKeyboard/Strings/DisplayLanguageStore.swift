// Reactive root state for the app UI display language; drives SwiftUI live-switch with no restart.

import Foundation
import SwiftUI

/// Holds the active `DisplayLanguage` for the running process and exposes it to the SwiftUI tree via
/// `.environment`. `@Observable` so reading `string(_:)` inside a `body` registers a precise dependency
/// on the stored `resolver`: changing the language recomposes only the views that read a localized
/// string — no Activity/keyboard restart (plan D7 live-switch).
///
/// Host: a single store injected at `AppRootView`; `setLanguage` persists the selection to App-Group
/// settings. Extension: its own store (separate process). The App-Group value is the cross-process data
/// channel, but `UserDefaults.didChangeNotification` only fires for in-process writes (Apple contract), so
/// it is a best-effort wake at most; the reliable re-read is `syncFromSettings()` at each language surface's
/// appear point (the keyboard overlays call it from `.onAppear`).
@MainActor
@Observable
final class DisplayLanguageStore {
    private(set) var resolver: StringResolver

    /// The picker selection — CAN be `.system` (Automatic). Drives the picker checkmark + the Settings
    /// row's trailing label. The string resolver is NOT built from this directly; it is built from the
    /// EFFECTIVE language (`selected.effectiveLanguage(deviceSubtag)`), which `resolver.language` carries —
    /// so `.system` resolves to a real authored language and the resolver never sees `.system`.
    private(set) var selected: DisplayLanguage

    init(language: DisplayLanguage) {
        selected = language
        resolver = StringResolver(language.effectiveLanguage(Self.deviceLanguageSubtag()))
    }

    /// Builds a store from the persisted App-Group tag — the host launch + extension sync entry point.
    convenience init() {
        self.init(language: DisplayLanguage.fromTag(SharedSettings.shared.displayLanguage))
    }

    /// Localized string for `key` under the active language. Reading this in a SwiftUI `body` is what
    /// makes the view live-switch.
    func string(_ key: StringKey) -> String {
        resolver.resolve(key)
    }

    /// The EFFECTIVE display language currently rendering — already resolved away from `.system` (the
    /// resolver is built from `effectiveLanguage`). JSON content models resolve against it via
    /// `LocalizedContentText.resolve(for:)`. Reading it in a `body` registers the same live-switch
    /// dependency on `resolver` as `string(_:)`.
    var language: DisplayLanguage { resolver.language }

    /// Display label for a selectable `DisplayLanguage`, shared by the picker rows and the Settings-row
    /// trailing value so neither special-cases `.system` on its own: `.system` → the localized Automatic
    /// string, every authored language → its (language-invariant) endonym.
    func selectionLabel(for language: DisplayLanguage) -> String {
        language == .system ? string(.settingsDisplayLanguageAutomatic) : language.endonym
    }

    /// Persists `language` and updates the live state. Persisting writes the App-Group value, so the
    /// extension (separate process) picks it up via its settings-change observer. `.system` persists
    /// the tag `"system"`, so the Automatic selection survives a relaunch.
    func setLanguage(_ language: DisplayLanguage) {
        SharedSettings.shared.displayLanguage = language.tag
        apply(selected: language)
    }

    /// Re-reads the persisted tag into the live state. The extension calls this when a language surface
    /// appears (overlay `.onAppear`) so a host-side language change reflects without a keyboard restart.
    ///
    /// Recomputes `effective` from the OS locale EVEN WHEN `selected` is unchanged: under `.system` the
    /// persisted tag stays `"system"` but the device OS language can change between appearances, so the
    /// rebuild must key on `effective` (the concrete resolved language), not on `selected`.
    func syncFromSettings() {
        apply(selected: DisplayLanguage.fromTag(SharedSettings.shared.displayLanguage))
    }

    /// Updates `selected` + recomputes the effective language from the current device locale, rebuilding
    /// the resolver only when the effective (concrete) language actually changed. Keying the rebuild on
    /// the effective language (`resolver.language`, not `selected`) is what lets a SYSTEM refresh pick up
    /// an OS-language change while the persisted tag stays `"system"`.
    private func apply(selected newSelected: DisplayLanguage) {
        let newEffective = newSelected.effectiveLanguage(Self.deviceLanguageSubtag())
        selected = newSelected
        guard newEffective != resolver.language else { return }
        resolver = StringResolver(newEffective)
    }

    /// The device's preferred-language subtag (lowercased ISO 639), or `""` when unavailable. Reads
    /// `Locale.preferredLanguages.first` (the user's OS language-preference order — NOT `Locale.current`,
    /// which a per-app override can mask) and extracts its language code.
    private static func deviceLanguageSubtag() -> String {
        guard let preferred = Locale.preferredLanguages.first,
              let code = Locale(identifier: preferred).language.languageCode?.identifier
        else { return "" }
        return code.lowercased()
    }
}
