// 中文: Taigi Keyboard 主 App 進入點,負責 logger / Rust engine / KeyboardKit 設定 / 導覽列外觀。

import KeyboardKit
import SwiftUI

/// Taigi Keyboard app entry point.
// 中文: 主 App entry point。init 內依序安裝:
// 中文: 1) LoggerFactory(共用 core 引擎走 DebugLogger)
// 中文: 2) RustEngineBridge(Rust log sink,冪等)
// 中文: 3) Lexicon engine(主 App 程序的 fst + 詞庫 binary)
// 中文: 4) KeyboardKit App Group store(@AppStorage 前置條件)
// 中文: 5) UINavigationBar 字型外觀(SwiftUI .environment 不影響此處)
// 中文: 6) 自訂詞庫首次安裝 seed
@main
struct TaigiKeyboardApp: App {
    @StateObject private var keyboardStatus = KeyboardStatusContext(
        bundleId: (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension",
    )

    init() {
        // Install the shared-core logging backend so engine-layer code
        // routes logs through DebugLogger.
        LoggerFactory.install { DebugLogger(category: $0) }

        // Wire the Rust engine logger sink so Rust `log::warn!` lines reach
        // DebugLogger. Idempotent. Mirrors Android `Application.onCreate`.
        RustEngineBridge.install()

        // Install the Rust shared-core lexicon engine state for the main
        // app (Dictionary tab uses bundled fst + dict.bin reads). Idempotent
        // — extension calls the same install separately at viewDidLoad.
        // Bundle paths resolve through `ResourceBundleResolver`.
        installLexiconEngineForMainApp()

        // Configure KeyboardKit to persist settings via App Group.
        // Must be called before any @AppStorage access. Same KeyboardApp value
        // as the extension's setupKeyboardKit(for:) — identical store + prefix.
        KeyboardSettings.setupStore(for: .taigiKeyboard)

        // Navigation bar title font (UIKit appearance, not affected by SwiftUI .environment)
        let navAppearance = UINavigationBarAppearance()
        navAppearance.configureWithDefaultBackground()
        let largeFont = UIFont(name: KeyboardFonts.openHuninnFontName, size: AppStyle.navBarLargeTitleSize)
            ?? .systemFont(ofSize: AppStyle.navBarLargeTitleSize)
        let inlineFont = UIFont(name: KeyboardFonts.openHuninnFontName, size: AppStyle.navBarInlineTitleSize)
            ?? .systemFont(ofSize: AppStyle.navBarInlineTitleSize)
        navAppearance.largeTitleTextAttributes = [.font: largeFont]
        navAppearance.titleTextAttributes = [.font: inlineFont]
        UINavigationBar.appearance().standardAppearance = navAppearance
        UINavigationBar.appearance().scrollEdgeAppearance = navAppearance

        // Seed custom dictionary default entries on first install only
        Task {
            try? await CompositionRoot.customDictionaryService.seedDefaultEntryIfEmpty()
        }
    }

    /// Install the Rust shared-core lexicon engine for the main app
    /// process (Dictionary tab uses the same fst + bundled binaries).
    /// Idempotent; the keyboard extension does its own install in
    /// `KeyboardViewController.viewDidLoad`.
    // 中文: 為主 App 程序安裝 Rust shared-core lexicon 引擎(Dictionary tab 用)。
    // 中文: 冪等;鍵盤擴充走自己 viewDidLoad 的安裝路徑。
    private func installLexiconEngineForMainApp() {
        let bundle = ResourceBundleResolver.dictionaryBundle
        guard
            let fstURL = bundle.url(forResource: "dictionary", withExtension: "fst"),
            let dictBinURL = bundle.url(forResource: "dictionary", withExtension: "bin"),
            let assocBinURL = bundle.url(forResource: "association", withExtension: "bin")
        else {
            return
        }
        // syllables.fst is a v3.5.8 Phase 6 addition; absence is a graceful
        // skip (FetchAtPos returns empty candidates). Main app process does
        // not run continuous-input today, so empty path is acceptable here.
        let syllablesPath = bundle.url(forResource: "syllables", withExtension: "fst")?.path ?? ""
        let stamp = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String).flatMap(UInt32.init) ?? 1
        _ = RustEngineBridge.lexiconInstall(
            triePath: fstURL.path,
            dictionaryBinPath: dictBinURL.path,
            associationBinPath: assocBinURL.path,
            dictionaryVersion: stamp,
            syllableInventoryPath: syllablesPath,
        )
    }

    var body: some Scene {
        WindowGroup {
            AppRootView(keyboardStatus: keyboardStatus)
        }
    }
}

/// App root view.
///
/// Manages deep links and setup guide flow.
// 中文: App 根 View。處理 taigikeyboard:// deep link 與 setup guide 全螢幕流程。
struct AppRootView: View {
    @ObservedObject var keyboardStatus: KeyboardStatusContext
    @StateObject private var viewModel: SetupGuideViewModel
    // 中文: App UI 顯示語言 root state。注入在 AppRootView(ContentView + setup-guide cover 的共同祖先),
    // 中文: 讓 TabView 與全螢幕 cover 兩處都繼承同一份 store(plan D7 live-switch)。
    @State private var displayLanguageStore = DisplayLanguageStore()
    // Re-reads the persisted tag AND recomputes Automatic's effective language from the OS locale on
    // foreground, so a device-language change (while display = Automatic) takes effect without a relaunch.
    @Environment(\.scenePhase) private var scenePhase

    init(keyboardStatus: KeyboardStatusContext) {
        self.keyboardStatus = keyboardStatus
        _viewModel = StateObject(wrappedValue: SetupGuideViewModel(keyboardStatus: keyboardStatus))
    }

    var body: some View {
        ContentView(viewModel: viewModel)
            .onOpenURL { url in
                handleDeepLink(url)
            }
            .fullScreenCover(isPresented: $viewModel.shouldShowSetupGuide) {
                SetupGuideFullScreenView(
                    onComplete: {
                        viewModel.dismissSetupGuide()
                    },
                )
            }
            .task {
                viewModel.checkKeyboardStatus()
            }
            .onChange(of: scenePhase) { _, newPhase in
                if newPhase == .active {
                    displayLanguageStore.syncFromSettings()
                }
            }
            .environment(displayLanguageStore)
    }

    /// Handle deep link (e.g. taigikeyboard://settings).
    // 中文: 處理 taigikeyboard:// 開頭的 deep link;目前僅支援 host=settings 切到設定 tab。
    private func handleDeepLink(_ url: URL) {
        guard url.scheme == "taigikeyboard" else { return }

        if url.host == "settings" {
            NotificationCenter.default.post(
                name: .switchToSettingsTab,
                object: nil,
            )
        }
    }
}

// MARK: - Notification Names

extension Notification.Name {
    /// Switch to Settings tab via deep link.
    // 中文: deep link 觸發切換到設定 tab 的通知名稱。
    static let switchToSettingsTab = Notification.Name("switchToSettingsTab")
}
