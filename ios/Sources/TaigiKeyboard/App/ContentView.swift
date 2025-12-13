import SwiftUI
import KeyboardKit

struct ContentView: View {
    @State private var showSettings = false
    @State private var showDictionarySettings = false
    @State private var showCopyright = false
    @Binding var initialShowSettings: Bool
    @ObservedObject var viewModel: OnboardingViewModel

    var body: some View {
        NavigationView {
            GeometryReader { geometry in
                ZStack {
                    Color.Theme.surfacePrimary
                        .ignoresSafeArea()

                    ScrollView(.vertical, showsIndicators: false) {
                        VStack(spacing: 0) {
                            // Top spacing (matches Android layout)
                            Spacer()
                                .frame(height: max(80, geometry.safeAreaInsets.top + 64))

                            VStack(spacing: 16) {
                                // Card 1: Navigation (啟用方法 + 齒盤設定 + 詞庫管理)
                                NavigationCardView(
                                    viewModel: viewModel,
                                    showSettings: $showSettings,
                                    showDictionarySettings: $showDictionarySettings
                                )
                                .themedCard()

                                // Card 2: Resources (意見回饋 + 評分 + 分享)
                                ResourcesCardView()
                                .themedCard()

                                // Card 3: Copyright (版權聲明)
                                CopyrightCardView(showCopyright: $showCopyright)
                                .themedCard()
                            }
                            .padding(.horizontal, 20)
                            .frame(maxWidth: max(0, min(geometry.size.width - 40, 500)))
                            .frame(maxWidth: .infinity)

                            FooterView()
                                .padding(.top, 48)
                                .padding(.bottom, max(20, geometry.safeAreaInsets.bottom + 20))
                        }
                    }
                }
            }
            .navigationBarHidden(true)
            .sheet(isPresented: $showSettings) {
                SettingsView()
            }
            .sheet(isPresented: $showDictionarySettings) {
                DictionarySettingsView()
            }
            .sheet(isPresented: $showCopyright) {
                CopyrightView()
            }
        }
        .navigationViewStyle(.stack)
        .onChange(of: initialShowSettings) { _, newValue in
            if newValue {
                showSettings = true
                initialShowSettings = false
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        let bundleId = (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension"
        let keyboardStatus = KeyboardStatusContext(bundleId: bundleId)
        let viewModel = OnboardingViewModel(keyboardStatus: keyboardStatus)

        ContentView(
            initialShowSettings: .constant(false),
            viewModel: viewModel
        )
    }
}
