import Combine
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

    /// 設定變更監聯器
    private var settingsObserver: NSObjectProtocol?

    /// Combine subscriptions
    private var cancellables = Set<AnyCancellable>()

    /// 預期的 keyboardCase（用於保護自動大寫關閉時的狀態）
    private var expectedKeyboardCase: Keyboard.KeyboardCase = .lowercased

    /// 標記是否剛切換到 alphabetic 鍵盤（用於保護機制）
    private var justSwitchedToAlphabetic = false

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
            logger.debug("[MEMORY] KeyboardViewController #\(self.instanceId) init (total: \(Self.instanceCount))")
        #endif
    }

    required init?(coder: NSCoder) {
        #if DEBUG
            Self.instanceCount += 1
            instanceId = Self.instanceCount
        #endif
        super.init(coder: coder)
        #if DEBUG
            logger.debug("[MEMORY] KeyboardViewController #\(self.instanceId) init (total: \(Self.instanceCount))")
        #endif
    }

    deinit {
        #if DEBUG
            Self.instanceCount -= 1
            logger.debug("[MEMORY] KeyboardViewController #\(self.instanceId) deinit (remaining: \(Self.instanceCount))")
        #endif
        performCleanup()
    }

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()

        // Register custom fonts from containing app bundle (extension only)
        FontRegistration.registerFontsIfNeeded()

        setupServices()

        // 確保關鍵服務的 lazy var 被觸發，遵循 KeyboardKit 標準模式
        ensureEssentialServicesInitialized()

        // 確保新實例啟動時有乾淨的狀態
        ensureCleanState()

        // 監聽設定變更（從主 App 即時同步）
        setupSettingsObserver()

        // 設定 keyboardCase 保護機制（防止 KeyboardKit 10 內部路徑覆蓋狀態）
        setupKeyboardCaseProtection()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)

        // Write first, so the notification from this write
        // is coalesced with syncSettings changes
        SharedSettings.shared.isFullAccessEnabled = self.hasFullAccess

        syncSettings()
    }

    override func viewWillSetupKeyboardView() {
        setupKeyboardView { (controller: KeyboardInputViewController) in
            guard let keyboardController = controller as? KeyboardViewController else {
                return AnyView(EmptyView())
            }

            return AnyView(keyboardController.createQwertyKeyboardView(controller: controller))
        }
    }

    /// 建立 QWERTY 鍵盤視圖
    private func createQwertyKeyboardView(controller: KeyboardInputViewController) -> some View {
        let layoutService = CustomLayoutService()
        let layout = layoutService.keyboardLayout(for: controller.state.keyboardContext)

        return TaigiKeyboardView(
            services: controller.services,
            layout: layout,
            emojiKeyboardView: { [unowned self] in
                self.emojiService.getEmojiKeyboardView()
            },
            calloutStyle: createCalloutStyle(),
            autocompleteContext: controller.state.autocompleteContext,
            keyboardContext: controller.state.keyboardContext,
            composingManager: (controller.services.actionHandler as! ActionHandler).composingManager,
            onSuggestionTap: { [unowned controller] suggestion in
                controller.services.actionHandler.handle(suggestion)
            },
            onTranslateToggle: { [unowned self] in
                self.toggleTranslateSwap()
            }
        )
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

    // MARK: - Text Input Change

    /// 監聯輸入框切換（textDocumentProxy 變化）
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

    /// 文字變更後的非同步處理
    ///
    /// KeyboardKit 預設會在此調用 `setKeyboardCase(preferredKeyboardCase)`，
    /// 但當自動大寫關閉時，我們只執行 autocomplete，不調整 keyboardCase。
    override func textDidChangeAsync(_ textInput: UITextInput?) {
        let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

        #if DEBUG
        logger.debug("[CASE][textDidChangeAsync] isAutoCap=\(isAutoCap, privacy: .public) keyboardCase=\(String(describing: self.state.keyboardContext.keyboardCase), privacy: .public)")

        // DEBUG: NextWord trace - textDidChangeAsync state
        if let handler = actionHandler {
            logger.debug("[NEXTWORD][textDidChangeAsync] isShowingNextWord=\(handler.isShowingNextWord, privacy: .public) isComposing=\(handler.composingManager.isComposing, privacy: .public) rawInput='\(handler.composingManager.rawInput, privacy: .public)'")
        }
        #endif

        if isAutoCap {
            // 自動大寫開啟：使用 KeyboardKit 預設行為
            super.textDidChangeAsync(textInput)
        } else {
            // 自動大寫關閉：只執行 autocomplete，不調整 keyboardCase
            performAutocomplete()
        }
    }

    // MARK: - KeyboardCase Tracking

    #if DEBUG
    /// 覆寫 setKeyboardCase 來追蹤所有變更來源
    override func setKeyboardCase(_ case: Keyboard.KeyboardCase) {
        let before = state.keyboardContext.keyboardCase
        logger.debug("[CASE][setKeyboardCase] before=\(String(describing: before), privacy: .public) new=\(String(describing: `case`), privacy: .public)")
        super.setKeyboardCase(`case`)
        logger.debug("[CASE][setKeyboardCase] after=\(String(describing: self.state.keyboardContext.keyboardCase), privacy: .public)")
    }
    #endif

    // MARK: - Actions

    @objc func toggleTranslateSwap() {
        state.keyboardContext.toggleTranslateSwapped()
    }

    // MARK: - KeyboardCase Protection

    /// 設定 keyboardCase 保護機制
    ///
    /// KeyboardKit 10 會在 keyboardType 切換到 alphabetic 時，
    /// 透過內部路徑直接設定 keyboardCase = preferredKeyboardCase，
    /// 繞過我們覆寫的 tryChangeKeyboardCase 和 setKeyboardCase。
    /// 這個保護機制會監聽 keyboardCase 變化，在自動大寫關閉時恢復預期的狀態。
    private func setupKeyboardCaseProtection() {
        // 初始化預期值
        expectedKeyboardCase = state.keyboardContext.keyboardCase

        // 監聽 keyboardType 變化，設置標志
        state.keyboardContext.$keyboardType
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newType in
                guard let self = self else { return }
                if newType == .alphabetic {
                    self.justSwitchedToAlphabetic = true
                    #if DEBUG
                    self.logger.debug("[CASE][PROTECT] keyboardType → alphabetic, flag set")
                    #endif
                }
            }
            .store(in: &cancellables)

        // 監聽 keyboardCase 變化，檢查是否需要阻止
        state.keyboardContext.$keyboardCase
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newCase in
                guard let self = self else { return }
                let isAutoCap = self.state.keyboardContext.settings.isAutocapitalizationEnabled

                #if DEBUG
                self.logger.debug("[CASE][PROTECT] newCase=\(String(describing: newCase), privacy: .public) expected=\(String(describing: self.expectedKeyboardCase), privacy: .public) isAutoCap=\(isAutoCap, privacy: .public) justSwitched=\(self.justSwitchedToAlphabetic, privacy: .public)")
                #endif

                // 當自動大寫關閉且剛切換到字母鍵盤時，阻止非預期的 uppercased 變化
                if !isAutoCap
                    && self.justSwitchedToAlphabetic
                    && newCase == .uppercased
                    && self.expectedKeyboardCase != .uppercased
                    && self.expectedKeyboardCase != .capsLocked
                {
                    #if DEBUG
                    self.logger.debug("[CASE][PROTECT] ⚠️ BLOCKING uppercased, restoring to \(String(describing: self.expectedKeyboardCase), privacy: .public)")
                    #endif
                    // 使用異步恢復，確保在 KeyboardKit 內部處理完成後執行
                    let targetCase = self.expectedKeyboardCase
                    DispatchQueue.main.async { [weak self] in
                        guard let self = self else { return }
                        #if DEBUG
                        self.logger.debug("[CASE][PROTECT] async restoring to \(String(describing: targetCase), privacy: .public)")
                        #endif
                        self.state.keyboardContext.keyboardCase = targetCase
                    }
                } else {
                    // 更新預期值（合法的變化）
                    self.expectedKeyboardCase = newCase
                }

                // 清除標志（無論是否阻止，都清除）
                self.justSwitchedToAlphabetic = false
            }
            .store(in: &cancellables)
    }

    // MARK: - Settings Observer

    /// 設定監聯器（監聯主 App 的設定變更）
    private func setupSettingsObserver() {
        #if DEBUG
        logger.debug("[AUTOCAP][SETTINGS] setupSettingsObserver registered")
        #endif

        settingsObserver = NotificationCenter.default.addObserver(
            forName: UserDefaults.didChangeNotification,
            object: SharedSettings.sharedUserDefaults,
            queue: .main
        ) { [weak self] _ in
            #if DEBUG
            self?.logger.debug("[AUTOCAP][SETTINGS] UserDefaults.didChangeNotification received")
            #endif
            self?.syncSettings()
        }
    }

    /// 移除設定監聯器
    func removeSettingsObserver() {
        if let observer = settingsObserver {
            NotificationCenter.default.removeObserver(observer)
            settingsObserver = nil
        }
    }
}
