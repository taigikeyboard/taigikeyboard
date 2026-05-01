import KeyboardKit
import SwiftUI
import UIKit

// MARK: - Setup & Configuration

private let setupLogger = DebugLogger(category: "KeyboardViewController+Setup")

extension KeyboardViewController {
    func setupServices() {
        // Must be called before any KeyboardSettings access
        KeyboardSettings.setupStore(forAppGroup: SharedSettings.appGroupId)

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
        let stamp = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(UInt32.init) ?? 1
        if let stats = RustEngineBridge.lexiconInstall(
            triePath: fstURL.path,
            dictionaryBinPath: dictBinURL.path,
            associationBinPath: assocBinURL.path,
            dictionaryVersion: stamp,
        ) {
            setupLogger.info(
                "[LEXICON] installed: dict=\(stats.dictionaryRecordCount) "
                    + "fst_entries=\(stats.prefixIndexEntryCount) version=\(stamp)"
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
            spacebarDragGestureHandler: services.spacebarDragGestureHandler,
        )

        services.actionHandler = handler
        actionHandler = handler

        handler.composingManager.setContextSink(state.keyboardContext)
        handler.composingManager.delegate = self
        handler.nextWordController.contextUpdater = handler

        // 4. Connect Taigi AutocompleteService with handler (requires handler already created)
        wireTaigiAutocompleteProviders(from: services.autocompleteService, to: handler)

        // 5. Initialize tracking vars so syncSettings() doesn't false-trigger on first call
        lastInputMode = keyboardSettings.inputMode
        lastKeyboardLayoutType = keyboardSettings.keyboardLayoutType
    }

    /// Called at initial setup and from syncSettings() when input mode changes.
    func setupAutocompleteServiceForCurrentMode() {
        if keyboardSettings.inputMode == .english {
            services.autocompleteService = EnglishAutocompleteService()
            setupLogger.debug("[AUTOCOMPLETE] Using EnglishAutocompleteService")
        } else {
            let autocompleteService = AutocompleteService()
            services.autocompleteService = autocompleteService

            // Settings 變更路徑：ActionHandler 已存在，直接以 helper 連線。
            // 初始建構路徑：ActionHandler 尚未建立，setupCoreServices() 會在
            // handler 建好後再呼叫 wireTaigiAutocompleteProviders 補上 provider。
            if let handler = actionHandler {
                wireTaigiAutocompleteProviders(from: autocompleteService, to: handler)
            }
            setupLogger.debug("[AUTOCOMPLETE] Using TaigiAutocompleteService for mode: \(keyboardSettings.inputMode.rawValue)")
        }
        // Note: KeyboardKit's services.autocompleteService didSet automatically
        // syncs handler.autocompleteService — no manual sync needed.
    }

    /// 將 Taigi 專用的 composing / selection provider 接上 AutocompleteService。
    ///
    /// 兩個呼叫路徑共用：
    /// - `setupCoreServices()` 建立 handler 後初始連線
    /// - `setupAutocompleteServiceForCurrentMode()` 在 settings 變更時重建 service 後重新連線
    ///
    /// service 為 `EnglishAutocompleteService` 或其它非 Taigi 實作時，直接略過。
    func wireTaigiAutocompleteProviders(
        from service: any KeyboardKit.AutocompleteService,
        to handler: ActionHandler,
    ) {
        guard let taigiService = service as? AutocompleteService else { return }
        taigiService.setComposingManager(handler.composingManager)
        taigiService.setSelectionContextProvider(handler.nextWordController)
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

    func createCalloutStyle() -> Callouts.CalloutStyle {
        guard let fontName = keyboardSettings.fontType.customFontName else {
            return Callouts.CalloutStyle.standard
        }

        return Callouts.CalloutStyle(
            actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
            inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light),
        )
    }
}
