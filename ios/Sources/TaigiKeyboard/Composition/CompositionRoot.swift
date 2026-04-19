import Foundation

/// Production service graph. Wiring order matches constructor dependencies:
/// `TrieService` → repositories → leaf services → composite services.
/// `SharedSettings.shared` is referenced directly by init defaults.
enum CompositionRoot {
    // MARK: - Trie

    static let trieService: TrieService = .init(
        fileName: "dictionary",
        fileExtension: "trie",
        logCategory: "TrieService",
    )

    // MARK: - Repositories

    static let userFrequencyRepository: UserFrequencyRepository = .init()
    static let customDictionaryRepository: CustomDictionaryRepository = .init()
    static let dictionaryRepository: DictionaryRepository = .init(
        trieService: trieService,
        settingsProvider: SharedSettings.shared,
    )

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
        repository: dictionaryRepository,
        userFrequencyService: userFrequencyService,
        trieService: trieService,
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
        trieService: trieService,
        settingsProvider: SharedSettings.shared,
    )
}
