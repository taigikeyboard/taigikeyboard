import Foundation

/// Production service graph, shared by the main app and the keyboard extension.
/// `SharedSettings.shared` is referenced directly by init defaults.
///
/// The Rust shared-core engine owns the lexicon (the trie + binary readers)
/// and the user's data (the four stores, `docs/architecture/user-data-engine-roadmap.md`
/// P7b); this type holds no engine state itself. The lexicon is installed
/// once at process startup via `RustEngineBridge.lexiconInstall(...)` from
/// `TaigiKeyboardApp.installLexiconEngineForMainApp()` and
/// `KeyboardViewController.installLexiconEngine()`; the user data is opened by
/// `UserDataOpening.open(in:)` from the same two places.
enum CompositionRoot {
    /// The user's data, through the engine's ops.
    static let userData: any UserDataClient = EngineUserDataClient()

    /// Where a pick is counted.
    static let usageRecorder: any UsageRecorder = EngineUsageRecorder()

    static let dictionarySearchService: DictionarySearchService = .init(
        userData: userData,
        settingsProvider: SharedSettings.shared,
    )
}
