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

        handler.composingManager.setKeyboardContext(state.keyboardContext)
        handler.composingManager.delegate = self
        handler.nextWordController.contextUpdater = handler

        // 4. Connect Taigi AutocompleteService with handler (requires handler already created)
        wireTaigiAutocompleteProviders(from: services.autocompleteService, to: handler)

        // 5. Initialize tracking vars so syncSettings() doesn't false-trigger on first call
        let settings = SharedSettings.shared
        lastInputMode = settings.inputMode
        lastKeyboardLayoutType = settings.keyboardLayoutType
    }

    /// Called at initial setup and from syncSettings() when input mode changes.
    func setupAutocompleteServiceForCurrentMode() {
        let settings = SharedSettings.shared

        if settings.inputMode == .english {
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
            setupLogger.debug("[AUTOCOMPLETE] Using TaigiAutocompleteService for mode: \(settings.inputMode.rawValue)")
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
        let settings = SharedSettings.shared
        var needsAutocompleteReset = false

        // Check if input mode changed; if so, recreate AutocompleteService
        let currentInputMode = settings.inputMode
        if lastInputMode != currentInputMode {
            let previousMode = lastInputMode?.rawValue ?? "nil"
            setupLogger.debug("[SETTINGS] InputMode changed: \(previousMode) -> \(currentInputMode.rawValue)")
            lastInputMode = currentInputMode
            setupAutocompleteServiceForCurrentMode()
            needsAutocompleteReset = true
        }

        // Check if keyboard layout type changed
        let currentLayoutType = settings.keyboardLayoutType
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
        guard let fontName = SharedSettings.shared.fontType.customFontName else {
            return Callouts.CalloutStyle.standard
        }

        return Callouts.CalloutStyle(
            actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
            inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light),
        )
    }
}
