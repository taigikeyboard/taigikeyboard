// ActionHandler extension: suggestion selection (candidate commit, output formatting).
// 中文: ActionHandler 的候選詞選取擴充 — 解析建議內容、組出輸出字串、送出並交給 NextWord 記錄關聯。

import Foundation
import KeyboardKit

extension ActionHandler {
    // MARK: - Suggestion Selection

    // 中文: 候選詞點選主入口。先處理 raw-input 直送,再依組字 / NextWord 路徑組出 commit 字串並更新使用頻率。
    func handleSuggestionSelection(_ suggestion: Autocomplete.Suggestion) {
        // Raw input candidate: commit literal keystrokes directly (no tone conversion)
        if suggestion.additionalInfo["isRawInput"] == "true" {
            composingManager.commitRawInput()
            if settings.isAutoSpaceEnabled, !settings.isTranslateSwapped {
                keyboardContext.textDocumentProxy.insertText(" ")
            }
            return
        }

        // v3.5.8 Phase 9 Item 4 — Continuous-input commit branch.
        // Per `docs/engine/continuous-input-ranking.md` §10.3 (Tap-0/Tap-N
        // commit contract + clarification γ): both slot-0 and slot-N taps
        // commit `candidate[N].display_text` — the canonical dictionary
        // string (`hanji.unwrap_or(roman)`), NOT the visual roman form that
        // Item 6 renders in the cell title. The sidechannel `displayText` is
        // the wire to the canonical form; `suggestion.text` carries the
        // post-Item-6 visual roman and may be further view-rewritten (TPS
        // layout via `CandidateCellHelper.suggestionToHandle`), so it must
        // never be used as the commit string.
        //
        // Strict-required keys (Item 4 fork F2=A): `consumedBytes`,
        // `syllableCount`, and `displayText` all come from
        // `AutocompleteService.buildContinuousSuggestions`. Missing or
        // unparseable → drop the tap silently. Falling back to
        // `selectSuggestion(text:)` would lose `consumedBytes`, corrupting
        // the engine's `Phase::Continuous { raw }` byte alignment; falling
        // back to `suggestion.text` would γ-violate the commit contract.
        //
        // Frequency recording / NextWord handshake happen via the engine
        // effects on the mid/final commit
        // (`engine/composing/src/transition.rs:649-674`).
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
            // 中文: 用 transition.effects 是否含 commitTextReplacingPreedit 取代
            // 中文: wasComposing→!nowComposing 推導,徹底關掉 generation mismatch silent
            // 中文: reset 造成的假 commit。Invariant: didFinalCommit => didCommit。
            logger.debug(
                "[BUG3] tap-enter continuous displayLen=\(displayText.count) "
                    + "consumedBytes=\(consumedBytes) syll=\(syllableCount) "
                    + "docBefore=\(bug3Tail(keyboardContext.textDocumentProxy.documentContextBeforeInput))",
            )
            let (didCommit, didFinalCommit) = composingManager.commitContinuous(
                displayText: displayText,
                consumedBytes: consumedBytes,
                syllableCount: syllableCount,
            )
            logger.debug(
                "[BUG3] tap-after commitContinuous didCommit=\(didCommit) "
                    + "didFinal=\(didFinalCommit) "
                    + "docAfter=\(bug3Tail(keyboardContext.textDocumentProxy.documentContextBeforeInput))",
            )
            // Per-segment frequency learning mirrors the lexicon path: every
            // successful commit records, mid OR final. Engine effects don't
            // call into `UserFrequencyService`; ranking learning lives at the
            // platform boundary. Sidechannel `displayText` (not view-rewritten
            // suggestion.text) ensures frequency tracks what the engine
            // committed, not the TPS surface form (PR #257 r3214912627).
            // 中文: 每次成功 commit(mid 或 final)都記頻次;頻次用 sidechannel displayText。
            if didCommit, settings.isFrequencyRecordingEnabled {
                CompositionRoot.userFrequencyService.recordUsage(for: displayText)
            }
            // Auto-space only on FINAL commit (entire buffer consumed; engine
            // exits Continuous → Idle). Mid-commits keep composing more
            // syllables and must NOT insert a space.
            // 中文: 只有 final-commit 才補空白(整個 buffer 被消化、engine 退到 Idle)。
            if didFinalCommit, settings.isAutoSpaceEnabled {
                let isTPSLayout = settings.keyboardLayoutType == .tps
                let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped
                if !effectiveSwapped || settings.isOutputBothScripts {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }
            return
        }

        let isNextWordPrediction = suggestion.additionalInfo["isNextWord"] == "true"

        if composingManager.isComposing || isNextWordPrediction {
            let isTPSLayout = settings.keyboardLayoutType == .tps
            let effectiveSwapped = isTPSLayout || settings.isTranslateSwapped

            let (roman, hanzi) = parseRomanAndHanzi(from: suggestion, isNextWord: isNextWordPrediction, effectiveSwapped: effectiveSwapped)
            let textToCommit = formatOutputText(roman: roman, hanzi: hanzi, isTPSLayout: isTPSLayout, effectiveSwapped: effectiveSwapped)

            commitSuggestionText(textToCommit, isNextWord: isNextWordPrediction, suggestion: suggestion)

            let displayText = suggestion.additionalInfo["displayText"] ?? hanzi ?? roman
            if settings.isFrequencyRecordingEnabled {
                CompositionRoot.userFrequencyService.recordUsage(for: displayText)
            }

            logger.debug("[SELECT] suggestion.text='\(suggestion.text)' subtitle='\(suggestion.subtitle ?? "nil")' additionalInfo=\(suggestion.additionalInfo.description)")
            logger.debug("[SELECT] parsed roman='\(roman)' hanzi='\(hanzi ?? "nil")' displayText='\(displayText)'")

            // Romanization mode: auto-space (unless trailing hyphen)
            // TPS mode disables auto-space (effectiveSwapped is true for TPS)
            if settings.isAutoSpaceEnabled, !effectiveSwapped || settings.isOutputBothScripts {
                if !textToCommit.hasSuffix("-") {
                    keyboardContext.textDocumentProxy.insertText(" ")
                }
            }

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

    /// Extract romanization and Hanji from suggestion based on display mode
    // 中文: 依顯示模式從候選建議中拆出羅馬字 + 漢字。NextWord 路徑要把先前 swap 過的欄位還原。
    private func parseRomanAndHanzi(
        from suggestion: Autocomplete.Suggestion,
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
    // 中文: 依顯示模式組出最終輸出字串(純羅馬字 / 純漢字 / 兩種並陳)。TPS layout 時用 TPS bracket 顯示。
    private func formatOutputText(roman: String, hanzi: String?, isTPSLayout: Bool, effectiveSwapped: Bool) -> String {
        let bracketRoman = isTPSLayout
            ? RustEngineBridge.tlDisplayToTPS(roman, orMapsToER: settings.isTpsOrMappedToER)
            : roman

        if settings.isOutputBothScripts, let hanzi, !hanzi.isEmpty {
            return effectiveSwapped
                ? "\(hanzi) (\(bracketRoman))"
                : "\(bracketRoman) (\(hanzi))"
        } else if effectiveSwapped, let hanzi, !hanzi.isEmpty {
            return hanzi
        } else {
            return roman
        }
    }

    /// Commit text via proxy (NextWord) or composing manager (regular candidate).
    /// The `suggestion` parameter is kept for future telemetry/logging use
    /// but ComposingManager only needs the candidate text.
    // 中文: NextWord 直接走 textDocumentProxy 插字;一般候選走 ComposingManager.selectSuggestion。
    private func commitSuggestionText(_ text: String, isNextWord: Bool, suggestion _: Autocomplete.Suggestion) {
        if isNextWord {
            keyboardContext.textDocumentProxy.insertText(text)
        } else {
            composingManager.selectSuggestion(text: text)
        }
    }
}
