// Drives the composing slice against the real engine and the real dictionary.

import XCTest

@testable import TaigiInputMethodCore

/// The Rust composing state is one per process, so these cases isolate
/// themselves by generation — see `GenerationCounter`.
final class RustEngineBridgeComposingTests: XCTestCase {
    private let settings = EngineSettings.defaults

    private var generation: UInt64 = 0

    override func setUp() {
        super.setUp()
        generation = TestFixtures.generationCounter.next()
        InstalledLexicon.installOnce()
    }

    // MARK: - Composing

    func testAppend_rendersPreeditAndAsksHostToUpdateIt() throws {
        let transition = try XCTUnwrap(
            RustEngineBridge.composingStart("t", settings: settings, generation: generation),
        )

        XCTAssertTrue(transition.isComposing, "starting a composition must report composing")
        XCTAssertEqual(transition.rawInput, "t")
        XCTAssertEqual(
            transition.effects.first,
            .updatePreedit("t"),
            "the host has to be told to show the preedit before anything else",
        )
    }

    func testAppend_numericTone_rendersDiacriticInDisplayButKeepsRawDigits() throws {
        _ = RustEngineBridge.composingStart("tai", settings: settings, generation: generation)
        let transition = try XCTUnwrap(
            RustEngineBridge.composingAppend("5", settings: settings, generation: generation),
        )

        XCTAssertEqual(transition.rawInput, "tai5", "raw input stays the typed search key")
        XCTAssertEqual(
            transition.displayText,
            "t\u{00E2}i",
            "tone 5 must render as the circumflex the user reads",
        )
    }

    func testDeleteBackwardToEmpty_underBareComposing_emitsTheDocumentDeleteMacOSIgnores() throws {
        // Pinning the decode, not the behaviour: this effect only reaches macOS
        // when a composition was never promoted to the continuous phase, and the
        // executor deliberately does not act on it. Decoding it wrongly would
        // hide that the case exists at all.
        _ = RustEngineBridge.composingStart("a", settings: settings, generation: generation)
        let transition = try XCTUnwrap(
            RustEngineBridge.composingDeleteBackward(settings: settings, generation: generation),
        )

        XCTAssertFalse(transition.isComposing)
        XCTAssertEqual(
            transition.effects,
            [.clearPreeditWithoutCommit, .resetAutocomplete, .deleteBackwardFromDocument],
            "the engine's abort trio must arrive whole and in order",
        )
    }

    func testReset_endsCompositionWithoutWritingToTheDocument() throws {
        _ = RustEngineBridge.composingStart("tai", settings: settings, generation: generation)
        let transition = try XCTUnwrap(RustEngineBridge.composingReset(generation: generation))

        XCTAssertFalse(transition.isComposing)
        XCTAssertTrue(
            transition.effects.contains(.clearPreeditWithoutCommit),
            "an abort must clear the preedit",
        )
        XCTAssertTrue(
            transition.effects.committedTexts.isEmpty,
            "an abort must not commit anything",
        )
    }

    func testSelectSuggestion_commitsTheTextVerbatim() throws {
        _ = RustEngineBridge.composingStart("tai", settings: settings, generation: generation)
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)
        let transition = try XCTUnwrap(RustEngineBridge.composingSelectSuggestion(
            "tai",
            settings: settings,
            generation: generation,
        ))

        XCTAssertTrue(
            transition.effects.contains(.commitTextReplacingPreedit("tai")),
            "Enter must be able to commit exactly what was typed",
        )
        XCTAssertFalse(transition.isComposing)
    }

    func testCommitPreeditThenInsertExternal_commitsCompositionAndTrailingTextTogether() throws {
        _ = RustEngineBridge.composingStart("tai", settings: settings, generation: generation)
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)
        let transition = try XCTUnwrap(RustEngineBridge.composingCommitPreeditThenInsertExternal(
            " ",
            settings: settings,
            generation: generation,
        ))

        let committed = transition.effects.committedTexts
        XCTAssertEqual(
            committed.count,
            1,
            "one keystroke must reach the host as one document mutation, not two",
        )
        XCTAssertEqual(committed.first?.hasSuffix(" "), true, "the space must ride along with the commit")
        XCTAssertFalse(transition.isComposing)
    }

    // MARK: - Continuous input

    func testFetchAtPos_afterEnteringContinuous_returnsCandidates() throws {
        _ = RustEngineBridge.composingStart("taigi", settings: settings, generation: generation)
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)

        let result = try XCTUnwrap(
            RustEngineBridge.composingFetchAtPos(settings: settings, generation: generation),
        )
        let candidates = try XCTUnwrap(result.candidates)
        let first = try XCTUnwrap(
            candidates.first,
            "the installed dictionary must produce candidates for a real word",
        )
        XCTAssertFalse(first.displayText.isEmpty)
        XCTAssertLessThanOrEqual(
            first.consumedSpanEnd,
            UInt32("taigi".utf8.count),
            "a consumed span must stay inside the raw buffer",
        )
    }

    func testFetchAtPos_whenNotComposing_reportsNoContinuousPhaseRatherThanFailure() throws {
        let result = try XCTUnwrap(
            RustEngineBridge.composingFetchAtPos(settings: settings, generation: generation),
            "an idle engine answered the query; that is not a bridge failure",
        )

        XCTAssertNil(result.candidates, "no continuous phase must read as nil, not as an empty list")
    }

    func testFetchAtPos_literalRomanCandidateToggle_gatesTheIndexZeroLiteral() throws {
        _ = RustEngineBridge.composingStart("taigi", settings: settings, generation: generation)
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)

        // Off by default, matching iOS and Android: a dictionary candidate leads.
        let hidden = try XCTUnwrap(
            RustEngineBridge.composingFetchAtPos(settings: settings, generation: generation),
        )
        let hiddenFirst = try XCTUnwrap(XCTUnwrap(hidden.candidates).first)
        XCTAssertNotEqual(
            hiddenFirst.displayText,
            "taigi",
            "with the setting off, the typed literal must not be forced to the front",
        )

        let shown = try XCTUnwrap(RustEngineBridge.composingFetchAtPos(
            settings: settings.withLiteralRomanCandidate(enabled: true),
            generation: generation,
        ))
        XCTAssertEqual(
            shown.candidates?.first?.displayText,
            "taigi",
            "with the setting on, the preedit literal leads the list so 漢羅 commits in one keystroke",
        )
    }
}

private extension EngineSettings {
    func withLiteralRomanCandidate(enabled: Bool) -> EngineSettings {
        EngineSettings(
            inputMode: inputMode,
            isDoubleTapOOEnabled: isDoubleTapOOEnabled,
            isDoubleTapNNEnabled: isDoubleTapNNEnabled,
            isTranslateSwapped: isTranslateSwapped,
            isOutputBothScripts: isOutputBothScripts,
            isLiteralRomanCandidateEnabled: enabled,
        )
    }
}
