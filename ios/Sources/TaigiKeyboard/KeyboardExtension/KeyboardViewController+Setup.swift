// 中文: 鍵盤擴充設定 / 服務初始化擴充。
// 中文: 安裝 lexicon engine、串接 ActionHandler / AutocompleteService、處理設定變動同步。

import KeyboardKit
import SwiftUI
import UIKit

// MARK: - Setup & Configuration

private let setupLogger = DebugLogger(category: "KeyboardViewController+Setup")

extension KeyboardViewController {
    // 中文: 主初始化序 — 必須在第一次存取 KeyboardSettings 前呼叫。
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
    // 中文: 在 extension 啟動時安裝 Rust lexicon engine,bundle 資源整個生命週期都是唯讀,所以只裝一次。
    // 中文: 失敗就記 log,首次查詢時引擎自然回 [] 走 graceful 降級。
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
        // v3.5.8 Phase 6 — syllables.fst powers continuous-input candidate
        // fetch. Missing → empty path → engine skips inventory load →
        // FetchAtPos returns empty candidates (graceful degrade). Logged
        // explicitly so dogfood notices a missing pbxproj file ref early.
        let syllablesURL = bundle.url(forResource: "syllables", withExtension: "fst")
        if syllablesURL == nil {
            setupLogger.warning(
                "[LEXICON] syllables.fst not found in bundle; "
                    + "continuous-input candidate fetch will return empty",
            )
        }
        let stamp = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(UInt32.init) ?? 1
        if let stats = RustEngineBridge.lexiconInstall(
            triePath: fstURL.path,
            dictionaryBinPath: dictBinURL.path,
            associationBinPath: assocBinURL.path,
            dictionaryVersion: stamp,
            syllableInventoryPath: syllablesURL?.path ?? "",
        ) {
            setupLogger.info(
                "[LEXICON] installed: dict=\(stats.dictionaryRecordCount) "
                    + "fst_entries=\(stats.prefixIndexEntryCount) version=\(stamp) "
                    + "syllables=\(syllablesURL == nil ? "absent" : "present")",
            )
        } else {
            setupLogger.warning("[LEXICON] install returned nil; engine not installed")
        }
    }

    // 中文: 在支援 Liquid Glass 的裝置上開啟對應視覺效果。
    func setupLiquidGlass() {
        let context = state.keyboardContext
        if context.isLiquidGlassAvailable {
            context.isLiquidGlassEnabled = true
        }
    }

    /// Order matters: AutocompleteService → ActionHandler → link them together.
    // 中文: 順序敏感的服務組裝 — 先建 AutocompleteService,再建 ActionHandler,最後串連兩邊。
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
            keyboardAppContext: state.keyboardAppContext,
            spacebarDragGestureHandler: services.spacebarDragGestureHandler,
        )

        services.actionHandler = handler
        actionHandler = handler

        handler.composingManager.setContextSink(state.keyboardContext)
        handler.composingManager.delegate = self
        handler.nextWordController.contextUpdater = handler

        // 4. Connect TaigiAutocompleteService with handler (requires handler already created)
        wireTaigiAutocompleteProviders(from: services.autocompleteService, to: handler)

        // 5. Best-effort warmup of the two SYNCHRONOUS eager-empty hot-path
        //    user-data DBs that `fetchContinuousCandidates` reads:
        //    `user_frequency.db` (boost — `UserFrequencyService.frequencyDataBatch`)
        //    and `custom_dictionary.db` (custom candidates —
        //    `CustomDictionaryRepository.searchSync`). Both readers return `[]`
        //    until their connection is open and NEVER lazy-open (the Continuous
        //    fetch is synchronous and must not block on an async DB open), so
        //    without an eager warmup here a fresh session would ignore them
        //    indefinitely until the user committed something. NextWord's
        //    `user_association.db` is intentionally NOT warmed here — its reads
        //    are async and lazy-init per query (`ensureUserTablesCreated`).
        //    A future third synchronous eager-empty reader MUST be warmed here.
        //
        //    Fire-and-forget — `fetchContinuousCandidates` keeps its
        //    `isConnected()` cold-start guard for the race window before these
        //    Tasks land. Warning log on failure is intentional for
        //    observability (Codex PR #265 r3216760651 post-impl R5).
        //
        //    The custom_dictionary.db warmup is UNGATED (not behind
        //    `isCustomDictEnabled`): the lookup is already gated in
        //    `ComposingManager.buildCustomEntries`, and an ungated warmup keeps
        //    the connection ready for a live settings toggle (enable in the
        //    host app → works in an already-running extension, no relaunch).
        //    Regression guard: PR #279 (Item 13) deleted the old
        //    `LexiconService` fallback that lazy-opened this DB but only kept
        //    the user-freq warmup, so custom-dict candidates silently vanished
        //    from the keyboard (behavioral-invariants.md §26).
        // 中文: 連續輸入同步讀取的兩個 eager-empty user-data DB 都在此提前打開 +
        // 中文:   建 schema:user_frequency.db(boost)+ custom_dictionary.db(自訂詞候選)。
        // 中文:   兩者 searchSync / frequencyDataBatch 在連線開啟前回 [] 且不 lazy-open
        // 中文:   (連續 fetch 同步,不可 block async open),故須在此 warmup。
        // 中文:   NextWord user_association.db 不在此 — 走 async lazy-init,不需要。
        // 中文:   custom dict warmup ungated(查詢已在 buildCustomEntries gate),連線
        // 中文:   常駐讓設定即時開關免重啟生效;#279 刪舊 LexiconService lazy-open 卻只留
        // 中文:   user-freq warmup → 自訂詞候選消失(§26 regression guard)。
        let userFrequencyService = CompositionRoot.userFrequencyService
        Task {
            do {
                try await userFrequencyService.ensureInitialized()
                setupLogger.info("[INIT] User frequency DB warmed")
            } catch {
                setupLogger.warning(
                    "[INIT] User frequency DB warmup failed: \(error.localizedDescription)",
                )
            }
        }
        let customDictionaryRepository = CompositionRoot.customDictionaryRepository
        Task {
            do {
                try await customDictionaryRepository.ensureInitialized()
                setupLogger.info("[INIT] Custom dictionary DB warmed")
            } catch {
                setupLogger.warning(
                    "[INIT] Custom dictionary DB warmup failed: \(error.localizedDescription)",
                )
            }
        }

        // 6. Initialize tracking vars so syncSettings() doesn't false-trigger on first call
        lastInputMode = keyboardSettings.inputMode
        lastKeyboardLayoutType = keyboardSettings.keyboardLayoutType
    }

    /// Called at initial setup and from syncSettings() when input mode changes.
    // 中文: 依當前 inputMode 安裝對應的 AutocompleteService。English 模式用 EnglishAutocompleteService,
    // 中文: 其它模式用 Taigi 自家的 TaigiAutocompleteService。在初始 setup 與 settings 變動時都會呼叫。
    func setupAutocompleteServiceForCurrentMode() {
        if keyboardSettings.inputMode == .english {
            services.autocompleteService = EnglishAutocompleteService()
            setupLogger.debug("[AUTOCOMPLETE] Using EnglishAutocompleteService")
        } else {
            let autocompleteService = TaigiAutocompleteService()
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

    /// 將 Taigi 專用的 composing provider 接上 TaigiAutocompleteService。
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
        guard let taigiService = service as? TaigiAutocompleteService else { return }
        taigiService.setComposingManager(handler.composingManager)
    }

    /// Re-read settings from App Group UserDefaults.
    /// @AppStorage didSet doesn't fire for changes from an external process,
    /// so this is triggered via UserDefaults.didChangeNotification.
    // 中文: 從 App Group UserDefaults 重新讀取設定,@AppStorage 對跨 process 變動不會觸發 didSet,
    // 中文: 所以由外部 didChangeNotification 主動驅動。
    func syncSettings() {
        var needsAutocompleteReset = false

        // Check if input mode changed; if so, recreate AutocompleteService
        let currentInputMode = keyboardSettings.inputMode
        if lastInputMode != currentInputMode {
            let previousMode = lastInputMode?.rawValue ?? "nil"
            setupLogger.debug("[SETTINGS] InputMode changed: \(previousMode) -> \(currentInputMode.rawValue)")
            // v3.5.8 Phase 7B (Codex Fork F) — clear stale Continuous-input
            // state before swapping the AutocompleteService. POJ↔TL↔TPS uses
            // distinct Phase::Continuous { raw } byte conventions, so leftover
            // pending bytes would mis-align consumed-span offsets returned by
            // the new mode's FetchAtPos calls. `bumpGeneration` then causes
            // any in-flight engine call to be silently dropped at the FFI
            // boundary (`engine/composing/src/handle.rs:61-65`).
            // 中文: 切換輸入模式前先清連續輸入狀態 + bump generation,避免跨模式 byte
            // 中文: 偏移污染與 in-flight 請求滲入新模式。
            actionHandler?.composingManager.resetContinuous()
            actionHandler?.composingManager.bumpGeneration()
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

    // 中文: 依使用者選擇的字型生成長按 callout 視覺樣式。
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
