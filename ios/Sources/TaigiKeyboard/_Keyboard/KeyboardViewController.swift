import KeyboardKit
import OSLog
import SwiftUI

class KeyboardViewController: KeyboardInputViewController {
    // MARK: - Properties

    #if DEBUG
        private static var instanceCount = 0
        private var instanceId: Int
        internal let logger = Logger(
            subsystem: LexiconConstants.Logging.subsystem,
            category: "KeyboardViewController"
        )
    #endif

    var emojiSvc: EmojiService?
    weak var actionHandler: ActionHandler?
    var isCleanedUp = false

    var emojiService: EmojiService {
        if emojiSvc == nil {
            emojiSvc = EmojiService()
            emojiSvc?.delegate = self
        }
        return emojiSvc!
    }

    // MARK: - Initialization

    override init(nibName nibNameOrNil: String?, bundle nibBundleOrNil: Bundle?) {
        #if DEBUG
            Self.instanceCount += 1
            instanceId = Self.instanceCount
        #endif
        super.init(nibName: nibNameOrNil, bundle: nibBundleOrNil)
        #if DEBUG
            logger.info("[MEMORY] KeyboardViewController #\(self.instanceId) init (total: \(Self.instanceCount))")
        #endif
    }

    required init?(coder: NSCoder) {
        #if DEBUG
            Self.instanceCount += 1
            instanceId = Self.instanceCount
        #endif
        super.init(coder: coder)
        #if DEBUG
            logger.info("[MEMORY] KeyboardViewController #\(self.instanceId) init (total: \(Self.instanceCount))")
        #endif
    }

    deinit {
        #if DEBUG
            Self.instanceCount -= 1
            logger.info("[MEMORY] KeyboardViewController #\(self.instanceId) deinit (remaining: \(Self.instanceCount))")
        #endif
        performCleanup()
    }

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()

        setupServices()

        // 確保關鍵服務的 lazy var 被觸發，遵循 KeyboardKit 標準模式
        ensureEssentialServicesInitialized()

        // 確保新實例啟動時有乾淨的狀態
        ensureCleanState()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)

        syncSettings()

        // Phase 2: 設置 UI 服務
        setupUIServices()

        // EmojiService 使用延遲初始化，只在實際需要時創建

        // 同步 Full Access 狀態到 App Group
        SharedSettings.shared.isFullAccessEnabled = self.hasFullAccess
    }

    override func viewWillSetupKeyboardView() {
        setupKeyboardView { (controller: KeyboardInputViewController) in
            guard let keyboardController = controller as? KeyboardViewController else {
                return AnyView(EmptyView())
            }

            return AnyView(TaigiKeyboardView(
                state: controller.state,
                services: controller.services,
                emojiKeyboardView: { [unowned keyboardController] in
                    keyboardController.emojiService.getEmojiKeyboardView()
                },
                calloutStyle: keyboardController.createCalloutStyle(),
                autocompleteContext: controller.state.autocompleteContext,
                keyboardContext: controller.state.keyboardContext,
                composingManager: (controller.services.actionHandler as! ActionHandler).composingManager,
                onSuggestionTap: { [unowned controller] suggestion in
                    controller.services.actionHandler.handle(suggestion)
                },
                onTranslateToggle: { [unowned keyboardController] in
                    keyboardController.toggleTranslateSwap()
                },
                onCollapse: {}
            ))
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        if let handler = actionHandler {
            handler.keyboardViewController = nil
        }

        super.viewWillDisappear(animated)
        performCleanup()
    }

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        performCleanup()
    }

    // MARK: - Autocomplete

    /// 覆寫 autocompleteText 屬性，優先使用 rawInput（搜尋用）
    /// 必須使用 rawInput 而非 composingText，因為：
    /// - rawInput 包含聲調數字（如 "Soo1"），用於 Trie 搜尋
    /// - composingText 是顯示文字（如 "Soo"），聲調 1/4 不加調號
    /// - KeyboardKit 根據此值變化決定是否觸發 autocomplete
    override var autocompleteText: String? {
        // 如果正在組字中，使用 rawInput 作為自動完成的輸入
        if let handler = actionHandler, handler.composingManager.isComposing {
            return handler.composingManager.rawInput
        }
        // 否則使用 KeyboardKit 的預設邏輯
        return super.autocompleteText
    }

    override func performAutocomplete() {
        super.performAutocomplete()
    }

    // MARK: - Text Input Change

    /// 監聽輸入框切換（textDocumentProxy 變化）
    override func textDidChange(_ textInput: UITextInput?) {
        super.textDidChange(textInput)

        // 重置 NextWord 上下文（切換輸入框時）
        if let handler = services.actionHandler as? ActionHandler {
            handler.resetNextWordContext()

            // 如果正在顯示 NextWord，清除候選詞
            if handler.isShowingNextWord {
                state.autocompleteContext.reset()
            }
        }
    }

    // MARK: - Actions

    @objc func toggleTranslateSwap() {
        state.keyboardContext.toggleTranslateSwapped()
    }
}
