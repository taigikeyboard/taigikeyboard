import KeyboardKit
import SwiftUI

// MARK: - Setup & Configuration

extension KeyboardViewController {
    /// 設置所有服務
    func setupServices() {
        KeyboardModels.setupDeviceConfiguration()
        setupLiquidGlass() // 設置 Liquid Glass 支援
        setupCoreServices() // 只設置核心服務
    }

    /// 設置 Liquid Glass 支援（使用 KeyboardKit 內建功能）
    func setupLiquidGlass() {
        let context = state.keyboardContext

        // 啟用 KeyboardKit 的 Liquid Glass 支援
        if context.isLiquidGlassAvailable {
            context.setIsLiquidGlassEnabled(true)
        } else {
            // 可選：在較舊版本上強制啟用以預覽效果
            // context.setIsLiquidGlassEnabled(true)
        }
    }

    /// 設置核心服務（立即需要的服務）
    func setupCoreServices() {
        // 1. 先設置 LayoutService 和 StyleService（必須在視圖建構前）
        services.layoutService = CustomLayoutService()
        services.styleService = CustomStyleService(keyboardContext: state.keyboardContext)

        // 2. 設置 AutocompleteContext 配置（不創建服務實例）
        let autocompleteContext = state.autocompleteContext
        autocompleteContext.settings.suggestionsDisplayCount = 100

        // 3. 核心 ActionHandler（使用延遲初始化的其他服務）
        let handler = ActionHandler(
            controller: self,
            keyboardContext: state.keyboardContext,
            keyboardBehavior: services.keyboardBehavior, // lazy var - 按需創建
            autocompleteContext: state.autocompleteContext,
            autocompleteService: services.autocompleteService, // lazy var - 按需創建
            emojiContext: state.emojiContext,
            feedbackContext: state.feedbackContext,
            feedbackService: services.feedbackService, // lazy var - 按需創建
            spaceDragGestureHandler: services.spaceDragGestureHandler, // lazy var - 按需創建
        )

        services.actionHandler = handler
        actionHandler = handler
        handler.keyboardViewController = self

        // 設定 ComposingManager 的 KeyboardContext 與 KeyboardViewController
        handler.composingManager.setKeyboardContext(state.keyboardContext)
        handler.composingManager.setKeyboardViewController(self)
    }

    /// 設置 UI 相關服務（首次顯示時需要）
    func setupUIServices() {
        // 設置低靈敏度的空白鍵拖拽手勢處理器
        services.spaceDragGestureHandler = .spaceDrag(
            sensitivity: .low,
            action: { [weak self] offset in
                guard let self else { return }
                let context = self.state.keyboardContext
                let isLtr = context.locale.isLeftToRight
                let adjustedOffset = isLtr ? offset : -offset
                self.adjustTextPosition(by: adjustedOffset)
            }
        )
    }

    /// 確保關鍵服務已初始化（遵循 KeyboardKit 標準模式）
    func ensureEssentialServicesInitialized() {
        // 觸發 KeyboardKit 的 lazy var 初始化，確保每個實例都有完整服務
        // 這遵循 KeyboardKit 的設計：服務綁定到實例，而非全域狀態

        // 設置自定義 AutocompleteService（替換預設的 .disabled）
        let autocompleteService = AutocompleteService()
        services.autocompleteService = autocompleteService

        // 連接 AutocompleteService 和 ComposingManager
        if let handler = actionHandler {
            autocompleteService.setComposingManager(handler.composingManager)
        }

        // 觸發其他關鍵服務的 lazy 初始化（如果尚未初始化）
        _ = services.keyboardBehavior // 確保行為服務可用
        _ = services.feedbackService // 確保回饋服務可用
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

    /// 同步設定
    func syncSettings() {
        let settings = SharedSettings.shared
        settings.syncToKeyboardContext(state.keyboardContext)
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
