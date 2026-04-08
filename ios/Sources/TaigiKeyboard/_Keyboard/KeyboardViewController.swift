import Combine
import KeyboardKit
import SwiftUI

class KeyboardViewController: KeyboardInputViewController {
    // MARK: - Properties

    let logger = DebugLogger(category: "KeyboardViewController")

    var emojiSvc: EmojiService?
    weak var actionHandler: ActionHandler?
    var isCleanedUp = false

    /// 設定變更監聯器
    private var settingsObserver: NSObjectProtocol?

    /// Combine subscriptions
    private var cancellables = Set<AnyCancellable>()

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// These two properties are part of a 3-layer workaround (see also:
    /// - textDidChangeAsync override (this file)
    /// - tryChangeKeyboardCase override (ActionHandler.swift)
    /// - setupKeyboardCaseProtection (this file)
    /// Remove when KeyboardKit provides a proper API to disable auto-capitalization.
    private var expectedKeyboardCase: Keyboard.KeyboardCase = .lowercased
    private var justSwitchedToAlphabetic = false

    /// 記錄上次的輸入模式，用於偵測變更
    var lastInputMode: InputMode?
    /// 記錄上次的佈局類型，用於偵測變更
    var lastKeyboardLayoutType: KeyboardLayoutType?

    var emojiService: EmojiService {
        if emojiSvc == nil {
            emojiSvc = EmojiService()
            emojiSvc?.delegate = self
        }
        return emojiSvc!
    }

    // MARK: - Initialization

    // TODO: Migrate settingsObserver to Combine, actionHandler/emojiService to weak refs,
    // so deinit only handles logical state reset (composing/markedText/autocomplete).
    deinit {
        performCleanup()
    }

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()

        // Register custom fonts from containing app bundle (extension only)
        FontRegistration.registerFontsIfNeeded()

        setupServices()

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
        SharedSettings.shared.isFullAccessEnabled = hasFullAccess

        syncSettings()
    }

    override func viewWillSetupKeyboardView() {
        setupKeyboardView { (controller: KeyboardInputViewController) in
            guard let keyboardController = controller as? KeyboardViewController,
                  let handler = keyboardController.actionHandler
            else {
                return AnyView(EmptyView())
            }

            return AnyView(keyboardController.createQwertyKeyboardView(
                controller: controller,
                composingManager: handler.composingManager,
            ))
        }
    }

    /// 建立 QWERTY 鍵盤視圖
    private func createQwertyKeyboardView(
        controller: KeyboardInputViewController,
        composingManager: ComposingManager,
    ) -> some View {
        let layoutService = CustomLayoutService()
        let layout = layoutService.keyboardLayout(for: controller.state.keyboardContext)

        return TaigiKeyboardView(
            services: controller.services,
            layout: layout,
            emojiKeyboardView: { [unowned self] in
                emojiService.getEmojiKeyboardView()
            },
            calloutStyle: createCalloutStyle(),
            autocompleteContext: controller.state.autocompleteContext,
            keyboardContext: controller.state.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { [unowned controller] suggestion in
                controller.services.actionHandler.handle(suggestion)
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

    /// FIXME: Workaround layer 1/3 for KeyboardKit 10 auto-capitalization override.
    /// Skips super's setKeyboardCase(preferredKeyboardCase) when auto-cap is off.
    override func textDidChangeAsync(_ textInput: UITextInput?) {
        let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

        let caseDesc = String(describing: state.keyboardContext.keyboardCase)
        logger.debug("[CASE][textDidChangeAsync] isAutoCap=\(isAutoCap) keyboardCase=\(caseDesc)")

        // DEBUG: NextWord trace - textDidChangeAsync state
        if let handler = actionHandler {
            let isNextWord = handler.isShowingNextWord
            let isComp = handler.composingManager.isComposing
            let raw = handler.composingManager.rawInput
            logger.debug("[NEXTWORD][textDidChangeAsync] isShowingNextWord=\(isNextWord) isComposing=\(isComp) rawInput='\(raw)'")
        }

        if isAutoCap {
            // 自動大寫開啟：使用 KeyboardKit 預設行為
            super.textDidChangeAsync(textInput)
        } else {
            // 自動大寫關閉：只執行 autocomplete，不調整 keyboardCase
            performAutocomplete()
        }
    }

    // MARK: - KeyboardCase Tracking

    /// FIXME: Workaround layer 2/3 — debug tracking for KeyboardKit case changes.
    /// Remove when auto-capitalization workaround is no longer needed.
    override func setKeyboardCase(_ case: Keyboard.KeyboardCase) {
        let before = state.keyboardContext.keyboardCase
        let beforeDesc = String(describing: before)
        let newDesc = String(describing: `case`)
        logger.debug("[CASE][setKeyboardCase] before=\(beforeDesc) new=\(newDesc)")
        super.setKeyboardCase(`case`)
        let afterDesc = String(describing: state.keyboardContext.keyboardCase)
        logger.debug("[CASE][setKeyboardCase] after=\(afterDesc)")
    }

    // MARK: - KeyboardCase Protection

    /// FIXME: Workaround layer 3/3 — Combine-based guard against KeyboardKit 10
    /// internally setting keyboardCase = preferredKeyboardCase via a code path that
    /// bypasses our setKeyboardCase/tryChangeKeyboardCase overrides.
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
                guard let self else { return }
                if newType == .alphabetic {
                    justSwitchedToAlphabetic = true
                    logger.debug("[CASE][PROTECT] keyboardType → alphabetic, flag set")
                }
            }
            .store(in: &cancellables)

        // 監聽 keyboardCase 變化，檢查是否需要阻止
        state.keyboardContext.$keyboardCase
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newCase in
                guard let self else { return }
                let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

                logger.debug("[CASE][PROTECT] newCase=\(String(describing: newCase)) expected=\(String(describing: expectedKeyboardCase)) isAutoCap=\(isAutoCap) justSwitched=\(justSwitchedToAlphabetic)")

                // 當自動大寫關閉且剛切換到字母鍵盤時，阻止非預期的 uppercased 變化
                if !isAutoCap,
                   justSwitchedToAlphabetic,
                   newCase == .uppercased,
                   expectedKeyboardCase != .uppercased,
                   expectedKeyboardCase != .capsLocked
                {
                    logger.debug("[CASE][PROTECT] ⚠️ BLOCKING uppercased, restoring to \(String(describing: expectedKeyboardCase))")
                    // 使用異步恢復，確保在 KeyboardKit 內部處理完成後執行
                    let targetCase = expectedKeyboardCase
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        logger.debug("[CASE][PROTECT] async restoring to \(String(describing: targetCase))")
                        state.keyboardContext.keyboardCase = targetCase
                    }
                } else {
                    // 更新預期值（合法的變化）
                    expectedKeyboardCase = newCase
                }

                // 清除標志（無論是否阻止，都清除）
                justSwitchedToAlphabetic = false
            }
            .store(in: &cancellables)
    }

    // MARK: - Settings Observer

    /// 設定監聯器（監聯主 App 的設定變更）
    private func setupSettingsObserver() {
        logger.debug("[AUTOCAP][SETTINGS] setupSettingsObserver registered")

        settingsObserver = NotificationCenter.default.addObserver(
            forName: UserDefaults.didChangeNotification,
            object: SharedSettings.sharedUserDefaults,
            queue: .main,
        ) { [weak self] _ in
            self?.logger.debug("[AUTOCAP][SETTINGS] UserDefaults.didChangeNotification received")
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
