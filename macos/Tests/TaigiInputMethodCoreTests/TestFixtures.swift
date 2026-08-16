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

    /// The shipped defaults with the two output flags overridden — the only
    /// settings any case here varies, and the pair PR5 will put behind UI.
    static func settings(swapped: Bool = false, bothScripts: Bool = false) -> EngineSettings {
        EngineSettings(
            inputMode: .tl,
            isDoubleTapOOEnabled: true,
            isDoubleTapNNEnabled: true,
            isTranslateSwapped: swapped,
            isOutputBothScripts: bothScripts,
            isLiteralRomanCandidateEnabled: false,
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

    init(swapped: Bool = false, bothScripts: Bool = false) {
        current = TestFixtures.settings(swapped: swapped, bothScripts: bothScripts)
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
