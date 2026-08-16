// The settings form: every setting the composing engine reads, and nothing else.

import SwiftUI

/// The whole user-facing settings surface of the macOS input method.
///
/// Bound with `@AppStorage` rather than through `SettingsStore`, so the form
/// re-renders when a value is changed from outside it — the input-source menu's
/// TL/POJ items write straight to `UserDefaults`, and so does anyone running
/// `defaults write`. The keys and the fallbacks come from the store's
/// descriptors, so the form and the engine cannot disagree about either.
///
/// The labels are Traditional-Chinese string literals, which SwiftUI reads as
/// localization keys: this package has no string catalog, so they render as
/// written, and adding one later needs no change here. macOS is not part of the
/// `i18n/` pipeline the iOS and Android apps are built from.
struct SettingsView: View {
    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    @AppStorage(SettingsStore.Keys.isDoubleTapOOEnabled.name)
    private var isDoubleTapOOEnabled = SettingsStore.Keys.isDoubleTapOOEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isDoubleTapNNEnabled.name)
    private var isDoubleTapNNEnabled = SettingsStore.Keys.isDoubleTapNNEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isTranslateSwapped.name)
    private var isTranslateSwapped = SettingsStore.Keys.isTranslateSwapped.defaultValue

    @AppStorage(SettingsStore.Keys.isOutputBothScripts.name)
    private var isOutputBothScripts = SettingsStore.Keys.isOutputBothScripts.defaultValue

    @AppStorage(SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    private var isLiteralRomanCandidateEnabled = SettingsStore.Keys.isLiteralRomanCandidateEnabled.defaultValue

    var body: some View {
        Form {
            Section {
                Picker("羅馬字系統", selection: $inputMode) {
                    Text("台羅 (TL)").tag(InputMode.tl)
                    Text("白話字 (POJ)").tag(InputMode.poj)
                }
                .pickerStyle(.radioGroup)
            }

            Section {
                Toggle("連打 oo 轉 o͘", isOn: $isDoubleTapOOEnabled)
                Toggle("連打 nn 轉 ⁿ", isOn: $isDoubleTapNNEnabled)
            } header: {
                Text("輸入")
            }

            Section {
                Toggle("漢羅對調", isOn: $isTranslateSwapped)
                Toggle("漢羅並列", isOn: $isOutputBothScripts)
                Toggle("顯示羅馬字候選", isOn: $isLiteralRomanCandidateEnabled)
            } header: {
                Text("候選")
            }
        }
        .formStyle(.grouped)
        .frame(width: Metrics.formWidth)
        .fixedSize(horizontal: false, vertical: true)
    }

    private enum Metrics {
        static let formWidth: CGFloat = 380
    }
}
