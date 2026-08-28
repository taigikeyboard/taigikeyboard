// 中文: 鍵盤擴充的主 ViewController。
// 中文: 主檔負責 KeyboardKit 生命週期 + 輸入欄位偵測。
// 中文: 其他細節在 KeyboardViewController+*.swift 各擴充。

import Combine
import KeyboardKit
import SwiftUI

// 中文: 鍵盤擴充進入點 — 同時擔任 ComposingDelegate,把 effect 派送到 textDocumentProxy。
class KeyboardViewController: KeyboardInputViewController, ComposingDelegate {
    // MARK: - Properties

    let logger = DebugLogger(category: "KeyboardViewController")

    /// DI seam. Default retains existing process-wide singleton behavior;
    /// tests (and future composition roots) can substitute a stub.
    // 中文: DI 縫隙 — 預設用 SharedSettings.shared,測試或新組裝點可注入 stub。
    let keyboardSettings: any KeyboardEnvironment = SharedSettings.shared

    var emojiServiceStorage: EmojiService?
    weak var actionHandler: ActionHandler?
    var isCleanedUp = false

    private var cancellables = Set<AnyCancellable>()

    /// Guards the successful `setupKeyboardKit(for:)` completion body:
    /// KeyboardKit may invoke the setup hook again in the controller's
    /// lifetime, and `setupServices()` / `setupSettingsObserver()` are not
    /// idempotent (service rebuild + duplicate Combine subscriptions).
    /// A failed completion leaves the guard unset so a retry can complete.
    // 中文: 成功的 setup completion 只跑一次 — setupServices/observer 不可重入;
    // 中文: 失敗不消耗 guard,保留重試機會。
    private var hasCompletedKeyboardKitSetup = false

    /// Previous values for change detection in syncSettings()
    var lastInputMode: InputMode?
    var lastKeyboardLayoutType: KeyboardLayoutType?

    /// Previous resolved key-height scale. Row height lives in the layout, which
    /// is built once in `createKeyboardView`; when the active/edited theme changes
    /// it, `syncSettings()` rebuilds the keyboard view so the layout recomputes.
    /// Colors / font / corner already update in place via `TaigiKeyboardView`.
    // 中文: 上次解析的鍵高係數。row height 由 layout 一次建好,主題改變其值時 syncSettings 重建鍵盤 view。
    var lastResolvedKeyHeightScale: Double?

    /// Identity of the most recent `UITextInput` seen by `textWillChange`.
    /// Pointer-equality detects field switches without touching the iOS 26
    /// SDK's broken `documentIdentifier` UUID bridge.
    // 中文: 用 UITextInput 物件的 ObjectIdentifier 來偵測輸入欄位切換,
    // 中文: 避開 iOS 26 SDK 損壞的 documentIdentifier UUID bridge。
    private var lastTextInputID: ObjectIdentifier?

    // 中文: emoji 服務的 lazy 取出口 — 第一次存取時建立並把自己設為 delegate。
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

        // Install the shared-core logging backend so engine-layer code
        // routes logs through DebugLogger.
        LoggerFactory.install { DebugLogger(category: $0) }

        // Wire the Rust engine logger sink so Rust `log::warn!` lines reach
        // DebugLogger. Idempotent — main app also calls this in `init`.
        RustEngineBridge.install()

        // Install the lexicon engine state (fst + dictionary.bin +
        // association.bin) once at extension launch. Idempotent — calling
        // again with the same paths is a no-op observation-wise. Bundle
        // assets are read-only and stable across the keyboard extension's
        // lifetime, so no reinstall is needed within a session.
        installLexiconEngine()

        // Register custom fonts from containing app bundle (extension only)
        FontRegistration.registerFontsIfNeeded()

        // Everything that touches KeyboardKit settings or services is deferred
        // to the setupKeyboardKit(for:) completion — see viewWillSetupKeyboardKit.
    }

    /// Standard KeyboardKit setup (KK ≥ 10.8.1 / upstream #967): the SDK
    /// wires App Group settings syncing before its keyboard-case logic reads
    /// any setting, so `isAutocapitalizationEnabled = false` works with no
    /// case workarounds. Settings and service writes happen in the completion
    /// — writes made before setup completes can be overwritten by it.
    // 中文: KeyboardKit 標準初始化 — 設定與服務組裝一律在 completion 內做,
    // 中文: setup 前寫入可能被覆寫(官方契約)。
    override func viewWillSetupKeyboardKit() {
        setupKeyboardKit(for: .taigiKeyboard) { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let error):
                // Do not consume the once-guard: a later successful hook
                // invocation can still complete setup.
                logger.warning("[SETUP] setupKeyboardKit failed: \(error.localizedDescription)")
            case .success:
                guard !hasCompletedKeyboardKitSetup else { return }
                hasCompletedKeyboardKitSetup = true
                setupServices()
                setupSettingsObserver()
                // viewDidAppear may have fired before this completion; its
                // syncSettings() call is guarded on actionHandler, so run the
                // initial sync here.
                syncSettings()
            }
        }
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)

        // Update full access status (read by main app's SetupGuide).
        // Guard to avoid unnecessary UserDefaults write → didChangeNotification → double syncSettings().
        let currentFullAccess = hasFullAccess
        if keyboardSettings.isFullAccessEnabled != currentFullAccess {
            keyboardSettings.isFullAccessEnabled = currentFullAccess
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
            settings: keyboardSettings,
            services: services,
            layout: layout,
            emojiKeyboardView: { [unowned self] in
                emojiService.emojiKeyboardView
            },
            calloutStyle: .taigi(for: keyboardSettings.fontType),
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
    // 中文: autocomplete 用 rawInput 而非 composingText — rawInput 保留調符數字以利 Trie lookup。
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

    /// v3.5.4 lifecycle (plan §4.2): bump the composing engine's
    /// input-context generation on a real field change.
    ///
    /// `selfCommitInProgress` skips the IME's own commits (candidate tap,
    /// self-driven write re-entry). The `ObjectIdentifier` compare detects
    /// genuine field switches via UITextInput class-instance identity —
    /// distinct fields are distinct instances. Same-field textWillChange
    /// fires (typing, selection change) keep the same identity and skip
    /// the bump, preserving an active composing buffer.
    // 中文: 真正切換輸入欄位時 +1 generation,觸發引擎丟棄 stale 狀態。
    // 中文: 自我 commit / 同欄位 textWillChange 不算切換,維持組字 buffer 不被誤清。
    override func textWillChange(_ textInput: UITextInput?) {
        super.textWillChange(textInput)
        guard let manager = actionHandler?.composingManager else { return }
        if manager.selfCommitInProgress { return }
        let id = textInput.map { ObjectIdentifier($0 as AnyObject) }
        if lastTextInputID == id { return }
        lastTextInputID = id
        // Real input-field switch — hard-abort the continuous composition
        // (Model B: nailed segments were never in the document, so the
        // generation bump cleanly discards them; `continuous-input-
        // ranking.md` §10.6 external-region-clear = hard abort).
        manager.bumpGeneration()
        // Mirror the composing-slice IME-session bump for the NextWord
        // engine handle so cross-field state (lastSelectedWord / is_showing /
        // current_generation) drops on real input-context changes — Codex
        // post-impl P2-2. Android does the equivalent in
        // `NextWordHandler.resetContext()` invoked from `onStartInputView`.
        actionHandler?.nextWordController.bumpEnvelopeGeneration()
    }

    // MARK: - UIResponder Text Input Overrides

    // Route UIResponder text-input calls (e.g. external/hardware keyboard, system
    // voice input) through the text document proxy so they land in the host doc.
    // ComposingDelegate.execute(_:) operates on the proxy independently — these
    // overrides are for UIResponder-chain callers, not for the composing engine.

    override func insertText(_ text: String) {
        textDocumentProxy.insertText(text)
    }

    override func deleteBackward() {
        textDocumentProxy.deleteBackward()
    }

    // MARK: - Settings Observer

    /// Observe main app settings via App Group UserDefaults → syncSettings()
    // 中文: 觀察主 App 透過 App Group UserDefaults 改設定,觸發 syncSettings 即時同步。
    private func setupSettingsObserver() {
        NotificationCenter.default.publisher(
            for: UserDefaults.didChangeNotification,
            object: keyboardSettings.settingsUserDefaults,
        )
        .receive(on: DispatchQueue.main)
        .sink { [weak self] _ in
            self?.syncSettings()
        }
        .store(in: &cancellables)
    }
}
