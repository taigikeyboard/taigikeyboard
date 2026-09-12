// Main view controller for the keyboard extension — owns the KeyboardKit
// lifecycle, input-field detection, and the KeyboardKit 10 auto-cap workaround.
// Other concerns live in the KeyboardViewController+*.swift extensions.

import Combine
import KeyboardKit
import SwiftUI

// Keyboard extension entry point — also acts as ComposingDelegate, dispatching
// effects to textDocumentProxy.
class KeyboardViewController: KeyboardInputViewController, ComposingDelegate {
    // MARK: - Properties

    let logger = DebugLogger(category: "KeyboardViewController")

    /// DI seam. Default retains existing process-wide singleton behavior;
    /// tests (and future composition roots) can substitute a stub.
    let keyboardSettings: any KeyboardEnvironment = SharedSettings.shared

    var emojiServiceStorage: EmojiService?
    weak var actionHandler: ActionHandler?
    var isCleanedUp = false

    private var cancellables = Set<AnyCancellable>()

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// These two properties are part of a 2-layer workaround:
    /// - Layer 1: textDidChangeAsync override (this file) — skips super when auto-cap off
    /// - Layer 2: setupKeyboardCaseProtection (this file) — Combine guard for internal path
    /// Also: tryChangeKeyboardCase override (ActionHandler.swift) — blocks non-shift case changes
    /// 2026-08-28: the standard `setupKeyboardKit(for:)` path (KK 10.9.0) did NOT
    /// fix the symptom on device in this app (benchmark result not reproduced) —
    /// workaround restored. Do NOT remove again without a passing on-device
    /// dogfood of the standard path in THIS app.
    private var expectedKeyboardCase: Keyboard.KeyboardCase = .lowercased
    private var justSwitchedToAlphabetic = false

    /// Previous values for change detection in syncSettings()
    var lastInputMode: InputMode?
    var lastKeyboardLayoutType: KeyboardLayoutType?

    /// Previous resolved key-height scale. Row height lives in the layout, which
    /// is built once in `createKeyboardView`; when the active/edited theme changes
    /// it, `syncSettings()` rebuilds the keyboard view so the layout recomputes.
    /// Colors / font / corner already update in place via `TaigiKeyboardView`.
    var lastResolvedKeyHeightScale: Double?

    /// Identity of the most recent `UITextInput` seen by `textWillChange`.
    /// Pointer-equality detects field switches without touching the iOS 26
    /// SDK's broken `documentIdentifier` UUID bridge.
    private var lastTextInputID: ObjectIdentifier?

    var emojiService: EmojiService {
        if emojiServiceStorage == nil {
            emojiServiceStorage = EmojiService()
            emojiServiceStorage?.delegate = self
        }
        return emojiServiceStorage!
    }

    // MARK: - Initialization

    // TODO: Migrate actionHandler/emojiService to weak refs,
    // so deinit only handles logical state reset (composing/markedText/autocomplete).
    deinit {
        performCleanup()
    }

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()

        // Install the shared-core logging backend so engine-layer code
        // routes logs through DebugLogger.
        LoggerFactory.install { DebugLogger(category: $0) }

        // Wire the Rust engine logger sink so Rust `log::warn!` lines reach
        // DebugLogger. Idempotent — main app also calls this in `init`.
        RustEngineBridge.install()

        // Install the lexicon engine state (fst + dictionary.bin +
        // association.bin) once at extension launch. Idempotent — calling
        // again with the same paths is a no-op observation-wise. Bundle
        // assets are read-only and stable across the keyboard extension's
        // lifetime, so no reinstall is needed within a session.
        installLexiconEngine()

        // Register custom fonts from containing app bundle (extension only)
        FontRegistration.registerFontsIfNeeded()

        setupServices()

        // Observe settings changes (live sync from main app)
        setupSettingsObserver()

        // Guard keyboardCase against KeyboardKit 10 internal path overriding state
        setupKeyboardCaseProtection()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)

        // Update full access status (read by main app's SetupGuide).
        // Guard to avoid unnecessary UserDefaults write → didChangeNotification → double syncSettings().
        let currentFullAccess = hasFullAccess
        if keyboardSettings.isFullAccessEnabled != currentFullAccess {
            keyboardSettings.isFullAccessEnabled = currentFullAccess
        }

        syncSettings()
    }

    override func viewWillSetupKeyboardView() {
        setupKeyboardView { [unowned self] _ in
            guard let handler = actionHandler else {
                return AnyView(EmptyView())
            }

            return AnyView(createKeyboardView(
                composingManager: handler.composingManager,
            ))
        }
    }

    private func createKeyboardView(
        composingManager: ComposingManager,
    ) -> some View {
        let layoutService = CustomLayoutService()
        let layout = layoutService.keyboardLayout(for: state.keyboardContext)

        return TaigiKeyboardView(
            settings: keyboardSettings,
            services: services,
            layout: layout,
            emojiKeyboardView: { [unowned self] in
                emojiService.emojiKeyboardView
            },
            calloutStyle: .taigi(for: keyboardSettings.fontType),
            autocompleteContext: state.autocompleteContext,
            keyboardContext: state.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { [unowned self] suggestion in
                services.actionHandler.handle(suggestion)
            },
            onTranslateToggle: { [unowned self] in
                state.keyboardContext.toggleTranslateSwapped()
            },
            onCandidateDisplayModeChange: { [unowned self] mode in
                state.keyboardContext.candidateDisplayMode = mode
                // Not display-only: under 羅馬字 the ENGINE collapses same-roman
                // rows (§44), so the open list is fetched again, not repainted.
                performAutocomplete()
            },
        )
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        performCleanup()
    }

    // MARK: - Autocomplete

    /// Prefer rawInput over composingText for autocomplete:
    /// rawInput keeps tone digits ("Soo1") for Trie lookup;
    /// composingText is display-only ("Soo", tone 1/4 have no diacritics).
    override var autocompleteText: String? {
        if let handler = actionHandler, handler.composingManager.isComposing {
            return handler.composingManager.rawInput
        }
        return super.autocompleteText
    }

    // MARK: - Text Input Change

    override func textDidChange(_ textInput: UITextInput?) {
        super.textDidChange(textInput)
        actionHandler?.nextWordController.resetAndClearUI()
    }

    /// v3.5.4 lifecycle (plan §4.2): bump the composing engine's
    /// input-context generation on a real field change.
    ///
    /// `selfCommitInProgress` skips the IME's own commits (candidate tap,
    /// self-driven write re-entry). The `ObjectIdentifier` compare detects
    /// genuine field switches via UITextInput class-instance identity —
    /// distinct fields are distinct instances. Same-field textWillChange
    /// fires (typing, selection change) keep the same identity and skip
    /// the bump, preserving an active composing buffer.
    override func textWillChange(_ textInput: UITextInput?) {
        super.textWillChange(textInput)
        guard let manager = actionHandler?.composingManager else { return }
        if manager.selfCommitInProgress {
            return
        }
        let id = textInput.map { ObjectIdentifier($0 as AnyObject) }
        if lastTextInputID == id {
            return
        }
        lastTextInputID = id
        // Real input-field switch — hard-abort the continuous composition
        // (Model B: nailed segments were never in the document, so the
        // generation bump cleanly discards them; `continuous-input-
        // ranking.md` §10.6 external-region-clear = hard abort).
        manager.bumpGeneration()
        // Mirror the composing-slice IME-session bump for the NextWord
        // engine handle so cross-field state (lastSelectedWord / is_showing /
        // current_generation) drops on real input-context changes — Codex
        // post-impl P2-2. Android does the equivalent in
        // `NextWordHandler.resetContext()` invoked from `onStartInputView`.
        actionHandler?.nextWordController.bumpEnvelopeGeneration()
    }

    /// FIXME: Workaround layer 1/2 for KeyboardKit 10 auto-capitalization override.
    /// Skips super's setKeyboardCase(preferredKeyboardCase) when auto-cap is off.
    override func textDidChangeAsync(_ textInput: UITextInput?) {
        let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled
        logger.debug("[CASE][textDidChangeAsync] isAutoCap=\(isAutoCap) keyboardCase=\(String(describing: state.keyboardContext.keyboardCase))")

        if isAutoCap {
            super.textDidChangeAsync(textInput)
        } else {
            performAutocomplete()
        }
    }

    // MARK: - KeyboardCase Protection

    /// FIXME: Workaround layer 2/2 — Combine-based guard against KeyboardKit 10
    /// internally setting keyboardCase = preferredKeyboardCase via a code path that
    /// bypasses our setKeyboardCase/tryChangeKeyboardCase overrides.
    ///
    /// KeyboardKit 10 sets keyboardCase = preferredKeyboardCase via an internal path
    /// when keyboardType switches to alphabetic, bypassing our tryChangeKeyboardCase
    /// and setKeyboardCase overrides. This guard observes keyboardCase changes and
    /// restores the expected state when auto-capitalization is off.
    private func setupKeyboardCaseProtection() {
        // Initialize expected value
        expectedKeyboardCase = state.keyboardContext.keyboardCase

        // Observe keyboardType changes and set the flag
        state.keyboardContext.$keyboardType
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newType in
                guard let self else { return }
                if newType == .alphabetic {
                    justSwitchedToAlphabetic = true
                    logger.debug("[CASE][PROTECT] keyboardType → alphabetic, flag set")
                }
            }
            .store(in: &cancellables)

        // Observe keyboardCase changes and block unexpected mutations
        state.keyboardContext.$keyboardCase
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newCase in
                guard let self else { return }
                let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

                logger.debug("[CASE][PROTECT] newCase=\(String(describing: newCase)) expected=\(String(describing: expectedKeyboardCase)) isAutoCap=\(isAutoCap) justSwitched=\(justSwitchedToAlphabetic)")

                // Block unexpected uppercased when auto-cap is off and just switched to alphabetic
                if !isAutoCap,
                   justSwitchedToAlphabetic,
                   newCase == .uppercased,
                   expectedKeyboardCase != .uppercased,
                   expectedKeyboardCase != .capsLocked
                {
                    logger.debug("[CASE][PROTECT] ⚠️ BLOCKING uppercased, restoring to \(String(describing: expectedKeyboardCase))")
                    // Restore asynchronously to ensure KeyboardKit internal processing completes first
                    let targetCase = expectedKeyboardCase
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        logger.debug("[CASE][PROTECT] async restoring to \(String(describing: targetCase))")
                        state.keyboardContext.keyboardCase = targetCase
                    }
                } else {
                    // Update expected value (legitimate change)
                    expectedKeyboardCase = newCase
                }

                // Clear the flag regardless of whether we blocked
                justSwitchedToAlphabetic = false
            }
            .store(in: &cancellables)
    }

    // MARK: - UIResponder Text Input Overrides

    // Route UIResponder text-input calls (e.g. external/hardware keyboard, system
    // voice input) through the text document proxy so they land in the host doc.
    // ComposingDelegate.execute(_:) operates on the proxy independently — these
    // overrides are for UIResponder-chain callers, not for the composing engine.

    override func insertText(_ text: String) {
        textDocumentProxy.insertText(text)
    }

    override func deleteBackward() {
        textDocumentProxy.deleteBackward()
    }

    // MARK: - Settings Observer

    /// Observe main app settings via App Group UserDefaults → syncSettings()
    private func setupSettingsObserver() {
        NotificationCenter.default.publisher(
            for: UserDefaults.didChangeNotification,
            object: keyboardSettings.settingsUserDefaults,
        )
        .receive(on: DispatchQueue.main)
        .sink { [weak self] _ in
            self?.syncSettings()
        }
        .store(in: &cancellables)
    }
}
