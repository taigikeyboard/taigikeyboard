// Main app entry point: logger, Rust engine, KeyboardKit setup, navigation-bar appearance.

import KeyboardKit
import SwiftUI

/// Taigi Keyboard app entry point.
///
/// `init` installs, in order: LoggerFactory, the RustEngineBridge log sink (idempotent), the
/// Lexicon engine (this process's fst + dictionary binaries), the KeyboardKit App Group store
/// (`@AppStorage` prerequisite), the UINavigationBar font appearance (out of SwiftUI
/// `.environment` reach), and the first-run custom-dictionary seed.
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
struct AppRootView: View {
    @ObservedObject var keyboardStatus: KeyboardStatusContext
    @StateObject private var viewModel: SetupGuideViewModel
    // Display-language root state, injected at the common ancestor of ContentView and the
    // setup-guide cover so the TabView and the full-screen cover share one store (D7 live-switch).
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
    static let switchToSettingsTab = Notification.Name("switchToSettingsTab")
}
