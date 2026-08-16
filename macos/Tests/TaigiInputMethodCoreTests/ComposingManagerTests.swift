// Drives the manager against the real engine, recording what the host is told.

@testable import TaigiInputMethodCore
import XCTest

/// The Rust composing state is one per process, so each case gets a generation
/// nobody else uses — see `GenerationCounter`.
@MainActor
final class ComposingManagerTests: XCTestCase {
    private func makeManager() throws -> ComposingManager {
        try TestFixtures.makeComposingManager(
            startingGeneration: TestFixtures.generationCounter.next(),
        )
    }

    func testAppend_showsThePreeditAndMirrorsTheEngine() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        manager.append("t", executing: executor)

        XCTAssertTrue(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "t")
        XCTAssertEqual(
            executor.effects.first,
            .updatePreedit("t"),
            "the host has to be shown the preedit before anything else",
        )
    }

    func testAppend_numericTone_showsTheDiacriticButKeepsTheTypedDigits() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()

        for character in ["t", "a", "i", "5"] {
            manager.append(character, executing: executor)
        }

        XCTAssertEqual(manager.rawInput, "tai5", "the raw buffer stays the engine's search key")
        XCTAssertEqual(
            manager.displayText,
            "t\u{00E2}i",
            "the mirror carries what the marked region renders, which is not what was typed",
        )
        XCTAssertTrue(
            executor.effects.contains(.updatePreedit("t\u{00E2}i")),
            "tone 5 must reach the user as the circumflex they read, not as the digit they typed",
        )
    }

    func testCommitComposition_writesTheCompositionAndEndsIt() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        for character in ["t", "a", "i"] {
            manager.append(character, executing: executor)
        }
        executor.clearEffects()

        manager.commitComposition(executing: executor)

        XCTAssertFalse(manager.isComposing)
        XCTAssertFalse(
            executor.committedTexts.isEmpty,
            "Return must put the composition into the document",
        )
    }

    func testCommitCompositionThenInsert_reachesTheHostAsOneWrite() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        for character in ["t", "a", "i"] {
            manager.append(character, executing: executor)
        }
        executor.clearEffects()

        manager.commitComposition(thenInsert: " ", executing: executor)

        let committed = executor.committedTexts
        XCTAssertEqual(committed.count, 1, "one keystroke must be one document mutation")
        XCTAssertEqual(
            committed.first?.hasSuffix(" "),
            true,
            "the space rides along with the commit instead of racing it",
        )
        XCTAssertFalse(manager.isComposing)
    }

    func testCancelComposition_clearsWithoutWritingToTheDocument() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        for character in ["t", "a", "i"] {
            manager.append(character, executing: executor)
        }
        executor.clearEffects()

        manager.cancelComposition(executing: executor)

        XCTAssertFalse(manager.isComposing)
        XCTAssertTrue(executor.effects.contains(.clearPreeditWithoutCommit))
        XCTAssertTrue(
            executor.committedTexts.isEmpty,
            "Escape must not write anything the user did not ask for",
        )
    }

    func testDeleteBackward_toEmpty_endsTheComposition() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        manager.append("t", executing: executor)

        manager.deleteBackward(executing: executor)

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
    }

    func testStartNewSession_dropsTheCompositionWithoutTouchingTheOldClient() throws {
        let manager = try makeManager()
        let executor = RecordingEffectExecutor()
        for character in ["t", "a", "i"] {
            manager.append(character, executing: executor)
        }
        executor.clearEffects()

        manager.startNewSession()

        XCTAssertFalse(manager.isComposing)
        XCTAssertEqual(manager.rawInput, "")
        XCTAssertTrue(
            manager.displayText.isEmpty,
            "a dropped composition leaves nothing to render, so the third mirror field clears too",
        )
        XCTAssertEqual(
            executor.effects,
            [],
            "the marked region belongs to a client this manager cannot reach — the controller clears it",
        )
    }
}
