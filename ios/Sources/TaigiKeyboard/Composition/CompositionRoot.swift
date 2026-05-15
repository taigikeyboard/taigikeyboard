// 中文: 主 App 與鍵盤擴充共用的服務組裝點 (Composition Root)。
// 中文: 順序固定:repositories → leaf services → composite services。
// 中文: lexicon engine 安裝由啟動點各自負責,這裡不持有引擎狀態。

import Foundation

/// Production service graph. Wiring order matches constructor dependencies:
/// repositories → leaf services → composite services.
/// `SharedSettings.shared` is referenced directly by init defaults.
///
/// The Rust shared-core lexicon engine owns the trie + binary readers; install
/// happens once at process startup via `RustEngineBridge.lexiconInstall(...)`
/// from `TaigiKeyboardApp.installLexiconEngineForMainApp()` and
/// `KeyboardViewController.installLexiconEngine()`.
// 中文: 服務相依圖的組裝點,所有單例服務都從這裡取出。
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
