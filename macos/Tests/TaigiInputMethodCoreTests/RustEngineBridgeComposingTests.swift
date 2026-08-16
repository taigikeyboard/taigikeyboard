// Drives the composing slice against the real engine and the real dictionary.

@testable import TaigiInputMethodCore
import XCTest

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

    /// Types `text` one character at a time, which is the only way a
    /// composition ever starts in production: `Append` enters `Phase::Composing`
    /// from Idle by itself, and there is no bridge op that seeds a whole buffer.
    @discardableResult
    private func compose(_ text: String) throws -> ComposingTransition {
        var last: ComposingTransition?
        for character in text {
            last = RustEngineBridge.composingAppend(
                String(character),
                settings: settings,
                generation: generation,
            )
        }
        return try XCTUnwrap(last, "composing '\(text)' produced no transition")
    }

    // MARK: - Composing

    func testAppend_rendersPreeditAndAsksHostToUpdateIt() throws {
        let transition = try compose("t")

        XCTAssertTrue(transition.isComposing, "starting a composition must report composing")
        XCTAssertEqual(transition.rawInput, "t")
        XCTAssertEqual(
            transition.effects.first,
            .updatePreedit("t"),
            "the host has to be told to show the preedit before anything else",
        )
    }

    func testAppend_numericTone_rendersDiacriticInDisplayButKeepsRawDigits() throws {
        _ = try compose("tai")
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
        _ = try compose("a")
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
        _ = try compose("tai")
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

    func testCommitRaw_commitsTheCompositionAsRendered() throws {
        let composed = try compose("tai")
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)

        let transition = try XCTUnwrap(
            RustEngineBridge.composingCommitRaw(settings: settings, generation: generation),
        )

        XCTAssertEqual(
            transition.effects.committedTexts,
            [composed.displayText],
            "Return must commit exactly what the marked region was showing",
        )
        XCTAssertFalse(transition.isComposing)
    }

    /// Guards the mistake this bridge used to document: committing the literal
    /// by handing the marked-region text to `SelectSuggestion` re-prepends the
    /// nailed prefix (`engine/composing/src/transition.rs:724`), so a
    /// composition reading `台北大學` with `台北` nailed would commit
    /// `台北台北大學`. `CommitRaw` is the op that does not, which is why
    /// `SelectSuggestion` has no macOS wrapper at all.
    func testCommitRaw_afterANailedSegment_writesTheCompositionOnceNotTwice() throws {
        _ = try compose("taigi")
        _ = RustEngineBridge.composingEnterContinuous(settings: settings, generation: generation)
        let pendingBytes = UInt32("taigi".utf8.count)
        let candidates = try XCTUnwrap(
            XCTUnwrap(
                RustEngineBridge.composingFetchAtPos(settings: settings, generation: generation),
            ).candidates,
        )
        let partial = try XCTUnwrap(
            candidates.first { $0.consumedSpanEnd > 0 && $0.consumedSpanEnd < pendingBytes },
            "taigi must offer a candidate shorter than the whole buffer to nail",
        )
        let nailed = try XCTUnwrap(RustEngineBridge.composingCommitContinuous(
            documentText: partial.roman,
            canonicalText: partial.displayText,
            associationTl: partial.canonicalTl,
            consumedBytes: partial.consumedSpanEnd,
            syllableCount: partial.syllableCount,
            settings: settings,
            generation: generation,
        ))
        XCTAssertTrue(nailed.isComposing, "a partial candidate nails a segment and keeps composing")

        let transition = try XCTUnwrap(
            RustEngineBridge.composingCommitRaw(settings: settings, generation: generation),
        )

        XCTAssertEqual(
            transition.effects.committedTexts,
            [nailed.displayText],
            "the nailed prefix belongs to the document once, not twice",
        )
    }

    func testCommitPreeditThenInsertExternal_commitsCompositionAndTrailingTextTogether() throws {
        _ = try compose("tai")
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
        _ = try compose("taigi")
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
        _ = try compose("taigi")
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
            isFrequencyRecordingEnabled: isFrequencyRecordingEnabled,
            isAssociationRecordingEnabled: isAssociationRecordingEnabled,
        )
    }
}
