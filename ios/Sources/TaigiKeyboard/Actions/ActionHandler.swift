import Foundation
import KeyboardKit

/// Taigi keyboard action handler — dispatches keyboard gestures to per-action handlers.
///
/// **Action flow** (gesture → output):
/// 1. `handle(_:on:)` — KeyboardKit entry point, filters gesture type
/// 2. `handleTaigiSpecificAction` — dispatches by action type
/// 3. Per-action handlers (in extension files):
///    - `handleCharacterInput` → composing / direct output  (KeyActions)
///    - `handleSpaceAction` → commit composing / insert space  (KeyActions)
///    - `handleReturnAction` → commit raw or selected candidate  (KeyActions)
///    - `handleBackspaceAction` → delete / re-predict NextWord  (KeyActions)
///    - `handleSuggestionSelection` → commit + frequency + NextWord  (Suggestions)
/// 4. `nextWordController.process()` — record association → update state → predict  (NextWordController)
public class ActionHandler: KeyboardAction.StandardActionHandler {
    // MARK: - Properties

    let logger = DebugLogger(category: "ActionHandler")

    let settings = SharedSettings.shared
    public let composingManager = ComposingManager()
    let nextWordController = NextWordController()

    /// Distinguishes space-drag (cursor move) from space-tap (insert space)
    private var isSpaceDragInProgress = false

    // MARK: - Action Dispatch

    /// - Returns: true if handled (skip KeyboardKit default)
    private func handleTaigiSpecificAction(_ action: KeyboardAction) -> Bool {
        switch action {
        case .settings:
            openMainAppSettings()
            return false

        case let .character(char):
            return handleCharacterInput(char)

        case .space:
            return handleSpaceAction()

        case .primary(.return):
            return handleReturnAction()

        case .backspace:
            return handleBackspaceAction()

        case let .custom(name):
            handleCustomAction(name)
            return false

        default:
            return false
        }
    }

    // MARK: - KeyboardKit Override

    override public func handle(_ gesture: Keyboard.Gesture, on action: KeyboardAction) {
        // Space drag state tracking
        if action == .space {
            switch gesture {
            case .longPress:
                isSpaceDragInProgress = true
            case .release:
                if isSpaceDragInProgress {
                    isSpaceDragInProgress = false
                    super.handle(gesture, on: action) // Let KeyboardKit handle drag end
                    return
                }
                isSpaceDragInProgress = false
            case .end:
                isSpaceDragInProgress = false
            default:
                break
            }
        }

        guard gesture == .release else {
            if action == .backspace {
                // Allow backspace repeat-press gesture
                if gesture == .repeatPress {
                    TraceContext.with(TraceId.next()) {
                        logger.debug("[INPUT] fn=handle gesture=repeatPress action=\(String(describing: action))")
                        _ = handleBackspaceAction()
                    }
                }
                return
            }

            super.handle(gesture, on: action)
            return
        }

        var handled = false
        TraceContext.with(TraceId.next()) {
            logger.debug("[INPUT] fn=handle gesture=release action=\(String(describing: action))")
            handled = handleTaigiSpecificAction(action)
            if handled, !shouldSkipAutocomplete(for: action) {
                keyboardController?.performAutocomplete()
            }
        }
        if handled {
            return
        }
        super.handle(gesture, on: action)
    }

    /// Align with Android: skip autocomplete to preserve NextWord suggestions
    /// when not composing and pressing space or "-" during NextWord
    private func shouldSkipAutocomplete(for action: KeyboardAction) -> Bool {
        guard !composingManager.isComposing else { return false }
        if action == .space { return true }
        if case .character("-") = action, nextWordController.isShowing { return true }
        return false
    }

    override public func handle(_ suggestion: Autocomplete.Suggestion) {
        // English mode: use KeyboardKit default (auto-deletes typed chars then inserts)
        if settings.inputMode == .english {
            super.handle(suggestion)
            return
        }
        // Taigi mode: custom handling
        TraceContext.with(TraceId.next()) {
            handleSuggestionSelection(suggestion)
        }
    }

    override public func handle(_ action: KeyboardAction) {
        handle(.release, on: action)
    }

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// Part of 3-layer workaround — see KeyboardViewController.setupKeyboardCaseProtection() (Layer 2).
    /// Remove when KeyboardKit provides a proper API to disable auto-capitalization.
    ///
    /// - Shift: always let super handle (preserves doubleTap → Caps Lock)
    /// - Other actions: only call super when auto-cap is on
    override public func tryChangeKeyboardCase(
        after gesture: Keyboard.Gesture,
        on action: KeyboardAction,
    ) {
        let beforeCase = keyboardContext.keyboardCase
        let isAutoCap = keyboardContext.settings.isAutocapitalizationEnabled

        logger.debug("[CASE][tryChange] gesture=\(String(describing: gesture)) action=\(String(describing: action)) before=\(String(describing: beforeCase)) isAutoCap=\(isAutoCap)")

        // Shift: always let super handle (preserves doubleTap → Caps Lock)
        if case .shift = action {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after shift: \(String(describing: keyboardContext.keyboardCase))")
            return
        }

        // Other actions: only call super when auto-cap is on
        if isAutoCap {
            super.tryChangeKeyboardCase(after: gesture, on: action)
            logger.debug("[CASE][tryChange] after autoCap: \(String(describing: keyboardContext.keyboardCase))")
        } else {
            logger.debug("[CASE][tryChange] skipped (autoCap=false)")
        }
    }
}

// MARK: - AutocompleteContextUpdater

extension ActionHandler: AutocompleteContextUpdater {
    /// Engine-side predictions arrive here and are mapped to KeyboardKit
    /// `Autocomplete.Suggestion` values. This is the only place the
    /// engine's `RustEngineBridge.NextWordEnginePrediction` touches
    /// KeyboardKit types.
    func setNextWordPredictions(_ predictions: [RustEngineBridge.NextWordEnginePrediction]) {
        let suggestions = predictions.map { prediction in
            Autocomplete.Suggestion(
                text: prediction.text,
                title: prediction.text,
                subtitle: prediction.subtitle,
                additionalInfo: [
                    "isNextWord": "true",
                    "hanzi": prediction.hanzi,
                    // Raw TL sidechannel — NOT the commit string. Consumed only
                    // by the association-recording fork in
                    // `handleSuggestionSelection` (engine's `pojToTL` needs raw
                    // roman, not the mode-shaped display `text`).
                    "tl": prediction.tl,
                    "displayText": prediction.hanzi,
                ],
            )
        }
        keyboardController?.state.autocompleteContext.suggestionsFromService = suggestions
    }

    func resetNextWordSuggestions() {
        keyboardController?.state.autocompleteContext.reset()
    }
}
