import Combine
import KeyboardKit
import SwiftUI

class KeyboardViewController: KeyboardInputViewController, ComposingDelegate {
    // MARK: - Properties

    let logger = DebugLogger(category: "KeyboardViewController")

    var emojiServiceStorage: EmojiService?
    weak var actionHandler: ActionHandler?
    var isCleanedUp = false

    private var cancellables = Set<AnyCancellable>()

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// These two properties are part of a 2-layer workaround:
    /// - Layer 1: textDidChangeAsync override (this file) — skips super when auto-cap off
    /// - Layer 2: setupKeyboardCaseProtection (this file) — Combine guard for internal path
    /// Also: tryChangeKeyboardCase override (ActionHandler.swift) — blocks non-shift case changes
    /// Remove when KeyboardKit provides a proper API to disable auto-capitalization.
    private var expectedKeyboardCase: Keyboard.KeyboardCase = .lowercased
    private var justSwitchedToAlphabetic = false

    /// Previous values for change detection in syncSettings()
    var lastInputMode: InputMode?
    var lastKeyboardLayoutType: KeyboardLayoutType?

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

        // Install the shared-core logging backend so engine candidates
        // (CandidateProcessor / InputNormalizer) route logs through DebugLogger.
        LoggerFactory.install { DebugLogger(category: $0) }

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
        if SharedSettings.shared.isFullAccessEnabled != currentFullAccess {
            SharedSettings.shared.isFullAccessEnabled = currentFullAccess
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
            services: services,
            layout: layout,
            emojiKeyboardView: { [unowned self] in
                emojiService.emojiKeyboardView
            },
            calloutStyle: createCalloutStyle(),
            autocompleteContext: state.autocompleteContext,
            keyboardContext: state.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { [unowned self] suggestion in
                services.actionHandler.handle(suggestion)
            },
            onTranslateToggle: { [unowned self] in
                state.keyboardContext.toggleTranslateSwapped()
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

    // MARK: - ComposingDelegate (overrides must be in class body)

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
            object: SharedSettings.sharedUserDefaults,
        )
        .receive(on: DispatchQueue.main)
        .sink { [weak self] _ in
            self?.syncSettings()
        }
        .store(in: &cancellables)
    }
}
