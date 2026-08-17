// Reactive root state for the app UI display language; drives SwiftUI live-switch with no restart.

import Foundation
import Observation

/// Holds the active `DisplayLanguage` for the running process and exposes it to the SwiftUI tree via
/// `.environment`. `@Observable` so reading `string(_:)` inside a `body` registers a precise dependency
/// on the stored `resolver`: changing the language recomposes only the views that read a localized
/// string, with no window rebuild.
///
/// The input method is a single process, so one store per process is the intended ownership: the
/// settings form, the menu, and the candidate UI all read the same instance. It observes the persisted
/// key directly, so a language written from anywhere — the picker's `@AppStorage` binding, a shortcut,
/// or `defaults write` from a terminal — lands here without a notification path of its own.
@MainActor
@Observable
final class DisplayLanguageStore {
    private(set) var resolver: StringResolver

    /// The picker selection — CAN be `.system` (Automatic). Drives the picker checkmark and the
    /// settings row's trailing label. The resolver is NOT built from this directly; it is built from the
    /// EFFECTIVE language (`selected.effectiveLanguage(deviceSubtag)`), so `.system` never reaches it.
    private(set) var selected: DisplayLanguage

    private let settings: SettingsStore
    private let deviceLanguageSubtag: @Sendable () -> String
    private var observation: AnyObject?

    /// `deviceLanguageSubtag` is injected rather than read from `Locale` inline so a test pins the
    /// Automatic outcome instead of inheriting the language of whatever machine runs it.
    init(
        settings: SettingsStore,
        deviceLanguageSubtag: @escaping @Sendable () -> String = DisplayLanguageStore.systemLanguageSubtag,
    ) {
        self.settings = settings
        self.deviceLanguageSubtag = deviceLanguageSubtag
        let language = DisplayLanguage.fromTag(settings.displayLanguage)
        selected = language
        resolver = StringResolver(language.effectiveLanguage(deviceLanguageSubtag()))
        observation = settings.observeChanges(of: SettingsStore.Keys.displayLanguage) { [weak self] in
            // The observation fires on whichever thread wrote the value, so hop before touching
            // MainActor state. Each hop re-reads the current tag rather than applying a captured one,
            // so out-of-order callbacks still converge on what is actually stored.
            Task { @MainActor in self?.syncFromSettings() }
        }
    }

    /// Localized string for `key` under the active language. Reading this in a SwiftUI `body` is what
    /// makes the view live-switch.
    func string(_ key: StringKey) -> String {
        resolver.resolve(key)
    }

    /// The EFFECTIVE display language currently rendering — already resolved away from `.system`.
    var language: DisplayLanguage {
        resolver.language
    }

    /// Display label for a selectable `DisplayLanguage`, shared by the picker rows and the settings
    /// row's trailing value so neither special-cases `.system` on its own: `.system` → the localized
    /// Automatic string, every authored language → its (language-invariant) endonym.
    func selectionLabel(for language: DisplayLanguage) -> String {
        language == .system ? string(.settingsDisplayLanguageAutomatic) : language.endonym
    }

    /// Persists `language` and updates the live state. The write also feeds the observation, so a
    /// surface bound straight to the defaults key stays in step with one written through here.
    func setLanguage(_ language: DisplayLanguage) {
        settings.displayLanguage = language.tag
        apply(selected: language)
    }

    /// Re-reads the persisted tag into the live state — the observation's entry point, and the way a
    /// surface can resynchronize on appear.
    ///
    /// Recomputes the effective language EVEN WHEN `selected` is unchanged, because under `.system`
    /// the persisted tag stays `"system"` while the device OS language can change underneath it: the
    /// rebuild keys on `effective` (the concrete resolved language), not on `selected`.
    ///
    /// That is also why a language surface must ALSO call this at its own refresh points (window
    /// activation, and so on). The observation only fires on writes to the persisted tag, and an OS
    /// language change under Automatic writes nothing — so nothing fires.
    func syncFromSettings() {
        apply(selected: DisplayLanguage.fromTag(settings.displayLanguage))
    }

    /// Both assignments are guarded: `setLanguage` writes the defaults key and applies the change
    /// itself, and that write also feeds the observation, so every explicit pick arrives here twice.
    /// Under `@Observable` an assignment invalidates readers whether or not the value changed, so
    /// writing unconditionally would recompute every localized view a second time for nothing.
    private func apply(selected newSelected: DisplayLanguage) {
        let newEffective = newSelected.effectiveLanguage(deviceLanguageSubtag())
        if selected != newSelected {
            selected = newSelected
        }
        if resolver.language != newEffective {
            resolver = StringResolver(newEffective)
        }
    }

    /// The device's preferred-language subtag (lowercased ISO 639), or `""` when unavailable. Reads
    /// `Locale.preferredLanguages.first` — the user's OS language-preference order, NOT `Locale.current`,
    /// which a per-app override can mask.
    static let systemLanguageSubtag: @Sendable () -> String = {
        guard let preferred = Locale.preferredLanguages.first,
              let code = Locale(identifier: preferred).language.languageCode?.identifier
        else { return "" }
        return code.lowercased()
    }
}
