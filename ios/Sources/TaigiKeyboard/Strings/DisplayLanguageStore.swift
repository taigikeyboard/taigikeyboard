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

    var language: DisplayLanguage {
        didSet {
            guard language != oldValue else { return }
            resolver = StringResolver(language)
        }
    }

    init(language: DisplayLanguage) {
        self.language = language
        resolver = StringResolver(language)
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

    /// Persists `language` and updates the live state. Persisting writes the App-Group value, so the
    /// extension (separate process) picks it up via its settings-change observer.
    func setLanguage(_ language: DisplayLanguage) {
        SharedSettings.shared.displayLanguage = language.tag
        self.language = language
    }

    /// Re-reads the persisted tag into the live state. The extension calls this when a language surface
    /// appears (overlay `.onAppear`) so a host-side language change reflects without a keyboard restart.
    /// Idempotent — `language`'s `didSet` guards on equality, so an unchanged tag rebuilds nothing.
    func syncFromSettings() {
        language = DisplayLanguage.fromTag(SharedSettings.shared.displayLanguage)
    }
}
