import KeyboardKit
import SwiftUI

// MARK: - Setup & Configuration

private let setupLogger = DebugLogger(category: "KeyboardViewController+Setup")

extension KeyboardViewController {
    /// 設置所有服務
    func setupServices() {
        // 設定 KeyboardKit 使用 App Group 持久化設定
        // 必須在任何 KeyboardSettings 存取之前呼叫
        KeyboardSettings.setupStore(forAppGroup: SharedSettings.appGroupId)

        // One-time keyboard context config (constant, never changes)
        state.keyboardContext.settings.spacebarLongPressBehavior = .moveInputCursor

        setupLiquidGlass() // 設置 Liquid Glass 支援
        setupCoreServices() // 只設置核心服務
    }

    /// 設置 Liquid Glass 支援（使用 KeyboardKit 內建功能）
    func setupLiquidGlass() {
        let context = state.keyboardContext

        // 啟用 KeyboardKit 的 Liquid Glass 支援
        if context.isLiquidGlassAvailable {
            context.isLiquidGlassEnabled = true
        }
    }

    /// 設置核心服務（立即需要的服務）
    func setupCoreServices() {
        // 1. 設置 AutocompleteContext 配置
        state.autocompleteContext.settings.suggestionsDisplayCount = 100

        // 2. 建立正確的 AutocompleteService（必須在 ActionHandler 之前，
        //    否則 ActionHandler 內部會持有 KeyboardKit 預設 service 的引用）
        setupAutocompleteServiceForCurrentMode()

        // 3. 核心 ActionHandler — 取得正確的 autocompleteService
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

        // 4. 連結台語 AutocompleteService 與 handler（需要 handler 已建立）
        if let taigiService = services.autocompleteService as? AutocompleteService {
            taigiService.setComposingManager(handler.composingManager)
            taigiService.setActionHandler(handler)
        }

        // 5. 初始化追蹤變數，避免 syncSettings() 首次呼叫時誤判為「改變了」
        let settings = SharedSettings.shared
        lastInputMode = settings.inputMode
        lastKeyboardLayoutType = settings.keyboardLayoutType
    }

    /// 根據當前輸入模式設置對應的 AutocompleteService
    ///
    /// Called at initial setup and from syncSettings() when input mode changes.
    func setupAutocompleteServiceForCurrentMode() {
        let settings = SharedSettings.shared

        if settings.inputMode == .english {
            services.autocompleteService = EnglishAutocompleteService()
            setupLogger.debug("[AUTOCOMPLETE] Using EnglishAutocompleteService")
        } else {
            let autocompleteService = AutocompleteService()
            services.autocompleteService = autocompleteService

            // 連接 AutocompleteService 和 ComposingManager / ActionHandler
            if let handler = actionHandler {
                autocompleteService.setComposingManager(handler.composingManager)
                autocompleteService.setActionHandler(handler)
            }
            setupLogger.debug("[AUTOCOMPLETE] Using TaigiAutocompleteService for mode: \(settings.inputMode.rawValue)")
        }
        // Note: KeyboardKit's services.autocompleteService didSet automatically
        // syncs handler.autocompleteService — no manual sync needed.
    }

    /// 同步設定
    ///
    /// 當主 App 變更設定時，透過 UserDefaults.didChangeNotification 觸發此方法。
    /// 需要手動重新讀取 KeyboardKit 的設定，因為 @AppStorage 的 didSet
    /// 不會被外部進程的變更觸發。
    func syncSettings() {
        let settings = SharedSettings.shared
        var needsAutocompleteReset = false

        // 檢查輸入模式是否變更，若變更則重新設置 AutocompleteService
        let currentInputMode = settings.inputMode
        if lastInputMode != currentInputMode {
            let previousMode = lastInputMode?.rawValue ?? "nil"
            setupLogger.debug("[SETTINGS] InputMode changed: \(previousMode) -> \(currentInputMode.rawValue)")
            lastInputMode = currentInputMode
            setupAutocompleteServiceForCurrentMode()
            needsAutocompleteReset = true
        }

        // 檢查佈局類型是否變更
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

    /// 建立 Callout 樣式
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
