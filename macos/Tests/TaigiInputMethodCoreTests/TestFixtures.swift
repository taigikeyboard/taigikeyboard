// Shared test support: repository paths, generation isolation, one lexicon
// install, and the doubles and factories every composing suite needs.

import AppKit
import XCTest

@testable import TaigiInputMethodCore

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
    static func keyDownEvent(
        characters: String,
        modifiers: NSEvent.ModifierFlags = [],
    ) throws -> NSEvent {
        try XCTUnwrap(NSEvent.keyEvent(
            with: .keyDown,
            location: .zero,
            modifierFlags: modifiers,
            timestamp: 0,
            windowNumber: 0,
            context: nil,
            characters: characters,
            charactersIgnoringModifiers: characters,
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
            if case let .commitTextReplacingPreedit(text) = effect { return text }
            return nil
        }
    }
}

/// Collects effects instead of performing them, so a case can assert on what
/// the host was told without a client.
@MainActor
final class RecordingEffectExecutor: ComposingEffectExecutor {
    private(set) var effects: [ComposingTransition.Effect] = []

    var committedTexts: [String] { effects.committedTexts }

    func execute(_ effect: ComposingTransition.Effect) {
        effects.append(effect)
    }

    func clearEffects() {
        effects.removeAll()
    }
}

/// Installs the dictionary once for the whole test process, because the engine
/// holds it process-wide and re-installing per case would re-map ~24MB of data
/// for no gain.
enum InstalledLexicon {
    private static let lock = NSLock()
    private static nonisolated(unsafe) var stats: LexiconInstallStats?

    @discardableResult
    static func installOnce() -> LexiconInstallStats? {
        lock.lock()
        defer { lock.unlock() }
        if let stats { return stats }
        guard let artifacts = try? DictionaryArtifacts(baseURL: TestFixtures.dictionaryDirectory) else {
            return nil
        }
        stats = RustEngineBridge.lexiconInstall(artifacts: artifacts, dictionaryVersion: 1)
        return stats
    }
}
