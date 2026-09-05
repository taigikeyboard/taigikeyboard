// Keyboard-extension setup: installs the lexicon engine, wires ActionHandler / AutocompleteService,
// and re-syncs services when settings change.

import KeyboardKit
import SwiftUI
import UIKit

// MARK: - Setup & Configuration

private let setupLogger = DebugLogger(category: "KeyboardViewController+Setup")

extension KeyboardViewController {
    func setupServices() {
        // Must be called before any KeyboardSettings access. Legacy setup path
        // kept on purpose: the standard setupKeyboardKit(for:) migration did
        // NOT fix the auto-cap symptom on device (2026-08-28) — see the
        // FIXME layers in KeyboardViewController.
        KeyboardSettings.setupStore(for: .taigiKeyboard)

        state.keyboardContext.settings.spacebarLongPressBehavior = .moveInputCursor

        setupLiquidGlass()
        setupCoreServices()
    }

    /// Install the Rust shared-core lexicon engine state once at extension
    /// launch. Resolves bundle paths via `ResourceBundleResolver`; bundle
    /// assets are read-only + stable across the extension's lifetime so a
    /// single install is sufficient. Failures are logged and left to graceful
    /// degradation at first search call (engine returns
    /// `LexiconError::NotInitialized` → bridge returns `[]`).
    func installLexiconEngine() {
        let bundle = ResourceBundleResolver.dictionaryBundle
        guard
            let fstURL = bundle.url(forResource: "dictionary", withExtension: "fst"),
            let dictBinURL = bundle.url(forResource: "dictionary", withExtension: "bin"),
            let assocBinURL = bundle.url(forResource: "association", withExtension: "bin")
        else {
            setupLogger.warning("[LEXICON] missing bundle resource(s); engine not installed")
            return
        }
        // v3.5.8 Phase 6 — syllables.fst powers continuous-input candidate
        // fetch. Missing → empty path → engine skips inventory load →
        // FetchAtPos returns empty candidates (graceful degrade). Logged
        // explicitly so dogfood notices a missing pbxproj file ref early.
        let syllablesURL = bundle.url(forResource: "syllables", withExtension: "fst")
        if syllablesURL == nil {
            setupLogger.warning(
                "[LEXICON] syllables.fst not found in bundle; "
                    + "continuous-input candidate fetch will return empty",
            )
        }
        let stamp = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(UInt32.init) ?? 1
        if let stats = RustEngineBridge.lexiconInstall(
            triePath: fstURL.path,
            dictionaryBinPath: dictBinURL.path,
            associationBinPath: assocBinURL.path,
            dictionaryVersion: stamp,
            syllableInventoryPath: syllablesURL?.path ?? "",
        ) {
            setupLogger.info(
                "[LEXICON] installed: dict=\(stats.dictionaryRecordCount) "
                    + "fst_entries=\(stats.prefixIndexEntryCount) version=\(stamp) "
                    + "syllables=\(syllablesURL == nil ? "absent" : "present")",
            )
        } else {
            setupLogger.warning("[LEXICON] install returned nil; engine not installed")
        }
    }

    func setupLiquidGlass() {
        let context = state.keyboardContext
        if context.isLiquidGlassAvailable {
            context.isLiquidGlassEnabled = true
        }
    }

    /// Order matters: AutocompleteService → ActionHandler → link them together.
    func setupCoreServices() {
        // 1. Configure AutocompleteContext
        state.autocompleteContext.settings.suggestionsDisplayCount = 100

        // 2. Create the correct AutocompleteService (must be before ActionHandler,
        //    otherwise ActionHandler internally holds a reference to KeyboardKit's default service)
        setupAutocompleteServiceForCurrentMode()

        // 3. Core ActionHandler — receives the correct autocompleteService
        let handler = ActionHandler(
            controller: self,
            keyboardContext: state.keyboardContext,
            keyboardBehavior: services.keyboardBehavior,
            autocompleteContext: state.autocompleteContext,
            autocompleteService: services.autocompleteService,
            emojiContext: state.emojiContext,
            feedbackContext: state.feedbackContext,
            feedbackService: services.feedbackService,
            keyboardAppContext: state.appContext,
            spacebarDragGestureHandler: services.spacebarDragGestureHandler,
        )

        services.actionHandler = handler
        actionHandler = handler

        handler.composingManager.setContextSink(state.keyboardContext)
        handler.composingManager.delegate = self
        handler.nextWordController.contextUpdater = handler

        // 4. Connect TaigiAutocompleteService with handler (requires handler already created)
        wireTaigiAutocompleteProviders(from: services.autocompleteService, to: handler)

        // 5. Best-effort warmup of the two SYNCHRONOUS eager-empty hot-path
        //    user-data DBs that `fetchContinuousCandidates` reads:
        //    `user_frequency.db` (boost — `UserFrequencyService.frequencyDataBatch`)
        //    and `custom_dictionary.db` (custom candidates —
        //    `CustomDictionaryRepository.searchSync`). Both readers return `[]`
        //    until their connection is open and NEVER lazy-open (the Continuous
        //    fetch is synchronous and must not block on an async DB open), so
        //    without an eager warmup here a fresh session would ignore them
        //    indefinitely until the user committed something. NextWord's
        //    `user_association.db` is intentionally NOT warmed here — its reads
        //    are async and lazy-init per query (`ensureUserTablesCreated`).
        //    A future third synchronous eager-empty reader MUST be warmed here.
        //
        //    Fire-and-forget — `fetchContinuousCandidates` keeps its
        //    `isConnected()` cold-start guard for the race window before these
        //    Tasks land. Warning log on failure is intentional for
        //    observability (Codex PR #265 r3216760651 post-impl R5).
        //
        //    The custom_dictionary.db warmup is UNGATED (not behind
        //    `isCustomDictEnabled`): the lookup is already gated in
        //    `ComposingManager.buildCustomEntries`, and an ungated warmup keeps
        //    the connection ready for a live settings toggle (enable in the
        //    host app → works in an already-running extension, no relaunch).
        //    Regression guard: PR #279 (Item 13) deleted the old
        //    `LexiconService` fallback that lazy-opened this DB but only kept
        //    the user-freq warmup, so custom-dict candidates silently vanished
        //    from the keyboard (behavioral-invariants.md §26).
        let userFrequencyService = CompositionRoot.userFrequencyService
        Task {
            do {
                try await userFrequencyService.ensureInitialized()
                setupLogger.info("[INIT] User frequency DB warmed")
            } catch {
                setupLogger.warning(
                    "[INIT] User frequency DB warmup failed: \(error.localizedDescription)",
                )
            }
        }
        let customDictionaryRepository = CompositionRoot.customDictionaryRepository
        Task {
            do {
                try await customDictionaryRepository.ensureInitialized()
                setupLogger.info("[INIT] Custom dictionary DB warmed")
            } catch {
                setupLogger.warning(
                    "[INIT] Custom dictionary DB warmup failed: \(error.localizedDescription)",
                )
            }
        }

        // 6. Initialize tracking vars so syncSettings() doesn't false-trigger on first call
        lastInputMode = keyboardSettings.inputMode
        lastKeyboardLayoutType = keyboardSettings.keyboardLayoutType
        lastResolvedKeyHeightScale = keyboardSettings
            .resolvedAppearance(for: state.keyboardContext.colorScheme).keyHeightScale
    }

    /// Called at initial setup and from syncSettings() when input mode changes.
    func setupAutocompleteServiceForCurrentMode() {
        if keyboardSettings.inputMode == .english {
            services.autocompleteService = EnglishAutocompleteService()
            setupLogger.debug("[AUTOCOMPLETE] Using EnglishAutocompleteService")
        } else {
            let autocompleteService = TaigiAutocompleteService()
            services.autocompleteService = autocompleteService

            // Settings-change path: the handler already exists, so wire the provider now.
            // Initial-build path: setupCoreServices() wires it once the handler is built.
            if let handler = actionHandler {
                wireTaigiAutocompleteProviders(from: autocompleteService, to: handler)
            }
            setupLogger.debug("[AUTOCOMPLETE] Using TaigiAutocompleteService for mode: \(keyboardSettings.inputMode.rawValue)")
        }
        // Note: KeyboardKit's services.autocompleteService didSet automatically
        // syncs handler.autocompleteService — no manual sync needed.
    }

    /// Connects the Taigi composing provider to `TaigiAutocompleteService`. Called from
    /// `setupCoreServices()` once the handler exists, and again from
    /// `setupAutocompleteServiceForCurrentMode()` when a settings change rebuilds the service.
    /// Non-Taigi services such as `EnglishAutocompleteService` are skipped.
    func wireTaigiAutocompleteProviders(
        from service: any KeyboardKit.AutocompleteService,
        to handler: ActionHandler,
    ) {
        guard let taigiService = service as? TaigiAutocompleteService else { return }
        taigiService.setComposingManager(handler.composingManager)
    }

    /// Re-read settings from App Group UserDefaults.
    /// @AppStorage didSet doesn't fire for changes from an external process,
    /// so this is triggered via UserDefaults.didChangeNotification.
    func syncSettings() {
        var needsAutocompleteReset = false

        // Check if input mode changed; if so, recreate AutocompleteService
        let currentInputMode = keyboardSettings.inputMode
        if lastInputMode != currentInputMode {
            let previousMode = lastInputMode?.rawValue ?? "nil"
            setupLogger.debug("[SETTINGS] InputMode changed: \(previousMode) -> \(currentInputMode.rawValue)")
            // v3.5.8 Phase 7B (Codex Fork F) — clear stale Continuous-input
            // state before swapping the AutocompleteService. POJ↔TL↔TPS uses
            // distinct Phase::Continuous { raw } byte conventions, so leftover
            // pending bytes would mis-align consumed-span offsets returned by
            // the new mode's FetchAtPos calls. `bumpGeneration` then causes
            // any in-flight engine call to be silently dropped at the FFI
            // boundary (`engine/composing/src/handle.rs:61-65`).
            actionHandler?.composingManager.resetContinuous()
            actionHandler?.composingManager.bumpGeneration()
            lastInputMode = currentInputMode
            setupAutocompleteServiceForCurrentMode()
            needsAutocompleteReset = true
        }

        // Check if keyboard layout type changed
        let currentLayoutType = keyboardSettings.keyboardLayoutType
        if lastKeyboardLayoutType != currentLayoutType {
            let previousLayout = lastKeyboardLayoutType.map { String(describing: $0) } ?? "nil"
            setupLogger.debug("[SETTINGS] LayoutType changed: \(previousLayout) -> \(String(describing: currentLayoutType))")
            lastKeyboardLayoutType = currentLayoutType
            needsAutocompleteReset = true
        }

        if needsAutocompleteReset {
            state.autocompleteContext.reset()
        }

        // Rebuild the keyboard view when the active/edited theme changes row height.
        // Row height is baked into the layout (built once in createKeyboardView);
        // colors / font / corner update in place via TaigiKeyboardView, but height
        // needs a fresh layout. Gate on the RESOLVED key-height so switching between
        // two same-height themes (or any non-height change) never rebuilds.
        let currentKeyHeightScale = keyboardSettings
            .resolvedAppearance(for: state.keyboardContext.colorScheme).keyHeightScale
        if lastResolvedKeyHeightScale != currentKeyHeightScale {
            setupLogger.debug("[SETTINGS] Key height changed: \(lastResolvedKeyHeightScale.map { String($0) } ?? "nil") -> \(currentKeyHeightScale) — rebuilding keyboard view")
            lastResolvedKeyHeightScale = currentKeyHeightScale
            viewWillSetupKeyboardView()
        }

        // Sync auto-capitalization override from KeyboardKit settings
        let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

        setupLogger.debug("[AUTOCAP][SYNC] isAutoCap=\(isAutoCap) keyboardCase=\(String(describing: state.keyboardContext.keyboardCase))")

        if isAutoCap {
            if state.keyboardContext.autocapitalizationTypeOverride != nil {
                state.keyboardContext.autocapitalizationTypeOverride = nil
            }
        } else {
            if state.keyboardContext.autocapitalizationTypeOverride != Keyboard.AutocapitalizationType.none {
                state.keyboardContext.autocapitalizationTypeOverride = Keyboard.AutocapitalizationType.none
            }
            if state.keyboardContext.keyboardCase != .capsLocked,
               state.keyboardContext.keyboardCase != .lowercased
            {
                state.keyboardContext.keyboardCase = .lowercased
            }
        }
    }

}
