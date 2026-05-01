import Foundation

/// Production service graph. Wiring order matches constructor dependencies:
/// repositories → leaf services → composite services.
/// `SharedSettings.shared` is referenced directly by init defaults.
///
/// The Rust shared-core lexicon engine owns the trie + binary readers; install
/// happens once at process startup via `RustEngineBridge.lexiconInstall(...)`
/// from `TaigiKeyboardApp.installLexiconEngineForMainApp()` and
/// `KeyboardViewController.installLexiconEngine()`.
enum CompositionRoot {
    // MARK: - Repositories

    static let userFrequencyRepository: UserFrequencyRepository = .init()
    static let customDictionaryRepository: CustomDictionaryRepository = .init()

    // MARK: - Leaf services

    static let userFrequencyService: UserFrequencyService = .init(
        repository: userFrequencyRepository,
    )
    static let customDictionaryService: CustomDictionaryService = .init(
        repository: customDictionaryRepository,
    )
    static let nextWordService: NextWordService = .init(
        settingsProvider: SharedSettings.shared,
    )

    // MARK: - Composite services

    static let lexiconService: LexiconService = .init(
        userFrequencyService: userFrequencyService,
        customDictionaryRepository: customDictionaryRepository,
        settingsProvider: SharedSettings.shared,
    )

    static let backupService: BackupService = .init(
        customDictionaryService: customDictionaryService,
        userFrequencyRepository: userFrequencyRepository,
        nextWordService: nextWordService,
    )

    static let dictionarySearchService: DictionarySearchService = .init(
        customDictionaryRepository: customDictionaryRepository,
        settingsProvider: SharedSettings.shared,
    )
}
