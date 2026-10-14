// ActionHandler extension: suggestion selection (Continuous and NextWord prediction taps, auto space).

import Foundation
import KeyboardKit

extension ActionHandler {
    // MARK: - Suggestion Selection

    func handleSuggestionSelection(_ suggestion: AutocompleteSuggestion) {
        // Continuous candidate (R5): the engine resolves what the pick writes
        // from the candidate's own scripts under the live settings, counts the
        // pick, and answers the auto-space verdict — so Continuous commits match
        // macOS / Windows / Linux / Android by construction. The pick is read
        // off the suggestion's metadata, never its view-rewritten `text`.
        if suggestion.additionalInfo["isContinuous"] == "true" {
            // Strict-required metadata (Item 4 fork F2=A): missing → drop the
            // tap. Committing the cell text without it would lose
            // `consumedBytes` and corrupt `Phase::Continuous { raw }` alignment.
            guard let pick = Self.continuousPick(for: suggestion) else {
                logger.debug(
                    "[SELECT] continuous metadata decode failed; dropping tap. "
                        + "additionalInfo=\(suggestion.additionalInfo.description)",
                )
                return
            }
            // Auto-space only on the FINAL commit, as the engine judged what the
            // pick wrote (romanization, no trailing `-`, §23); a nail keeps
            // composing and a stale / rejected pick wrote nothing.
            if case let .finalized(earnsAutoSpace) = composingManager.commitContinuous(pick) {
                appendAutoSpace(ifEarned: settings.isAutoSpaceEnabled && earnsAutoSpace)
            }
            return
        }

        // Every composing-time candidate is a Continuous one (above), so what
        // remains is a NextWord prediction tap or a plain insert.
        guard suggestion.additionalInfo["isNextWord"] == "true" else {
            hostText.insert(suggestion.text)
            return
        }

        // A NextWord prediction tap (D1): the engine resolves what it writes
        // from the prediction's own scripts under the live settings and counts
        // the pick — the strip / overlay may have rewritten `text` (the swap,
        // the TPS rendering), so the pick is read off the metadata.
        let pick = Self.predictionPick(for: suggestion)
        guard let commit = composingManager.commitPrediction(pick) else {
            logger.debug(
                "[SELECT] prediction ignored by the engine; dropping tap. "
                    + "additionalInfo=\(suggestion.additionalInfo.description)",
            )
            return
        }
        hostText.insert(commit.text)
        logger.debug("[SELECT] prediction script=\(pick.script) textToCommit='\(commit.text)'")
        appendAutoSpace(ifEarned: settings.isAutoSpaceEnabled && commit.earnsAutoSpace)

        // NextWord learns the reading as sent, so it gets the canonical TL,
        // never the rendered `roman` (POJ in POJ mode; a POJ → TL fold would
        // misread TL `eng` / `ek`).
        nextWordController.process(text: pick.hanji, roman: pick.canonicalTl)
    }

    // MARK: - Suggestion Helpers

    /// The engine request for a Continuous candidate tap, read off the
    /// suggestion's metadata alone (`TaigiAutocompleteService.continuousSidechannels`)
    /// — the view may have rewritten `text` (`CandidateCellHelper.suggestionToHandle`:
    /// the swap, the TPS rendering). `nil` when a strict key is missing.
    static func continuousPick(for suggestion: AutocompleteSuggestion) -> RustEngineBridge.ContinuousPick? {
        let info = suggestion.additionalInfo
        guard let canonicalText = info["displayText"],
              let roman = info["roman"],
              let consumedBytes = info["consumedBytes"].flatMap(UInt32.init),
              let syllableCount = info["syllableCount"].flatMap(UInt32.init)
        else {
            return nil
        }
        return RustEngineBridge.ContinuousPick(
            script: commitScript(for: suggestion),
            roman: roman,
            canonicalText: canonicalText,
            // Absent only when the pick carries no reading (TPS-OOV) → "" → the
            // engine uses the raw committed slice for NextWord.
            associationTl: info["canonicalTl"] ?? "",
            hanji: info["hanji"],
            consumedBytes: consumedBytes,
            syllableCount: syllableCount,
        )
    }

    /// Which script a Continuous tap commits: a §42 split cell the one its
    /// marker names, every other cell what the output settings lead with. A
    /// defective marker is declined by `CandidateCellScript.marker`, so the
    /// cell commits the lead, as the render guard shows it.
    // CROSS-PLATFORM INVARIANT — mirrors android CandidateClickHandler.kt `commitScript`.
    static func commitScript(for suggestion: AutocompleteSuggestion) -> Taigi_Engine_CommitScript {
        switch CandidateCellScript.marker(for: suggestion) {
        case CandidateCellScript.hanji: .hanji
        case CandidateCellScript.roman: .roman
        default: .lead
        }
    }

    /// The engine request for a NextWord prediction tap, read off the
    /// suggestion's metadata (`ActionHandler.predictionIdentity`) — never its
    /// view-rewritten `text` / `subtitle`. A §42 split cell commits the script
    /// its marker names (`commitScript(for:)`); a split cell whose own script
    /// is missing commits the lead, as a Continuous pick does.
    static func predictionPick(for suggestion: AutocompleteSuggestion) -> RustEngineBridge.PredictionPick {
        let info = suggestion.additionalInfo
        return RustEngineBridge.PredictionPick(
            script: commitScript(for: suggestion),
            roman: info["roman"] ?? "",
            hanji: info["hanzi"] ?? "",
            canonicalTl: info["canonicalTl"] ?? "",
        )
    }

    /// Whether to insert the trailing auto-space: the setting is on, the commit
    /// wrote romanization, and the committed DOCUMENT string does not end in a
    /// hyphen continuation (a mid-word hyphen keeps composing). The caller still
    /// owns the final-commit gate on the Continuous path.
    // CROSS-PLATFORM INVARIANT — mirrors android/.../candidates/CandidateClickHandler.kt
    // `shouldAppendAutoSpace`. Drift causes silent divergence (one platform spacing
    // after a hyphen continuation).
    static func shouldAppendAutoSpace(
        isAutoSpaceEnabled: Bool,
        wroteRomanization: Bool,
        documentText: String,
    ) -> Bool {
        isAutoSpaceEnabled && wroteRomanization && !documentText.hasSuffix("-")
    }

    /// Inserts the trailing auto space when this commit earned it, and arms
    /// the punctuation swap on it — the ONE insertion site, so the space and
    /// the knowledge that it is ours can never come apart.
    ///
    /// A commit that earns nothing arms nothing: the event already consumed
    /// the previous arm (`performInputEvent`), so the next punctuation key sees
    /// no space of ours to swap.
    func appendAutoSpaceIfEarned(documentText: String, wroteRomanization: Bool) {
        appendAutoSpace(ifEarned: Self.shouldAppendAutoSpace(
            isAutoSpaceEnabled: settings.isAutoSpaceEnabled,
            wroteRomanization: wroteRomanization,
            documentText: documentText,
        ))
    }

    private func appendAutoSpace(ifEarned isEarned: Bool) {
        guard isEarned else { return }
        hostText.insert(" ")
        armAutoSpaceSwap()
    }

    /// Whether the layout in use composes romanization — TL and POJ do, TPS
    /// composes Bopomofo, which takes no word spacing. The verdict for every
    /// commit that writes the composition AS TYPED (the raw-input candidate,
    /// Return while composing), none of which goes through a candidate's
    /// rendering.
    // CROSS-PLATFORM INVARIANT — one name on all five platforms: android
    // `rawPreeditWritesRomanization`, desktop-core (macOS / Windows / Linux)
    // `policies::raw_preedit_writes_romanization`.
    static func rawPreeditWritesRomanization(isTPSLayout: Bool) -> Bool {
        !isTPSLayout
    }

    /// The layout composes Bopomofo rather than romanization. Named once —
    /// the raw-preedit commits read it (Android: `prefs.isTpsLayout`).
    var isTPSLayout: Bool {
        settings.keyboardLayoutType == .tps
    }
}
