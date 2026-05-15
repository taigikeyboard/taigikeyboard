// 中文: 鍵盤擴充的主 ViewController。
// 中文: 主檔負責 KeyboardKit 生命週期 + 輸入欄位偵測 + KeyboardKit 10 auto-cap workaround。
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

    /// FIXME: Workaround for KeyboardKit 10 auto-capitalization override.
    /// These two properties are part of a 2-layer workaround:
    /// - Layer 1: textDidChangeAsync override (this file) — skips super when auto-cap off
    /// - Layer 2: setupKeyboardCaseProtection (this file) — Combine guard for internal path
    /// Also: tryChangeKeyboardCase override (ActionHandler.swift) — blocks non-shift case changes
    /// Remove when KeyboardKit provides a proper API to disable auto-capitalization.
    private var expectedKeyboardCase: Keyboard.KeyboardCase = .lowercased
    private var justSwitchedToAlphabetic = false

    /// Previous values for change detection in syncSettings()
    var lastInputMode: InputMode?
    var lastKeyboardLayoutType: KeyboardLayoutType?

    /// Identity of the most recent `UITextInput` seen by `textWillChange`.
    /// Pointer-equality detects field switches without touching the iOS 26
    /// SDK's broken `documentIdentifier` UUID bridge.
    // 中文: 用 UITextInput 物件的 ObjectIdentifier 來偵測輸入欄位切換,
    // 中文: 避開 iOS 26 SDK 損壞的 documentIdentifier UUID bridge。
    private var lastTextInputID: ObjectIdentifier?

    /// Bug 3 (v3.5.8 Phase 9) — iOS-only continuous mid-commit tail-leak
    /// workaround. A continuous mid-commit commits the chosen segment via
    /// `insertText` and re-marks the pending raw tail via `setMarkedText`
    /// in one synchronous turn; the host then confirms that marked tail
    /// into literal document text during its textWillChange→textDidChange
    /// settle (deterministic — `continuous-input-ranking.md` §10.6 iOS
    /// divergence). `armed…` holds the re-marked tail from the moment it
    /// is set inside a self-driven mid-commit; `leaked…` is the frozen
    /// confirmed-leaked string awaiting deletion on the next preedit clear
    /// / re-mark. `armedTailPresentBeforeHostSettle` is the pre-settle
    /// baseline captured at `textWillChange`: the leak is real only when
    /// the tail becomes a document suffix that was NOT already one before
    /// the host's settle — this rejects the false positive where a
    /// committed segment (e.g. a roman-output form) coincidentally ends
    /// with the pending raw tail. Android is structurally immune (atomic
    /// `commitText` + host-tracked composing region) — see §10.6.
    // 中文: Bug 3 iOS-only — 連續 mid-commit 重設的 marked tail 被 host 在
    // 中文: textDidChange 前確認成字面文字;用 textWillChange 前的後綴基準
    // 中文: 排除「commit 字串本身剛好以 tail 結尾」的假陽性,再補刪掉。
    private var armedContinuousMidCommitTail: String = ""
    private var leakedContinuousMidCommitTail: String = ""
    private var armedTailPresentBeforeHostSettle = false

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

        setupServices()

        // Observe settings changes (live sync from main app)
        setupSettingsObserver()

        // Guard keyboardCase against KeyboardKit 10 internal path overriding state
        setupKeyboardCaseProtection()
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
            calloutStyle: createCalloutStyle(),
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
        detectContinuousMidCommitTailLeak()
        actionHandler?.nextWordController.resetAndClearUI()
    }

    /// Bug 3: snapshot, at the pre-settle `textWillChange` boundary,
    /// whether the armed tail is ALREADY a document suffix. Runs before
    /// the field-switch guards (the mid-commit's own `textWillChange`
    /// early-returns on same-field identity) so the baseline is always
    /// captured. A natural-callback read — not a perturbing mid-dispatch
    /// probe.
    // 中文: 在 host settle 前(textWillChange)記錄 armed tail 是否已是後綴,作為基準。
    private func captureMidCommitTailPreConfirmBaseline() {
        guard !armedContinuousMidCommitTail.isEmpty, leakedContinuousMidCommitTail.isEmpty else { return }
        let context = textDocumentProxy.documentContextBeforeInput ?? ""
        armedTailPresentBeforeHostSettle = context.hasSuffix(armedContinuousMidCommitTail)
    }

    /// Bug 3: detect the host confirming a continuous mid-commit's
    /// re-marked tail into literal document text. `textDidChange` is the
    /// proven finalize boundary (real-device trace). The leak is real only
    /// when the tail is the live document suffix AND was NOT a suffix at
    /// the pre-settle baseline — i.e. it was *newly appended* by the host
    /// confirming the marked region, not a commit string that already
    /// ended with the same characters (Codex post-impl must-fix #1). A
    /// host that keeps the region alive never triggers the compensation
    /// delete (the marked region is still there for `setMarkedText("")` to
    /// clear normally). Single-shot: the leak materializes within the one
    /// text-change transaction the mid-commit triggers, so the arm is
    /// consumed at the next `textDidChange` regardless of outcome.
    // 中文: 在 textDidChange(已證實的 finalize 邊界)偵測 — tail 必須是
    // 中文: 「新被附加」的後綴(基準時不是後綴、settle 後才是)才算 leak。
    private func detectContinuousMidCommitTailLeak() {
        guard !armedContinuousMidCommitTail.isEmpty,
              leakedContinuousMidCommitTail.isEmpty,
              actionHandler?.composingManager.selfCommitInProgress == false,
              actionHandler?.composingManager.isComposing == true
        else { return }
        let tail = armedContinuousMidCommitTail
        let wasPresentBeforeSettle = armedTailPresentBeforeHostSettle
        armedContinuousMidCommitTail = ""
        armedTailPresentBeforeHostSettle = false
        guard !wasPresentBeforeSettle else { return }
        if let context = textDocumentProxy.documentContextBeforeInput, context.hasSuffix(tail) {
            leakedContinuousMidCommitTail = tail
        }
    }

    /// Bug 3: a confirmed-leaked tail is now literal document text (the
    /// marked region is gone, so `setMarkedText("")` is a no-op). Delete
    /// the leaked characters before the next preedit clear / re-mark so
    /// the following commit replaces them. Grapheme-count `deleteBackward`
    /// is correct for NFC tone-marked tails. Called from `setMarkedText` /
    /// `clearMarkedText` (`KeyboardViewController+TextInput.swift`).
    ///
    /// Cursor-side re-validation (Codex PR #282 r3249647910, P1): the leak
    /// state only resets on a real field switch, so the caret may have
    /// moved within the same field since the leak was recorded. Clear the
    /// state first, then delete ONLY when the leaked tail is still the
    /// immediate document suffix — otherwise abandon (leaving the stray
    /// tail is far better than deleting unrelated text at the moved caret).
    // 中文: 補償 — 先清狀態,只有當 leaked tail 仍是游標前文件後綴才刪;
    // 中文: 游標已移走(同欄位不觸發 reset)就放棄,絕不刪到無關文字。
    func compensateLeakedContinuousMidCommitTail() {
        let leaked = leakedContinuousMidCommitTail
        guard !leaked.isEmpty else { return }
        leakedContinuousMidCommitTail = ""
        armedContinuousMidCommitTail = ""
        armedTailPresentBeforeHostSettle = false
        guard textDocumentProxy.documentContextBeforeInput?.hasSuffix(leaked) == true else { return }
        for _ in 0 ..< leaked.count {
            textDocumentProxy.deleteBackward()
        }
    }

    /// Bug 3: arm the re-marked tail. Only called when `setMarkedText` runs
    /// inside a self-driven continuous mid-commit (`selfCommitInProgress`).
    /// A frozen leak is never overwritten — compensation clears it first.
    // 中文: 武裝 — 只在自我送出的連續 mid-commit 內呼叫,凍結中的 leak 不被覆寫。
    func armContinuousMidCommitTail(_ tail: String) {
        guard leakedContinuousMidCommitTail.isEmpty else { return }
        armedContinuousMidCommitTail = tail
        // Fresh per mid-commit: textWillChange sets the real baseline; if
        // it never fires, a false baseline lets detect treat a newly
        // appended suffix as the leak (the proven host behaviour).
        armedTailPresentBeforeHostSettle = false
    }

    /// Bug 3: drop all workaround state without deleting — used on a real
    /// input-field switch. The leak (if any) lives in the field we are
    /// leaving; deleting backward in the new field would corrupt it.
    // 中文: 真正切換欄位時只丟棄狀態、不刪字(leak 在舊欄位,刪新欄位會壞)。
    private func resetContinuousMidCommitTailState() {
        armedContinuousMidCommitTail = ""
        leakedContinuousMidCommitTail = ""
        armedTailPresentBeforeHostSettle = false
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
        // Bug 3: capture the pre-settle suffix baseline before the
        // field-switch guards (the mid-commit's textWillChange same-field
        // early-returns below, so this must run first).
        captureMidCommitTailPreConfirmBaseline()
        guard let manager = actionHandler?.composingManager else { return }
        if manager.selfCommitInProgress { return }
        let id = textInput.map { ObjectIdentifier($0 as AnyObject) }
        if lastTextInputID == id { return }
        lastTextInputID = id
        // Real input-field switch — the Bug 3 tail-leak workaround is
        // scoped to one composing session; drop it (without deleting).
        resetContinuousMidCommitTailState()
        manager.bumpGeneration()
        // Mirror the composing-slice IME-session bump for the NextWord
        // engine handle so cross-field state (lastSelectedWord / is_showing /
        // current_generation) drops on real input-context changes — Codex
        // post-impl P2-2. Android does the equivalent in
        // `NextWordHandler.resetContext()` invoked from `onStartInputView`.
        actionHandler?.nextWordController.bumpEnvelopeGeneration()
    }

    /// FIXME: Workaround layer 1/2 for KeyboardKit 10 auto-capitalization override.
    /// Skips super's setKeyboardCase(preferredKeyboardCase) when auto-cap is off.
    override func textDidChangeAsync(_ textInput: UITextInput?) {
        let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled
        logger.debug("[CASE][textDidChangeAsync] isAutoCap=\(isAutoCap) keyboardCase=\(String(describing: state.keyboardContext.keyboardCase))")

        if isAutoCap {
            super.textDidChangeAsync(textInput)
        } else {
            performAutocomplete()
        }
    }

    // MARK: - KeyboardCase Protection

    /// FIXME: Workaround layer 2/2 — Combine-based guard against KeyboardKit 10
    /// internally setting keyboardCase = preferredKeyboardCase via a code path that
    /// bypasses our setKeyboardCase/tryChangeKeyboardCase overrides.
    ///
    /// KeyboardKit 10 sets keyboardCase = preferredKeyboardCase via an internal path
    /// when keyboardType switches to alphabetic, bypassing our tryChangeKeyboardCase
    /// and setKeyboardCase overrides. This guard observes keyboardCase changes and
    /// restores the expected state when auto-capitalization is off.
    // 中文: KeyboardKit 10 auto-cap workaround 第二層 — 用 Combine 觀察 keyboardCase 變動,
    // 中文: 在 auto-cap 關閉時把被內部路徑改掉的大寫狀態還原。
    private func setupKeyboardCaseProtection() {
        // Initialize expected value
        expectedKeyboardCase = state.keyboardContext.keyboardCase

        // Observe keyboardType changes and set the flag
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

        // Observe keyboardCase changes and block unexpected mutations
        state.keyboardContext.$keyboardCase
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] newCase in
                guard let self else { return }
                let isAutoCap = state.keyboardContext.settings.isAutocapitalizationEnabled

                logger.debug("[CASE][PROTECT] newCase=\(String(describing: newCase)) expected=\(String(describing: expectedKeyboardCase)) isAutoCap=\(isAutoCap) justSwitched=\(justSwitchedToAlphabetic)")

                // Block unexpected uppercased when auto-cap is off and just switched to alphabetic
                if !isAutoCap,
                   justSwitchedToAlphabetic,
                   newCase == .uppercased,
                   expectedKeyboardCase != .uppercased,
                   expectedKeyboardCase != .capsLocked
                {
                    logger.debug("[CASE][PROTECT] ⚠️ BLOCKING uppercased, restoring to \(String(describing: expectedKeyboardCase))")
                    // Restore asynchronously to ensure KeyboardKit internal processing completes first
                    let targetCase = expectedKeyboardCase
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        logger.debug("[CASE][PROTECT] async restoring to \(String(describing: targetCase))")
                        state.keyboardContext.keyboardCase = targetCase
                    }
                } else {
                    // Update expected value (legitimate change)
                    expectedKeyboardCase = newCase
                }

                // Clear the flag regardless of whether we blocked
                justSwitchedToAlphabetic = false
            }
            .store(in: &cancellables)
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
