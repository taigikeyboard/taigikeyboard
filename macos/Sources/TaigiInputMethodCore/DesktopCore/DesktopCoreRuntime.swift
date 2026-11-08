// The process's desktop-core runtime, driven over the shell seam.

import Foundation

/// The `taigi-desktop-core` runtime this process runs on
/// (docs/architecture/macos-desktop-core-roadmap.md D3, D5): `configure`
/// builds it once per process, and every request after that carries the
/// key-path settings as `UserDefaults` holds them at that moment.
///
/// The settings are rebuilt for each request rather than tracked: nothing can
/// go stale between a write and the request that follows it — a mode a global
/// action just switched included — the same reason `SettingsStore` caches
/// nothing. Which settings, and their defaults, is the core's answer to
/// `configure`.
final class DesktopCoreRuntime {
    /// What `Configure` carries.
    struct Configuration {
        /// Where the user data lives; `nil` opens no stores.
        var dataDirectory: URL?
        /// The directory holding the four dictionary files; `nil` installs
        /// no lexicon.
        var dictionariesDirectory: URL?
        /// The stamp the lexicon is installed under.
        var dictionaryStamp: UInt32
        /// The machine's UI language as of launch.
        var systemLocale: String
    }

    /// The runtime `configure` built — one per process, as the core
    /// configures once. `nil` before that, or when the core refused.
    /// Unguarded: written once, by the launch's (or a test process's first)
    /// `configure`, before the input-method server exists to read it.
    private(set) nonisolated(unsafe) static var configured: DesktopCoreRuntime?

    /// The whitelisted settings and their defaults, as the core described
    /// them.
    let settings: [Taigi_DesktopShell_SettingDescriptor]
    private let userDefaults: UserDefaults

    private init(settings: [Taigi_DesktopShell_SettingDescriptor], userDefaults: UserDefaults) {
        self.settings = settings
        self.userDefaults = userDefaults
    }

    /// Builds the process's one runtime; `nil` when the core refused (it
    /// was already configured) or the seam failed — logged by the bridge.
    static func configure(
        _ configuration: Configuration,
        userDefaults: UserDefaults = .standard,
    ) -> DesktopCoreRuntime? {
        var request = Taigi_DesktopShell_ConfigureRequest()
        if let dataDirectory = configuration.dataDirectory {
            request.dataDirectory = dataDirectory.path
        }
        if let dictionariesDirectory = configuration.dictionariesDirectory {
            request.dictionariesDirectory = dictionariesDirectory.path
        }
        request.dictionaryStamp = configuration.dictionaryStamp
        request.systemLocale = configuration.systemLocale
        let reply = DesktopCoreBridge.roundtrip(.configure(request), op: "configure") {
            if case let .configure(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        let runtime = reply.map { DesktopCoreRuntime(settings: $0.settings, userDefaults: userDefaults) }
        if let runtime {
            configured = runtime
        }
        return runtime
    }

    /// Installs the lexicon and opens the user data, once per process; a
    /// repeat answers the first result. Failures are the core's to log and
    /// leave alone: an engine with no lexicon types romanization and
    /// suggests nothing, which beats no input method at all.
    @discardableResult
    func prepare() -> Taigi_DesktopShell_PrepareReply? {
        send(.prepare(Taigi_DesktopShell_PrepareRequest()), op: "prepare") {
            if case let .prepare(reply) = $0 {
                reply
            } else {
                nil
            }
        }
    }

    /// Every request after `configure` goes through here, carrying the
    /// settings snapshot. A snapshot the core refuses refuses the request
    /// with it — logged by the bridge, answered `nil` — and the core keeps
    /// the last snapshot it accepted. The snapshot is built with the
    /// readers' own expressions (`storedValue`), so it is refused only when
    /// the two sides' whitelists disagree.
    private func send<Reply>(
        _ request: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        op: String,
        expected: (Taigi_DesktopShell_DesktopResponse.OneOf_Reply) -> Reply?,
    ) -> Reply? {
        DesktopCoreBridge.roundtrip(
            request,
            settings: settingsSnapshot(in: userDefaults),
            op: op,
            expected: expected,
        )
    }

    /// The whitelisted settings as `userDefaults` holds them now — what a
    /// request carries.
    func settingsSnapshot(in userDefaults: UserDefaults) -> Taigi_DesktopShell_SettingsSnapshot {
        Self.settingsSnapshot(settings, in: userDefaults)
    }

    /// The configured runtime's snapshot of `userDefaults`, for a request
    /// that must carry one (the key rules, the dictionary search); nil —
    /// logged under `op` — before the runtime exists.
    static func snapshot(in userDefaults: UserDefaults, op: String) -> Taigi_DesktopShell_SettingsSnapshot? {
        guard let runtime = configured else {
            DebugLogger(category: "DesktopCoreRuntime").error("[\(op)] no desktop-core runtime — no settings to read under")
            return nil
        }
        return runtime.settingsSnapshot(in: userDefaults)
    }

    /// The whitelisted settings `userDefaults` holds now; a setting it does
    /// not hold, or holds as something its reader would not accept, is left
    /// out and reads as the core's default.
    static func settingsSnapshot(
        _ settings: [Taigi_DesktopShell_SettingDescriptor],
        in userDefaults: UserDefaults,
    ) -> Taigi_DesktopShell_SettingsSnapshot {
        var snapshot = Taigi_DesktopShell_SettingsSnapshot()
        snapshot.entries = settings.compactMap { descriptor in
            storedValue(of: descriptor, in: userDefaults).map { value in
                var entry = Taigi_DesktopShell_SettingEntry()
                entry.name = descriptor.name
                entry.value = value
                return entry
            }
        }
        return snapshot
    }

    /// Read with the expression `SettingsStore` reads its own kind with, so
    /// the core sees what the Swift reader sees: `object(forKey:) as? Bool`
    /// for a boolean — a stored 0 or 1 number reads as one (inventory S26) —
    /// and `string(forKey:)` for text.
    private static func storedValue(
        of descriptor: Taigi_DesktopShell_SettingDescriptor,
        in userDefaults: UserDefaults,
    ) -> Taigi_DesktopShell_SettingValue? {
        var value = Taigi_DesktopShell_SettingValue()
        switch descriptor.kind {
        case .boolean:
            guard let stored = userDefaults.object(forKey: descriptor.name) as? Bool else { return nil }
            value.boolean = stored
        case .text:
            guard let stored = userDefaults.string(forKey: descriptor.name) else { return nil }
            value.text = stored
        case .unspecified, .UNRECOGNIZED:
            return nil
        }
        return value
    }
}

extension DesktopCoreRuntime.Configuration {
    /// The shipped app's: the user-data directory, the bundle's resources,
    /// `CFBundleVersion` as the stamp, the first preferred language. A
    /// directory that cannot be resolved is logged and left out, which
    /// skips only what needs it.
    static func launch(bundle: Bundle) -> Self {
        let logger = DebugLogger(category: "Bootstrap")
        var dataDirectory: URL?
        do {
            dataDirectory = try UserDataDirectory.standard()
        } catch {
            logger.error("user data directory unavailable: \(error)")
        }
        if bundle.resourceURL == nil {
            logger.error("bundle has no resource directory — lexicon not installed")
        }
        return Self(
            dataDirectory: dataDirectory,
            dictionariesDirectory: bundle.resourceURL,
            dictionaryStamp: dictionaryStamp(infoDictionary: bundle.infoDictionary),
            systemLocale: Locale.preferredLanguages.first ?? "",
        )
    }

    /// The bundle version doubles as the dictionary stamp: the data is
    /// rebuilt and re-bundled by the same release that bumps it. `1` when
    /// it is absent or not a number.
    static func dictionaryStamp(infoDictionary: [String: Any]?) -> UInt32 {
        (infoDictionary?["CFBundleVersion"] as? String).flatMap(UInt32.init) ?? 1
    }
}
