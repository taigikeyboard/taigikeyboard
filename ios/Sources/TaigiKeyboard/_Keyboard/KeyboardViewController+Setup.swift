import KeyboardKit
import OSLog
import SwiftUI

// MARK: - Setup & Configuration

private let setupLogger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "KeyboardViewController+Setup"
)

extension KeyboardViewController {
    /// 設置所有服務
    func setupServices() {
        // 設定 KeyboardKit 使用 App Group 持久化設定
        // 必須在任何 KeyboardSettings 存取之前呼叫
        KeyboardSettings.setupStore(forAppGroup: SharedSettings.appGroupId)

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
        // KeyboardKit 10: layoutService 已移除，改用 KeyboardView(layout:) 直接傳入
        // Layout 的建構移至 TaigiKeyboardView

        // 1. 設置 AutocompleteContext 配置（不創建服務實例）
        let autocompleteContext = state.autocompleteContext
        autocompleteContext.settings.suggestionsDisplayCount = 100

        // 2. 核心 ActionHandler（使用延遲初始化的其他服務）
        let handler = ActionHandler(
            controller: self,
            keyboardContext: state.keyboardContext,
            keyboardBehavior: services.keyboardBehavior, // lazy var - 按需創建
            autocompleteContext: state.autocompleteContext,
            autocompleteService: services.autocompleteService, // lazy var - 按需創建
            emojiContext: state.emojiContext,
            feedbackContext: state.feedbackContext,
            feedbackService: services.feedbackService, // lazy var - 按需創建
            spacebarDragGestureHandler: services.spacebarDragGestureHandler // lazy var - 按需創建
        )

        services.actionHandler = handler
        actionHandler = handler
        handler.keyboardViewController = self

        // 設定 ComposingManager 的 KeyboardContext 與 KeyboardViewController
        handler.composingManager.setKeyboardContext(state.keyboardContext)
        handler.composingManager.setKeyboardViewController(self)
    }

    /// 確保關鍵服務已初始化（遵循 KeyboardKit 標準模式）
    func ensureEssentialServicesInitialized() {
        // 觸發 KeyboardKit 的 lazy var 初始化，確保每個實例都有完整服務
        // 這遵循 KeyboardKit 的設計：服務綁定到實例，而非全域狀態

        // 根據輸入模式設置對應的 AutocompleteService
        setupAutocompleteServiceForCurrentMode()

        // 觸發其他關鍵服務的 lazy 初始化（如果尚未初始化）
        _ = services.keyboardBehavior // 確保行為服務可用
        _ = services.feedbackService // 確保回饋服務可用
    }

    /// 根據當前輸入模式設置對應的 AutocompleteService
    func setupAutocompleteServiceForCurrentMode() {
        let settings = SharedSettings.shared

        if settings.inputMode == .english {
            // 英文模式：使用 EnglishAutocompleteService
            let englishService = EnglishAutocompleteService()
            services.autocompleteService = englishService
            setupLogger.debug("[AUTOCOMPLETE] Using EnglishAutocompleteService")
        } else {
            // 台語模式（POJ/TL）：使用 AutocompleteService
            let autocompleteService = AutocompleteService()
            services.autocompleteService = autocompleteService

            // 連接 AutocompleteService 和 ComposingManager / ActionHandler
            if let handler = actionHandler {
                autocompleteService.setComposingManager(handler.composingManager)
                autocompleteService.setActionHandler(handler)
            }
            setupLogger.debug("[AUTOCOMPLETE] Using TaigiAutocompleteService for mode: \(settings.inputMode.rawValue, privacy: .public)")
        }
    }

    /// 確保新實例啟動時有乾淨的狀態
    func ensureCleanState() {
        // 重置 AutocompleteContext（清除任何殘留候選詞）
        state.autocompleteContext.reset()

        // 重置 ComposingManager（如果已初始化）
        if let handler = actionHandler {
            handler.composingManager.reset()
        }

        // 確保 TextDocumentProxy 沒有殘留的 markedText
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()
    }

    /// 記錄上次的輸入模式，用於偵測變更
    private static var lastInputMode: InputMode?
    /// 記錄上次的佈局類型，用於偵測變更
    private static var lastKeyboardLayoutType: KeyboardLayoutType?

    /// 同步設定
    ///
    /// 當主 App 變更設定時，透過 UserDefaults.didChangeNotification 觸發此方法。
    /// 需要手動重新讀取 KeyboardKit 的設定，因為 @AppStorage 的 didSet
    /// 不會被外部進程的變更觸發。
    func syncSettings() {
        let settings = SharedSettings.shared
        settings.syncToKeyboardContext(state.keyboardContext)

        // 檢查輸入模式是否變更，若變更則重新設置 AutocompleteService
        let currentInputMode = settings.inputMode
        if Self.lastInputMode != currentInputMode {
            setupLogger.debug("[SETTINGS] InputMode changed: \(Self.lastInputMode?.rawValue ?? "nil", privacy: .public) -> \(currentInputMode.rawValue, privacy: .public)")
            Self.lastInputMode = currentInputMode
            setupAutocompleteServiceForCurrentMode()

            // 清除候選詞（避免顯示舊模式的候選詞）
            state.autocompleteContext.reset()
        }

        // 檢查佈局類型是否變更，若變更則觸發鍵盤佈局重建
        let currentLayoutType = settings.keyboardLayoutType
        if Self.lastKeyboardLayoutType != currentLayoutType {
            setupLogger.debug("[SETTINGS] LayoutType changed: \(Self.lastKeyboardLayoutType.map { String(describing: $0) } ?? "nil", privacy: .public) -> \(String(describing: currentLayoutType), privacy: .public)")
            Self.lastKeyboardLayoutType = currentLayoutType
            state.autocompleteContext.reset()
        }

        // 讀取 KeyboardKit 持久化設定（直接從 UserDefaults）
        let persistedValue = KeyboardSettings.store.bool(
            forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
        )
        // 讀取 KeyboardContext.settings 的值（@AppStorage）
        let contextValue = state.keyboardContext.settings.isAutocapitalizationEnabled

        setupLogger.debug("[AUTOCAP][SYNC] persisted=\(persistedValue, privacy: .public) context=\(contextValue, privacy: .public) keyboardCase=\(String(describing: self.state.keyboardContext.keyboardCase), privacy: .public)")

        // 重新讀取 KeyboardKit 的自動大寫設定
        // @AppStorage 會讀取最新值，但 didSet 不會被觸發
        // 所以需要手動設定 autocapitalizationTypeOverride
        // Only write if different from current value to avoid triggering re-renders
        if contextValue {
            if state.keyboardContext.autocapitalizationTypeOverride != nil {
                state.keyboardContext.autocapitalizationTypeOverride = nil
            }
        } else {
            if state.keyboardContext.autocapitalizationTypeOverride != Keyboard.AutocapitalizationType.none {
                state.keyboardContext.autocapitalizationTypeOverride = Keyboard.AutocapitalizationType.none
            }
            // 關閉自動大寫時，重置 keyboardCase 為小寫（Caps Lock 除外）
            if state.keyboardContext.keyboardCase != .capsLocked
                && state.keyboardContext.keyboardCase != .lowercased {
                state.keyboardContext.keyboardCase = .lowercased
            }
        }

        setupLogger.debug("[AUTOCAP][SYNC] override=\(String(describing: self.state.keyboardContext.autocapitalizationTypeOverride), privacy: .public) keyboardCase=\(String(describing: self.state.keyboardContext.keyboardCase), privacy: .public)")
    }

    /// 建立 Callout 樣式
    func createCalloutStyle() -> Callouts.CalloutStyle {
        let fontType = SharedSettings.shared.fontType

        switch fontType {
        case .system:
            return Callouts.CalloutStyle.standard
        case .openHuninn:
            let fontName = KeyboardModels.Fonts.openHuninnFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light)
            )
        case .iansui:
            let fontName = KeyboardModels.Fonts.iansuiFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light)
            )
        }
    }
}
