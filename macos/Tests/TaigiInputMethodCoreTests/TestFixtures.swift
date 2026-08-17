// Shared test support: repository paths, generation isolation, one lexicon
// install, and the doubles and factories every composing suite needs.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// `#filePath` is confined to this file. Production code resolves its data
/// relative to the running bundle; only the tests, which run outside any bundle,
/// need to know where the repository keeps its copies.
enum TestFixtures {
    /// `<repo>/ios/Resources/Dictionaries` — the same directory
    /// `bundle-app.sh` copies into the assembled `.app`.
    static let dictionaryDirectory = repositoryRoot
        .appendingPathComponent("ios/Resources/Dictionaries")

    /// One counter for the whole test process, so no two suites can hand the
    /// engine the same generation — spacing per-suite counters apart by hand
    /// only works until a suite grows past the gap.
    static let generationCounter = GenerationCounter(startingAt: 1000)

    /// A key-down event carrying `characters`. The ten-argument AppKit
    /// initializer lives here once; every suite that needs a key event is
    /// otherwise a copy of it.
    ///
    /// `charactersIgnoringModifiers` defaults to `characters` because the two
    /// only differ for a chord — which is exactly what a case passing it
    /// separately is testing: Control rewrites the digits it is held with, so
    /// `⌃3` really does arrive as an Escape in `characters`.
    static func keyDownEvent(
        characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        charactersIgnoringModifiers: String? = nil,
    ) throws -> NSEvent {
        try XCTUnwrap(NSEvent.keyEvent(
            with: .keyDown,
            location: .zero,
            modifierFlags: modifiers,
            timestamp: 0,
            windowNumber: 0,
            context: nil,
            characters: characters,
            charactersIgnoringModifiers: charactersIgnoringModifiers ?? characters,
            isARepeat: false,
            keyCode: 0,
        ))
    }

    /// `client: nil`: IMK rejects anything but a real client proxy here, so a
    /// test double reaches the controller through the callbacks instead — which
    /// is also how the controller learns its client in production.
    static func makeInputController() throws -> TaigiInputController {
        try XCTUnwrap(
            TaigiInputController(server: nil, delegate: nil, client: nil),
            "could not construct the controller under test",
        )
    }

    /// The shipped defaults with the output and learning flags overridable —
    /// the only settings any case here varies.
    static func settings(
        inputMode: InputMode = .tl,
        swapped: Bool = false,
        bothScripts: Bool = false,
        frequencyRecording: Bool = true,
        associationRecording: Bool = true,
        customDict: Bool = true,
        dictionarySources: DictionarySourceToggles = .defaults,
    ) -> EngineSettings {
        EngineSettings(
            inputMode: inputMode,
            isTranslateSwapped: swapped,
            isOutputBothScripts: bothScripts,
            isLiteralRomanCandidateEnabled: false,
            isFrequencyRecordingEnabled: frequencyRecording,
            isAssociationRecordingEnabled: associationRecording,
            isCustomDictEnabled: customDict,
            dictionarySources: dictionarySources,
        )
    }

    /// An empty directory nothing else in the process is using. Each call gets
    /// its own, so a case that learns something cannot change what the next case
    /// starts from.
    static func scratchDirectory() throws -> URL {
        try UserDataDirectory.created(
            URL(fileURLWithPath: NSTemporaryDirectory())
                .appendingPathComponent("TaigiLearning-\(UUID().uuidString)"),
        )
    }

    /// The user-data stores over a scratch directory, open and ready.
    ///
    /// Real SQLite rather than a double: the whole of what these types do is
    /// SQL, and a double would only prove that the fake behaves like the fake.
    static func makeUserDataStores() throws -> UserDataStores {
        let directory = try scratchDirectory()
        let stores = UserDataStores(directory: { directory })
        stores.open()
        waitUntilReady(stores)
        return stores
    }

    /// Spins the run loop until every store has opened. Opening is
    /// asynchronous by design — a keystroke must never wait on it — so a case
    /// that asserts on stored rows has to wait here instead.
    static func waitUntilReady(
        _ stores: UserDataStores,
        timeout: TimeInterval = 5,
        file: StaticString = #filePath,
        line: UInt = #line,
    ) {
        let opened = spinRunLoop(
            until: {
                stores.frequency.isReady && stores.association.isReady
                    && stores.customDictionary.isReady
            },
            timeout: timeout,
        )
        if !opened {
            XCTFail("the learning stores did not open within \(timeout)s", file: file, line: line)
        }
    }

    /// A display-language store pinned to `language`, over the caller's own defaults suite.
    ///
    /// Every case that asserts on localized chrome needs one: the shared store reads the machine's
    /// real settings, and the device subtag is pinned too so a `.system` case cannot inherit the
    /// language of whoever is running the tests.
    @MainActor
    static func makeDisplayLanguageStore(
        _ language: DisplayLanguage,
        userDefaults: UserDefaults,
        deviceSubtag: String = "zh",
    ) -> DisplayLanguageStore {
        userDefaults.set(language.tag, forKey: SettingsStore.Keys.displayLanguage.name)
        return DisplayLanguageStore(
            settings: SettingsStore(userDefaults: userDefaults),
            deviceLanguageSubtag: { deviceSubtag },
        )
    }

    /// Runs the current run loop until `condition` holds, and answers whether
    /// it did before `timeout`.
    ///
    /// The learning stores do their work on their own queue and answer through
    /// state a test can only poll, so every case that asserts on what they
    /// stored needs this shape — once, here, rather than once per suite.
    static func spinRunLoop(
        until condition: () -> Bool,
        timeout: TimeInterval = 5,
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while !condition() {
            guard Date() < deadline else { return false }
            RunLoop.current.run(until: Date().addingTimeInterval(0.01))
        }
        return true
    }

    /// A manager wired to scratch stores unless a case supplies its own.
    ///
    /// The production initializer takes no defaults on purpose — the shipped
    /// stores write to the user's home directory. Defaulting HERE is the
    /// opposite hazard and the safe one: a case that forgets to say where its
    /// learning goes gets a directory that is thrown away, never the user's.
    @MainActor
    static func makeComposingManager(
        settingsProvider: EngineSettingsProvider = StubEngineSettingsProvider(),
        stores: UserDataStores? = nil,
        startingGeneration: UInt64,
    ) throws -> ComposingManager {
        let stores = try stores ?? makeUserDataStores()
        return ComposingManager(
            settingsProvider: settingsProvider,
            frequencyStore: stores.frequency,
            customDictionaryStore: stores.customDictionary,
            nextWordLearner: NextWordLearner(store: stores.association),
            startingGeneration: startingGeneration,
        )
    }

    /// A coordinator of its own, over scratch stores and an unused generation.
    ///
    /// Never `ComposingSessionCoordinator.shared`: that one is process-wide,
    /// guards process-wide engine state, and in the shipped app arms the real
    /// Carbon hotkeys through `AppDelegate`'s availability callback.
    @MainActor
    static func makeCoordinator() throws -> ComposingSessionCoordinator {
        let stores = try makeUserDataStores()
        return try ComposingSessionCoordinator(
            composingManager: makeComposingManager(
                stores: stores,
                startingGeneration: generationCounter.next(),
            ),
            learningStores: stores,
        )
    }

    /// A candidate carrying only the fields a case is asserting on. The engine
    /// fills ten, and a suite about navigation or rendering should not have to
    /// name the eight it does not care about.
    static func candidate(
        roman: String = "tai",
        hanji: String? = nil,
        displayText: String = "",
        canonicalTl: String = "",
        consumedSpanEnd: UInt32 = 0,
        syllableCount: UInt32 = 1,
    ) -> ContinuousCandidate {
        ContinuousCandidate(
            consumedSpanStart: 0,
            consumedSpanEnd: consumedSpanEnd,
            syllableCount: syllableCount,
            displayText: displayText,
            score: 0,
            form: 0,
            mode: .unspecified,
            roman: roman,
            hanji: hanji,
            canonicalTl: canonicalTl,
        )
    }

    /// From `<repo>/macos/Tests/TaigiInputMethodCoreTests/TestFixtures.swift`.
    private static let repositoryRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent() // TaigiInputMethodCoreTests
        .deletingLastPathComponent() // Tests
        .deletingLastPathComponent() // macos
        .deletingLastPathComponent() // <repo>
}

extension DictionarySourceToggles {
    /// Every dictionary switched off — the state the wire cannot say with a
    /// `0`, and so the one both the filter suite and the search suite are
    /// about.
    ///
    /// Spelled out rather than derived from `.defaults`, because a source added
    /// to the struct must fail to compile here until someone has said which
    /// side of "off" it belongs on.
    static let allSourcesOff = DictionarySourceToggles(
        kautian: false,
        taigitv: false,
        itaigi: false,
        sitbut: false,
        taihoa: false,
        taijit: false,
        kungge: false,
        stti: false,
        khpoo: false,
        variant: false,
        khiin: false,
        lkk: false,
        dev: false,
        kautianSubcollections: .defaults,
    )
}

/// Hands out a generation nobody else is using.
///
/// The Rust composing state is one per process, and the engine drops that state
/// whenever the generation it is handed differs from the last one it saw
/// (`engine/composing/src/handle.rs:61-66`). A case that runs under its own
/// generation therefore starts from an idle engine no matter what ran before it.
final class GenerationCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var value: UInt64

    init(startingAt value: UInt64) {
        self.value = value
    }

    func next() -> UInt64 {
        lock.lock()
        defer { lock.unlock() }
        value += 1
        return value
    }
}

extension [ComposingTransition.Effect] {
    /// The texts the engine asked to be written to the document, in order.
    /// Assertions want the content, and hand-rolling the pattern match at every
    /// call site buries what each case is actually checking.
    var committedTexts: [String] {
        compactMap { effect in
            if case let .commitTextReplacingPreedit(text) = effect {
                return text
            }
            return nil
        }
    }

    /// The compositions the engine asked to be shown, in order — the marked
    /// region's contents over time.
    var preeditTexts: [String] {
        compactMap { effect in
            if case let .updatePreedit(text) = effect {
                return text
            }
            return nil
        }
    }
}

/// Serves fixed settings, so a case can drive the manager under an output mode
/// the shipped defaults do not use. The `UserDefaults`-backed provider is PR5's;
/// until it exists this is the only way to reach the other three renderings.
final class StubEngineSettingsProvider: EngineSettingsProvider {
    let current: EngineSettings

    init(
        inputMode: InputMode = .tl,
        swapped: Bool = false,
        bothScripts: Bool = false,
        frequencyRecording: Bool = true,
        associationRecording: Bool = true,
        customDict: Bool = true,
        dictionarySources: DictionarySourceToggles = .defaults,
    ) {
        current = TestFixtures.settings(
            inputMode: inputMode,
            swapped: swapped,
            bothScripts: bothScripts,
            frequencyRecording: frequencyRecording,
            associationRecording: associationRecording,
            customDict: customDict,
            dictionarySources: dictionarySources,
        )
    }
}

/// Collects effects instead of performing them, so a case can assert on what
/// the host was told without a client.
@MainActor
final class RecordingEffectExecutor: ComposingEffectExecutor {
    private(set) var effects: [ComposingTransition.Effect] = []

    var committedTexts: [String] {
        effects.committedTexts
    }

    func execute(_ effect: ComposingTransition.Effect) {
        effects.append(effect)
    }

    func clearEffects() {
        effects.removeAll()
    }
}

/// Records what the controller asked of the candidate bar instead of opening a
/// window, so the routing between keys, the list model and the panel's ownership
/// rules can be asserted without a screen.
@MainActor
final class RecordingCandidatePresenter: CandidatePresenter {
    enum Call: Equatable {
        case show(CandidateBarContent, caretRect: CGRect)
        case hide(isOwner: Bool)
        case hideForHandover
    }

    private(set) var calls: [Call] = []
    /// Nil once nothing is showing, mirroring the real panel's ownership so a
    /// case can pin the handover rule end to end.
    private(set) var owner: ComposingSessionToken?

    /// What is on screen right now, and nil once the bar has been hidden —
    /// returning the last content shown regardless would let a case assert on
    /// candidates the user can no longer see.
    var shownContent: CandidateBarContent? {
        guard isShowing else { return nil }
        return calls.reversed().compactMap { call in
            if case let .show(content, _) = call {
                return content
            }
            return nil
        }.first
    }

    var isShowing: Bool {
        owner != nil
    }

    func show(
        _ content: CandidateBarContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel _: CGWindowLevel,
        ownedBy owner: ComposingSessionToken,
    ) {
        self.owner = owner
        calls.append(.show(content, caretRect: caretRect))
    }

    func hide(ownedBy owner: ComposingSessionToken) {
        let isOwner = self.owner == owner
        calls.append(.hide(isOwner: isOwner))
        if isOwner {
            self.owner = nil
        }
    }

    func hideForHandover() {
        calls.append(.hideForHandover)
        owner = nil
    }
}

/// Installs the dictionary once for the whole test process, because the engine
/// holds it process-wide and re-installing per case would re-map ~24MB of data
/// for no gain.
enum InstalledLexicon {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var stats: LexiconInstallStats?

    @discardableResult
    static func installOnce() -> LexiconInstallStats? {
        lock.lock()
        defer { lock.unlock() }
        if let stats {
            return stats
        }
        guard let artifacts = try? DictionaryArtifacts(baseURL: TestFixtures.dictionaryDirectory) else {
            return nil
        }
        stats = RustEngineBridge.lexiconInstall(artifacts: artifacts, dictionaryVersion: 1)
        return stats
    }
}
