// ActionHandler extension: suggestion selection (candidate commit, output formatting).
// ActionHandler 的候選詞選取擴充 — 解析建議內容、組出輸出字串、送出並交給 NextWord 記錄關聯。

import Foundation
import KeyboardKit

/// What one commit writes into the document, and whether that string carries
/// romanization — the single input the auto-space gate reads.
// CROSS-PLATFORM INVARIANT — mirrors android `ResolvedCommit`, macos
// `CandidateDocumentText.ResolvedCommit`, windows
// `composing::document_text::ResolvedCommit`.
struct ResolvedCommit {
    let text: String
    let wroteRomanization: Bool
}

extension ActionHandler {
    // MARK: - Suggestion Selection

    // 候選詞點選主入口。先處理 raw-input 直送,再依組字 / NextWord 路徑組出 commit 字串並更新使用頻率。
    func handleSuggestionSelection(_ suggestion: AutocompleteSuggestion) {
        // Raw input candidate: commit literal keystrokes directly (no tone conversion)
        if suggestion.additionalInfo["isRawInput"] == "true" {
            composingManager.commitRawInput()
            // The literal keystrokes go into the document, so the verdict is
            // what the layout composes — romanization in TL/POJ, Bopomofo in
            // TPS, which takes no spacing. The output mode does not enter into
            // it: a raw commit writes the same string whichever script the
            // candidates would have led with.
            appendAutoSpaceIfEarned(
                documentText: suggestion.text,
                wroteRomanization: Self.rawPreeditWritesRomanization(isTPSLayout: isTPSLayout),
            )
            return
        }

        // v3.5.8 Phase 9 Item 4 + Bug 1 — Continuous-input commit branch.
        // Per `docs/engine/continuous-input-ranking.md` §10.3 clarification γ
        // (REVISED, Bug 1): the tap commits the **swap/TPS/both-scripts-
        // formatted document string** — formatted from the candidate's
        // roman/hanji by the SAME `parseRomanAndHanzi` + `formatOutputText`
        // helpers the legacy lexicon branch uses, so Continuous and lexicon
        // commits are identical for the same candidate under the same
        // settings. The sidechannel `displayText` (= engine canonical
        // `hanji ?? roman`) is NOT the document string anymore; it is
        // forwarded as `commitContinuous(canonicalText:)` so the engine keys
        // `user_frequency.db` + NextWord on the canonical token regardless of
        // display mode (user decision b).
        //
        // Strict-required keys (Item 4 fork F2=A): `consumedBytes`,
        // `syllableCount`, and `displayText` all come from
        // `TaigiAutocompleteService.buildContinuousSuggestions`. Missing or
        // unparseable → drop the tap silently. Falling back to
        // `selectSuggestion(text:)` would lose `consumedBytes`, corrupting
        // the engine's `Phase::Continuous { raw }` byte alignment.
        //
        // Frequency recording stays on the canonical sidechannel below;
        // NextWord handshake fires via the engine effects (now keyed on
        // `canonical_text`) on the mid/final commit
        // (`engine/composing/src/transition.rs` `commit_continuous`).
        if suggestion.additionalInfo["isContinuous"] == "true" {
            guard let displayText = suggestion.additionalInfo["displayText"],
                  let consumedBytesStr = suggestion.additionalInfo["consumedBytes"],
                  let syllableCountStr = suggestion.additionalInfo["syllableCount"],
                  let consumedBytes = UInt32(consumedBytesStr),
                  let syllableCount = UInt32(syllableCountStr)
            else {
                logger.debug(
                    "[SELECT] continuous metadata decode failed; dropping tap. "
                        + "additionalInfo=\(suggestion.additionalInfo.description)",
                )
                return
            }
            // Effect-backed commit signal (Codex PR #257 r3214932308):
            // `commitContinuous` returns `(didCommit, didFinalCommit)` derived
            // from `transition.effects` containing `.commitTextReplacingPreedit`.
            // This closes the generation-mismatch race left open by earlier
            // wasComposing/isComposing gating: when `engine/composing/src/handle.rs:61-65`
            // silently resets the engine to Idle before dispatch, the resulting
            // CommitContinuous noop emits zero effects, so both flags stay
            // false and neither frequency recording nor auto-space fires for
            // text that was never written.
            // 用 transition.effects 是否含 commitTextReplacingPreedit 取代
            // wasComposing→!nowComposing 推導,徹底關掉 generation mismatch silent
            // reset 造成的假 commit。Invariant: didFinalCommit => didCommit。
            // v3.5.8 Phase 9 Bug 1 (Option A): the continuous tap must commit
            // the SAME swap/TPS/both-scripts-formatted string the legacy
            // lexicon path commits — reuse `parseRomanAndHanzi` +
            // `formatOutputText` verbatim so parity is by construction
            // (`parseRomanAndHanzi` already compensates for
            // `CandidateCellHelper.suggestionToHandle`'s text↔subtitle
            // pre-swap / TPS rewrite). The canonical sidechannel `displayText`
            // (= engine `hanji ?? roman`) is forwarded as `canonicalText` so
            // `user_frequency.db` + NextWord keys stay mode-independent
            // (user decision b). Frequency recording below already keys on
            // the canonical sidechannel and is unchanged.
            //
            // §42 漢羅濫 split cells arrive with an `additionalInfo["cellScript"]`
            // marker and resolve the document text DIRECTLY from the marker +
            // info fields (`markedCellCommit` below), bypassing the swap
            // reconstruction — the identity sidechannels are shared by both
            // cells, so 詞頻 / NextWord recording is unchanged whichever cell
            // of the same candidate is tapped.
            // 帶 cellScript 標記的 split cell 直接由標記解出 commit 字串;
            // 兩個 cell 共用同一組 identity sidechannel,學習路徑不變。
            let docText: String
            // Whether this commit wrote romanization into the document — the
            // SINGLE auto-space verdict read by the gate below (§42: the space
            // follows the script actually committed, not the mode). Both arms
            // resolve it beside the string, never from the mode afterwards:
            //   marked roman cell          → true  (bare roman)
            //   marked hanji, 括號標註 OFF → false (pure 漢字)
            //   marked hanji, 括號標註 ON  → true  (`漢字 (羅馬字)` DID write
            //                                       the romanization)
            //   unmarked, no hanji         → true  (§34 literal, an OOV name:
            //                                       romanization under every mode)
            //   unmarked, hanji + swapped  → false (pure 漢字)
            //   unmarked, otherwise        → true  (roman-led, or the bracket form)
            let wroteRomanization: Bool
            if let cellScript = CandidateCellScript.marker(for: suggestion) {
                let resolved = Self.markedCellCommit(
                    cellScript: cellScript,
                    cellText: suggestion.text,
                    roman: suggestion.additionalInfo[CandidateCellScript.bracketRomanKey],
                    isOutputBothScripts: settings.isOutputBothScripts,
                )
                docText = resolved.text
                wroteRomanization = resolved.wroteRomanization
            } else {
                // Unmarked commit — or a wire-defective marker, which
                // `CandidateCellScript.marker` already declined for the render
                // guard too, so this suggestion still carries the un-split
                // dual-script shape this path expects.
                let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped
                let (roman, hanzi) = parseRomanAndHanzi(
                    from: suggestion,
                    isNextWord: false,
                    effectiveSwapped: effectiveSwapped,
                )
                let resolved = formatOutputText(
                    roman: roman,
                    hanzi: hanzi,
                    isTPSLayout: isTPSLayout,
                    effectiveSwapped: effectiveSwapped,
                )
                docText = resolved.text
                wroteRomanization = resolved.wroteRomanization
            }
            // R2: canonical TL identity sidechannel — forwarded as
            // `associationTl` so NextWord learns the same `next_tl`/`prev_tl`
            // a normal candidate commit records. Absent (wire skew / older
            // suggestion) → "" → engine falls back to the raw committed slice.
            let associationTl = suggestion.additionalInfo["canonicalTl"] ?? ""
            let (didCommit, didFinalCommit) = composingManager.commitContinuous(
                displayText: docText,
                canonicalText: displayText,
                associationTl: associationTl,
                consumedBytes: consumedBytes,
                syllableCount: syllableCount,
            )
            // Per-segment frequency learning mirrors the lexicon path: every
            // successful commit records, mid OR final. Engine effects don't
            // call into `UserFrequencyService`; ranking learning lives at the
            // platform boundary. Sidechannel `displayText` (not view-rewritten
            // suggestion.text) ensures frequency tracks what the engine
            // committed, not the TPS surface form (PR #257 r3214912627).
            // 每次成功 commit(mid 或 final)都記頻次;頻次用 sidechannel displayText。
            // R5 pair-key (#7): record `(displayText, canonical TL)` so
            // 一字多音 keep separate frequency buckets. `associationTl` is
            // the canonical-TL sidechannel already extracted above (the same
            // reading NextWord learns); empty only on wire skew / TPS-OOV.
            if didCommit, settings.isFrequencyRecordingEnabled {
                CompositionRoot.userFrequencyService.recordUsage(for: displayText, tl: associationTl)
            }
            // Auto-space only on FINAL commit (entire buffer consumed; engine
            // exits Continuous → Idle). Mid-commits keep composing more
            // syllables and must NOT insert a space.
            // 只有 final-commit 才補空白(整個 buffer 被消化、engine 退到 Idle)。
            // Keys on the RESOLVED committed script — the `wroteRomanization`
            // verdict resolved above beside the string itself. Suffix check is
            // on the actual committed document string (`docText`) so a trailing
            // hyphen continuation suppresses the space — mirrors the legacy
            // lexicon path (Codex post-impl: auto-space suffix check must use
            // the document string, not the canonical key).
            if didFinalCommit {
                appendAutoSpaceIfEarned(documentText: docText, wroteRomanization: wroteRomanization)
            }
            return
        }

        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped

            let (roman, hanzi) = parseRomanAndHanzi(from: suggestion, isNextWord: isNextWordPrediction, effectiveSwapped: effectiveSwapped)
            let resolved = formatOutputText(roman: roman, hanzi: hanzi, isTPSLayout: isTPSLayout, effectiveSwapped: effectiveSwapped)
            let textToCommit = resolved.text

            commitSuggestionText(textToCommit, isNextWord: isNextWordPrediction, suggestion: suggestion)

            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            // R5 pair-key (#7): canonical-TL reading from the same
            // sidechannel; empty (legacy bucket) for a NextWord prediction
            // that carries no canonical TL.
            let canonicalTl = suggestion.additionalInfo["canonicalTl"] ?? ""
            if settings.isFrequencyRecordingEnabled {
                CompositionRoot.userFrequencyService.recordUsage(for: displayText, tl: canonicalTl)
            }

            logger.debug("[SELECT] suggestion.text='\(suggestion.text)' subtitle='\(suggestion.subtitle ?? "nil")' additionalInfo=\(suggestion.additionalInfo.description)")
            logger.debug("[SELECT] parsed roman='\(roman)' hanzi='\(hanzi ?? "nil")' displayText='\(displayText)'")

            // Romanization mode: auto-space (unless trailing hyphen).
            // TPS mode disables auto-space (effectiveSwapped is true for TPS).
            appendAutoSpaceIfEarned(
                documentText: textToCommit,
                wroteRomanization: resolved.wroteRomanization,
            )

            // Fork: `roman` is the commit string (may be POJ/Hanji); the engine
            // expects raw TL (it calls `pojToTL` on it). Next-word candidates
            // carry raw TL on the `additionalInfo["tl"]` sidechannel — use it
            // here to preserve association-recording semantics.
            let associationRoman = isNextWordPrediction
                ? (suggestion.additionalInfo["tl"] ?? "")
                : roman
            nextWordController.process(text: displayText, roman: associationRoman)
        } else {
            keyboardContext.textDocumentProxy.insertText(suggestion.text)
        }
    }

    // MARK: - Suggestion Helpers

    /// §42 漢羅濫 marked-cell document text.
    ///
    /// A split cell's `cellScript` marker is authoritative, so the document
    /// string resolves directly from the marker + info fields — never through
    /// `parseRomanAndHanzi` (whose contract is "derive from the UI-shaped
    /// suggestion") and never through a new `formatOutputText` arm:
    /// - `"hanji"` cell commits the hanji; 括號標註 ON appends the roman
    ///   sidechannel as `漢字 (羅馬字)` — today's swapped output. TPS never
    ///   applies (濫 is TL/POJ only), so the bracket roman is never TPS-rendered.
    /// - `"roman"` cell commits the BARE roman; 括號標註 is ignored (desktop
    ///   `.alternate` parity).
    ///
    /// Precondition: `cellScript` came from `CandidateCellScript.marker(for:)`,
    /// so it is a marker this build knows and `cellText` is non-empty — a wire
    /// defect is declined there, by the render guard and this caller together.
    /// A hanji cell whose roman sidechannel is missing commits the bare hanji
    /// rather than empty brackets.
    ///
    /// Returns the document string plus whether the commit wrote romanization
    /// (the bracket form DID write it, so the hanji arm's verdict is
    /// `isOutputBothScripts`). Static with the settings flag injected so tests
    /// pin it without a keyboard context.
    // 漢字 cell 出漢字(括號標註 ON 補 `(羅馬字)`);羅馬字 cell 恆出裸羅馬字、
    // 無視括號標註;回傳是否寫出羅馬字供自動空白判斷。壞掉的標記在
    // CandidateCellScript.marker 就被擋掉,render 與 commit 一起退回未標記路徑。
    static func markedCellCommit(
        cellScript: String,
        cellText: String,
        roman: String?,
        isOutputBothScripts: Bool,
    ) -> ResolvedCommit {
        guard cellScript == CandidateCellScript.hanji else {
            return ResolvedCommit(text: cellText, wroteRomanization: true)
        }
        guard isOutputBothScripts, let roman, !roman.isEmpty else {
            return ResolvedCommit(text: cellText, wroteRomanization: false)
        }
        return ResolvedCommit(
            text: Self.bracketedHanjiCommit(hanzi: cellText, roman: roman),
            wroteRomanization: true,
        )
    }

    /// Whether to insert the trailing auto-space: the setting is on, the commit
    /// wrote romanization, and the committed DOCUMENT string does not end in a
    /// hyphen continuation (a mid-word 連字 keeps composing). The caller still
    /// owns the final-commit gate on the Continuous path.
    // CROSS-PLATFORM INVARIANT — mirrors android/.../smartbar/CandidateClickHandler.kt
    // `shouldAppendAutoSpace`. Drift causes silent divergence (one platform spacing
    // after a hyphen continuation).
    static func shouldAppendAutoSpace(
        isAutoSpaceEnabled: Bool,
        wroteRomanization: Bool,
        documentText: String,
    ) -> Bool {
        isAutoSpaceEnabled && wroteRomanization && !documentText.hasSuffix("-")
    }

    /// The 括號標註 hanji-led output shape `漢字 (羅馬字)` — single spelling
    /// shared by `formatOutputText`'s swapped arm and the §42 marked hanji cell.
    static func bracketedHanjiCommit(hanzi: String, roman: String) -> String {
        "\(hanzi) (\(roman))"
    }

    /// Extract romanization and Hanji from suggestion based on display mode
    // 依顯示模式從候選建議中拆出羅馬字 + 漢字。NextWord 路徑要把先前 swap 過的欄位還原。
    private func parseRomanAndHanzi(
        from suggestion: AutocompleteSuggestion,
        isNextWord: Bool,
        effectiveSwapped: Bool,
    ) -> (roman: String, hanzi: String?) {
        if isNextWord {
            // CROSS-PLATFORM INVARIANT: next-word commit string == UI display string.
            // `suggestion.text` is mode-shaped (POJ in POJ mode, TL otherwise) by
            // `RustEngineBridge.nextwordFilter` (Rust shape rule), but `CandidateCellHelper.suggestionToHandle`
            // pre-swaps text↔subtitle in swapped/TPS modes before this handler runs —
            // so we must mirror that swap to recover the mode-shaped roman.
            // `additionalInfo["hanzi"]` carries hanzi even for hanzi-only predictions
            // (Case B) where `subtitle == nil`. The raw-TL sidechannel on
            // `additionalInfo["tl"]` is consumed separately at the association call
            // site (see `handleSuggestionSelection`).
            // Mirror: android/.../smartbar/NextWordHandler.kt:355-363 (TaigiWord.roman).
            // Swapped/TPS Case B (hanzi-only, no roman): `subtitle == nil` after
            // `suggestionToHandle` (swap gate requires non-empty subtitle). Fall
            // back to `""` so bracket-mode output stays `"漢字 ()"` — matches the
            // pre-fix sidechannel behavior, avoids Hanji duplication.
            let roman = effectiveSwapped
                ? (suggestion.subtitle ?? "")
                : suggestion.text
            return (roman, suggestion.additionalInfo["hanzi"])
        } else if effectiveSwapped {
            return (suggestion.subtitle ?? suggestion.text, suggestion.text)
        } else {
            return (suggestion.text, suggestion.subtitle)
        }
    }

    /// Format output text based on display mode (roman, Hanji, or both scripts)
    // 依顯示模式組出最終輸出字串(純羅馬字 / 純漢字 / 兩種並陳)。TPS layout 時用 TPS bracket 顯示。
    /// The document string AND whether writing it puts romanization in the
    /// document — resolved together, by the one branch that picks the string.
    ///
    /// Auto-space is a property of ROMANIZATION (`guá beh khì` needs the gaps,
    /// 我欲去 does not), so its gate has to answer for the string this commit
    /// actually writes. Deriving the verdict from the output mode instead is
    /// only ever an approximation, and it is wrong for a candidate with no
    /// Hanji: the 字面羅馬字 candidate (§34), an out-of-vocabulary name, a
    /// romanization-only custom entry all fall to the last arm and write
    /// romanization whatever the mode leads with.
    // CROSS-PLATFORM INVARIANT — mirrors android/.../CandidateClickHandler.kt
    // `unmarkedCommit` and macos/.../CandidateDocumentText.swift `resolved`.
    // Drift changes which commits earn a space.
    private func formatOutputText(
        roman: String,
        hanzi: String?,
        isTPSLayout: Bool,
        effectiveSwapped: Bool,
    ) -> ResolvedCommit {
        let bracketRoman = isTPSLayout
            ? RustEngineBridge.tlDisplayToTPS(roman, orMapsToER: settings.isTpsOrMappedToER)
            : roman

        // 括號標註 writes the pair, so the romanization IS in the document
        // whichever half leads.
        if settings.isOutputBothScripts, let hanzi, !hanzi.isEmpty {
            let text = effectiveSwapped
                ? Self.bracketedHanjiCommit(hanzi: hanzi, roman: bracketRoman)
                : "\(bracketRoman) (\(hanzi))"
            return ResolvedCommit(text: text, wroteRomanization: true)
        } else if effectiveSwapped, let hanzi, !hanzi.isEmpty {
            return ResolvedCommit(text: hanzi, wroteRomanization: false)
        } else {
            return ResolvedCommit(text: roman, wroteRomanization: true)
        }
    }

    /// Inserts the trailing auto space when this commit earned it, and arms
    /// the punctuation swap on it — the ONE insertion site, so the space and
    /// the knowledge that it is ours can never come apart.
    ///
    /// A commit that earns nothing arms nothing: the event already consumed
    /// the previous arm (`beginInputEvent`), so the next punctuation key sees
    /// no space of ours to swap.
    func appendAutoSpaceIfEarned(documentText: String, wroteRomanization: Bool) {
        guard Self.shouldAppendAutoSpace(
            isAutoSpaceEnabled: settings.isAutoSpaceEnabled,
            wroteRomanization: wroteRomanization,
            documentText: documentText,
        ) else { return }
        keyboardContext.textDocumentProxy.insertText(" ")
        armAutoSpaceSwap()
    }

    /// The auto-space verdict for Return on a HIGHLIGHTED candidate, which
    /// commits the suggestion's own `text` verbatim rather than the
    /// mode-formatted document string the tap path builds
    /// (`ComposingManager.confirmSelectedCandidate`). So the question is which
    /// script that one string is, and the candidate's own fields answer it —
    /// never the output mode, which would space a 漢字 commit in a
    /// romanization-led list and refuse one in a 漢字-led list.
    func highlightedCandidateWroteRomanization(_ suggestion: AutocompleteSuggestion) -> Bool {
        // A §42 split cell says which script it is outright.
        if let cellScript = CandidateCellScript.marker(for: suggestion) {
            return cellScript != CandidateCellScript.hanji
        }
        let (_, hanzi) = parseRomanAndHanzi(
            from: suggestion,
            isNextWord: false,
            effectiveSwapped: isTPSLayout || settings.isTranslateSwapped,
        )
        guard let hanzi, !hanzi.isEmpty else { return true }
        return suggestion.text != hanzi
    }

    /// Whether the layout in use composes romanization — TL and POJ do, TPS
    /// composes Bopomofo, which takes no word spacing. The verdict for every
    /// commit that writes the composition AS TYPED (the raw-input candidate,
    /// Return on the first slot), none of which goes through a candidate's
    /// rendering.
    // CROSS-PLATFORM INVARIANT — one name on all four platforms: android
    // `rawPreeditWritesRomanization`, macOS/Windows
    // `AutoSpacePolicy.rawPreeditWritesRomanization(inputMode:)` /
    // `policies::raw_preedit_writes_romanization`.
    static func rawPreeditWritesRomanization(isTPSLayout: Bool) -> Bool {
        !isTPSLayout
    }

    /// The layout composes Bopomofo rather than romanization. Named once —
    /// four call sites in this file alone read it (Android: `prefs.isTpsLayout`).
    var isTPSLayout: Bool {
        settings.keyboardLayoutType == .tps
    }

    /// Commit text via proxy (NextWord) or composing manager (regular candidate).
    /// The `suggestion` parameter is kept for future telemetry/logging use
    /// but ComposingManager only needs the candidate text.
    // NextWord 直接走 textDocumentProxy 插字;一般候選走 ComposingManager.selectSuggestion。
    private func commitSuggestionText(_ text: String, isNextWord: Bool, suggestion _: AutocompleteSuggestion) {
        if isNextWord {
            keyboardContext.textDocumentProxy.insertText(text)
        } else {
            composingManager.selectSuggestion(text: text)
        }
    }
}
