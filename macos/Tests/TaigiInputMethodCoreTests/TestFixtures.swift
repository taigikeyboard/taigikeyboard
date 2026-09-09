// Shared test support: repository paths, generation isolation, one lexicon
// install, and the doubles and factories every composing suite needs.

import AppKit
import CoreText
@testable import TaigiInputMethodCore
import XCTest

/// `#filePath` is confined to this file. Production code resolves its data
/// relative to the running bundle; only the tests, which run outside any bundle,
/// need to know where the repository keeps its copies.
enum TestFixtures {
    /// `<repo>/dictionaries` — the shared artifact directory `bundle-app.sh`
    /// copies into the assembled `.app`.
    static let dictionaryDirectory = repositoryRoot
        .appendingPathComponent("dictionaries")

    /// One counter for the whole test process, so no two suites can hand the
    /// engine the same generation — spacing per-suite counters apart by hand
    /// only works until a suite grows past the gap.
    static let generationCounter = GenerationCounter(startingAt: 1000)

    /// The candidate-window metrics a fresh install renders at — what any
    /// suite exercising a cell or its measurement should use, unless the case
    /// is specifically about a non-default size.
    static let defaultCandidateMetrics = CandidateMetrics(
        textSize: SettingsStore.Keys.candidateTextSize.defaultValue,
        windowSize: SettingsStore.Keys.candidateWindowSize.defaultValue,
        fontSelection: .builtIn(SettingsStore.Keys.fontType.defaultValue),
    )

    /// One panel of each layout, at the default metrics — what a suite
    /// asserting a property EVERY layout must hold walks over.
    @MainActor
    static func candidatePanels(style: CandidateWindowStyle = .sequoia) -> [CandidateBasePanel] {
        [
            HorizontalCandidatePanel(style: style, metrics: defaultCandidateMetrics),
            VerticalCandidatePanel(style: style, metrics: defaultCandidateMetrics),
            ExpandableCandidatePanel(style: style, metrics: defaultCandidateMetrics),
        ]
    }

    /// A panel's laid-out cells, in reading order: left to right, top to
    /// bottom. The panels keep their own cell arrays private, so a suite that
    /// asserts about what is DRAWN walks the view tree.
    ///
    /// Read in each cell's own superview, whose flippedness says which way its
    /// rows run — the row containers are flipped (row 0 at `y == 0`) while the
    /// window's content view is not.
    @MainActor
    static func candidateCells(in panel: CandidateBasePanel) -> [CandidateItemView] {
        func collect(_ view: NSView) -> [CandidateItemView] {
            if let item = view as? CandidateItemView {
                return [item]
            }
            return view.subviews.flatMap(collect)
        }
        func topDownY(_ item: CandidateItemView) -> CGFloat {
            item.superview?.isFlipped == false ? -item.frame.origin.y : item.frame.origin.y
        }
        guard let root = panel.contentView else { return [] }
        return collect(root)
            .filter { !$0.isHidden }
            .sorted {
                topDownY($0) == topDownY($1)
                    ? $0.frame.origin.x < $1.frame.origin.x
                    : topDownY($0) < topDownY($1)
            }
    }

    /// `<repo>/fonts/font` — the shared typeface directory every platform
    /// packages from, and the one `bundle-app.sh` copies into the assembled
    /// `.app`'s `ATSApplicationFontsPath`.
    static let fontDirectory = repositoryRoot.appendingPathComponent("fonts/font")

    /// Activates the bundled typefaces for this process, and answers which
    /// files failed to.
    ///
    /// The tests run outside any bundle, so Info.plist's
    /// `ATSApplicationFontsPath` — how the shipped app activates these — does
    /// nothing here. Without this, every `NSFont(name:)` for a bundled face
    /// returns nil and every font assertion would pass vacuously against the
    /// system-font fallback.
    ///
    /// Registers the whole directory, which is what `bundle-app.sh` copies —
    /// so a case whose file stopped shipping fails as an unresolvable
    /// PostScript name, in the suite that is about names.
    ///
    /// Registered once for the whole process: Core Text reports a second
    /// registration of the same file as an error, and a suite that ran after
    /// another would see it.
    static let unregisterableFontFiles: [String] = {
        let files = (try? FileManager.default.contentsOfDirectory(
            at: fontDirectory, includingPropertiesForKeys: nil,
        )) ?? []
        return files
            .filter { ["ttf", "otf"].contains($0.pathExtension) }
            .filter { !CTFontManagerRegisterFontsForURL($0 as CFURL, .process, nil) }
            .map(\.lastPathComponent)
    }()

    /// The width of `text` at `size` in the face a fresh install renders in —
    /// what every candidate-measurement expectation is traced against, and
    /// resolved through the same default `defaultCandidateMetrics` uses so the
    /// oracle follows the default rather than pinning today's value of it.
    @MainActor
    static func defaultFontWidth(of text: String, size: CGFloat) -> CGFloat {
        let font = SettingsStore.Keys.fontType.defaultValue.font(ofSize: size)
        return (text as NSString).size(withAttributes: [.font: font]).width
    }

    /// A key-down event carrying `characters`. The ten-argument AppKit
    /// initializer lives here once; every suite that needs a key event is
    /// otherwise a copy of it.
    ///
    /// `charactersIgnoringModifiers` defaults to `characters` because the two
    /// only differ for a chord — which is exactly what a case passing it
    /// separately is testing: Control rewrites the digits it is held with, so
    /// `⌃3` really does arrive as an Escape in `characters`.
    ///
    /// `keyCode` is the hardware key (`KeyEventSnapshot.keyCode`), read only
    /// by the recorder's refusal of a shifted number-row key; `0` — the `a`
    /// key — for everything else, since a chord is identified by the
    /// character its key types.
    static func keyDownEvent(
        characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        charactersIgnoringModifiers: String? = nil,
        keyCode: UInt16 = 0,
        isARepeat: Bool = false,
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
            isARepeat: isARepeat,
            keyCode: keyCode,
        ))
    }

    /// A key-down for one of the six navigation keys: the function-key scalar
    /// under the `.function` flag, which is what AppKit names as the arrow.
    static func arrowKeyDownEvent(
        _ key: NavigationKey,
        modifiers: NSEvent.ModifierFlags = [],
    ) throws -> NSEvent {
        let functionKey: Int = switch key {
        case .leftArrow: NSLeftArrowFunctionKey
        case .rightArrow: NSRightArrowFunctionKey
        case .upArrow: NSUpArrowFunctionKey
        case .downArrow: NSDownArrowFunctionKey
        case .pageUp: NSPageUpFunctionKey
        case .pageDown: NSPageDownFunctionKey
        }
        return try keyDownEvent(
            characters: String(UnicodeScalar(functionKey)!),
            modifiers: modifiers.union(.function),
        )
    }

    /// `<repo>/symbols/desktop-symbols.json` — the symbol picker's table,
    /// which `bundle-app.sh` copies into the assembled `.app`. The tests run
    /// outside any bundle, so a controller case injects this copy.
    static let symbolTableURL = repositoryRoot
        .appendingPathComponent("symbols")
        .appendingPathComponent(SymbolTable.fileName)

    /// The shipped symbol table, read the way the app reads it.
    static func shippedSymbolTable() throws -> SymbolTable {
        try SymbolTable.load(from: symbolTableURL)
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
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        frequencyRecording: Bool = true,
        associationRecording: Bool = true,
        customDict: Bool = true,
        dictionarySources: DictionarySourceToggles = .defaults,
    ) -> EngineSettings {
        EngineSettings(
            inputMode: inputMode,
            isTranslateSwapped: swapped,
            isOutputBothScripts: bothScripts,
            candidateDisplayMode: candidateDisplayMode,
            // §34/S22 ships ON; a case that wants it off writes the real
            // setting with `withSetting`, which is the path production reads.
            isLiteralRomanCandidateEnabled: true,
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

    /// A chord no `ComposingAction` ships with, for a case that needs to record
    /// one without the binding resolver dropping it as a duplicate.
    ///
    /// Derived rather than written down: a default added to the roster would
    /// otherwise silently invalidate fixtures that have nothing to do with
    /// defaults.
    static func chordNoDefaultHolds(key: String = "\r") throws -> ComposingKeyChord {
        let taken = Set(ComposingAction.allCases.map(\.defaultChord))
        let candidates: [NSEvent.ModifierFlags] = [
            [.control, .option], [.command, .option], [.control, .command],
        ]
        for modifiers in candidates {
            let chord = try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
            if !taken.contains(chord) {
                return chord
            }
        }
        throw XCTSkip("every \(key) chord this fixture knows is a default now")
    }

    /// From `<repo>/macos/Tests/TaigiInputMethodCoreTests/TestFixtures.swift`.
    private static let repositoryRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent() // TaigiInputMethodCoreTests
        .deletingLastPathComponent() // Tests
        .deletingLastPathComponent() // macos
        .deletingLastPathComponent() // <repo>
}

extension XCTestCase {
    /// A settings store over its own throwaway defaults suite, torn down with
    /// the case — so a case that writes a setting cannot leak it into the
    /// machine's real domain or into the next case.
    @MainActor
    func makeScratchSettingsStore() throws -> SettingsStore {
        let suiteName = "ScratchSettings.\(UUID().uuidString)"
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        addTeardownBlock { userDefaults.removePersistentDomain(forName: suiteName) }
        return SettingsStore(userDefaults: userDefaults)
    }

    /// Runs `body` with `key` in `UserDefaults.standard` set to `value` (nil
    /// writes nothing), restored afterwards to whatever it held — including
    /// "held nothing", which a bare `removeObject` would turn into a value a
    /// later case never chose. `.standard` rather than a scratch suite because
    /// the controller and the shared coordinator's engine must read ONE domain
    /// for these cases to mean anything.
    @MainActor
    func withSetting(_ key: String, to value: Any?, _ body: () throws -> Void) rethrows {
        let saved = UserDefaults.standard.object(forKey: key)
        defer {
            if let saved {
                UserDefaults.standard.set(saved, forKey: key)
            } else {
                UserDefaults.standard.removeObject(forKey: key)
            }
        }
        if let value {
            UserDefaults.standard.set(value, forKey: key)
        }
        try body()
    }

    /// Clears `key` in `UserDefaults.standard` and puts it back at teardown —
    /// including "held nothing", which a bare `removeObject` would turn into a
    /// value a later case never chose. The teardown-scoped counterpart to
    /// `withSetting`, for cases that have to `await` and so cannot run inside
    /// its synchronous body.
    @MainActor
    func clearSettingRestoredAtTeardown(_ key: String) {
        let saved = UserDefaults.standard.object(forKey: key)
        addTeardownBlock {
            if let saved {
                UserDefaults.standard.set(saved, forKey: key)
            } else {
                UserDefaults.standard.removeObject(forKey: key)
            }
        }
        UserDefaults.standard.removeObject(forKey: key)
    }

    /// The 候選詞顯示 mode, written to the `.standard` domain the shared
    /// coordinator's settings provider reads — a 合用 case has to say so to
    /// the manager, not only to a controller's scratch store.
    @MainActor
    func withDisplayMode(_ mode: CandidateDisplayMode, _ body: () throws -> Void) rethrows {
        try withSetting(SettingsStore.Keys.candidateDisplayMode.name, to: mode.rawValue, body)
    }

    /// Runs `body` with 漢字優先 on or off.
    ///
    /// Through `withSetting` rather than a scratch store, for the reason
    /// spelled out there: the controller reads its own `SettingsStore` while
    /// the shared coordinator's `ComposingManager` reads another, so a swap
    /// written to a scratch suite renders the bar one way and gates the commit
    /// the other. A case that means "the user is in 漢字 mode" has to move the
    /// domain BOTH of them read.
    @MainActor
    func withTranslateSwapped(_ swapped: Bool, _ body: () throws -> Void) rethrows {
        try withSetting(SettingsStore.Keys.isTranslateSwapped.name, to: swapped, body)
    }
}

/// A case's session with a candidate bar up — what the bar-walking helpers
/// below need of it. Each test file keeps its own `Session` and conforms.
@MainActor
protocol CandidateBarSession {
    var controller: TaigiInputController { get }
    var client: RecordingTextInputClient { get }
    var presenter: RecordingCandidatePresenter { get }
}

extension CandidateBarSession {
    /// Types `text` one character per key event, answering whether the last
    /// one was consumed.
    @discardableResult
    func type(_ text: String) throws -> Bool {
        var handled = false
        for character in text.map(String.init) {
            handled = try controller.handle(TestFixtures.keyDownEvent(characters: character), client: client)
        }
        return handled
    }

    /// One navigation key.
    @discardableResult
    func press(_ key: NavigationKey) throws -> Bool {
        try controller.handle(TestFixtures.arrowKeyDownEvent(key), client: client)
    }

    /// §34 opens the bar on the one-script literal — 顯示當咧拍的字 ships ON —
    /// so a case about a candidate that carries both scripts walks ⇥ onto the
    /// first one and hands it back.
    @discardableResult
    func walkToFirstTwoScriptCell() throws -> CandidateCellContent {
        let cells = try XCTUnwrap(presenter.shownContent).cells
        let index = try XCTUnwrap(cells.firstTwoScriptIndex, "taigi has hanji candidates")
        try walk(cells: index)
        return cells[index]
    }

    /// ⇥ `count` cells along a freshly opened bar.
    func walk(cells count: Int) throws {
        for _ in 0 ..< count {
            _ = try controller.handle(TestFixtures.keyDownEvent(characters: "\t"), client: client)
        }
        XCTAssertEqual(presenter.selectedIndex, count)
    }
}

extension [CandidateCellContent] {
    /// The first cell carrying both scripts — under 並排 the one an annotation
    /// sits on; the §34 literal ahead of it has one.
    var firstTwoScriptIndex: Int? {
        firstIndex { $0.annotation != nil }
    }

    var firstTwoScriptCell: CandidateCellContent? {
        firstTwoScriptIndex.map { self[$0] }
    }
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
            if case let .updatePreedit(text, _) = effect {
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
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        frequencyRecording: Bool = true,
        associationRecording: Bool = true,
        customDict: Bool = true,
        dictionarySources: DictionarySourceToggles = .defaults,
    ) {
        current = TestFixtures.settings(
            inputMode: inputMode,
            swapped: swapped,
            bothScripts: bothScripts,
            candidateDisplayMode: candidateDisplayMode,
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

/// Records what the controller asked of the candidate window instead of opening
/// one, so the routing between keys, the retained candidate array and the
/// window's ownership rules can be asserted without a screen.
///
/// The window is authoritative for the selection, so the double carries a
/// reference selection of its own: a flat fixed-nine page structure — the
/// measured packing the real window adds on top is pinned separately by
/// `HorizontalPageLayoutTests`, against the same clamp and page-start rules.
@MainActor
final class RecordingCandidatePresenter: CandidatePresenter {
    enum Call: Equatable {
        case show(CandidateWindowContent, caretRect: CGRect)
        case updateCells(CandidateWindowContent, isOwner: Bool)
        case navigate(CandidateNavigation)
        case hide(isOwner: Bool)
        case hideForHandover
    }

    private(set) var calls: [Call] = []
    /// Nil once nothing is showing, mirroring the real panel's ownership so a
    /// case can pin the handover rule end to end.
    private(set) var owner: ComposingSessionToken?

    private(set) var cells: [CandidateCellContent] = []
    private(set) var selectedIndex = 0

    /// What is on screen right now, and nil once the window has been hidden —
    /// returning the last content shown regardless would let a case assert on
    /// candidates the user can no longer see.
    var shownContent: CandidateWindowContent? {
        guard isShowing else { return nil }
        return CandidateWindowContent(
            cells: cells, slotKeySet: slotKeySet, leadCellIsUnkeyed: leadCellIsUnkeyed,
        )
    }

    /// The keys the window was last told pick — what a case asserting the
    /// hint matches the key contract reads.
    private(set) var slotKeySet: CandidateSlotKeySet = .bareKeys

    /// Whether the window was last told its first cell is the unkeyed §34
    /// literal — the double shifts its slots by one when it is, as the real
    /// panels do.
    private(set) var leadCellIsUnkeyed = false

    /// When set, `show` records the call but puts nothing on screen and takes
    /// no owner — what the real panel does for a caret on no display
    /// (`CandidatePanel.show`), so a case can pin what the controller does
    /// with a list that never reached the screen.
    var refusesToShow = false

    var isShowing: Bool {
        owner != nil
    }

    func show(
        _ content: CandidateWindowContent,
        anchoredTo caretRect: CGRect,
        hostWindowLevel _: CGWindowLevel,
        hostBundleIdentifier _: String?,
        ownedBy owner: ComposingSessionToken,
    ) {
        calls.append(.show(content, caretRect: caretRect))
        guard !refusesToShow else {
            hideForHandover()
            return
        }
        self.owner = owner
        cells = content.cells
        slotKeySet = content.slotKeySet
        leadCellIsUnkeyed = content.leadCellIsUnkeyed
        selectedIndex = 0
    }

    func updateCells(_ content: CandidateWindowContent, ownedBy owner: ComposingSessionToken) {
        calls.append(.updateCells(content, isOwner: self.owner == owner))
        // The real panel's contract: content changes in place, the window
        // stays up, and the selection keeps its absolute index (clamped).
        guard self.owner == owner, !cells.isEmpty, !content.cells.isEmpty else { return }
        slotKeySet = content.slotKeySet
        leadCellIsUnkeyed = content.leadCellIsUnkeyed
        cells = content.cells
        selectedIndex = min(selectedIndex, cells.count - 1)
    }

    func navigate(_ direction: CandidateNavigation, ownedBy owner: ComposingSessionToken) {
        calls.append(.navigate(direction))
        guard self.owner == owner, !cells.isEmpty else { return }
        // Only the walk is modelled, because only the walk means the same
        // thing in every real layout: one candidate along, clamped at both
        // ends. Where a PAGE direction lands depends on measured widths
        // (`HorizontalPageLayout`), so faking it here would let a controller
        // test pass against an algorithm production does not run — paging
        // cases assert the routing, and the geometry is pinned by
        // `HorizontalPageLayoutTests`.
        switch direction {
        case .left, .previousCandidate:
            selectedIndex = max(selectedIndex - 1, 0)
        case .right, .nextCandidate:
            selectedIndex = min(selectedIndex + 1, cells.count - 1)
        case .up, .down, .pageUp, .pageDown:
            break
        }
    }

    func selectedCandidateIndex(ownedBy owner: ComposingSessionToken) -> Int? {
        guard self.owner == owner, !cells.isEmpty else { return nil }
        return selectedIndex
    }

    /// Slots address the first page, which is where the selection stays in
    /// every controller case — the double never pages (see `navigate`). The
    /// unkeyed literal shifts them by one, as `CandidateBasePanel
    /// .candidateIndex(forKeySlot:)` does.
    func candidateIndex(forKeySlot slot: Int, ownedBy owner: ComposingSessionToken) -> Int? {
        guard self.owner == owner else { return nil }
        // Through the production rule, so the double cannot model a window the
        // real panels do not draw — including the shifted ninth key, which
        // falls off the modelled page rather than reaching a tenth cell.
        return CandidateIndexLabel.candidateIndex(
            forKeySlot: slot,
            leadCellIsUnkeyed: leadCellIsUnkeyed,
            indexForSlot: { slot in
                guard slot < HorizontalPageLayout.pageSize else { return nil }
                return cells.indices.contains(slot) ? slot : nil
            },
        )
    }

    func hide(ownedBy owner: ComposingSessionToken) {
        let isOwner = self.owner == owner
        calls.append(.hide(isOwner: isOwner))
        if isOwner {
            self.owner = nil
            cells = []
            selectedIndex = 0
        }
    }

    func hideForHandover() {
        calls.append(.hideForHandover)
        owner = nil
        cells = []
        selectedIndex = 0
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
