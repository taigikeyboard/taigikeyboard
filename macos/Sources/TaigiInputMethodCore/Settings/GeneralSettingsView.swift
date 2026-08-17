// The 一般 tab: every setting the composing engine reads, plus the shortcut
// recorders.

import KeyboardShortcuts
import SwiftUI

/// The general half of the settings window.
///
/// Bound with `@AppStorage` rather than through `SettingsStore`, so the form
/// re-renders when a value is changed from outside it — the input-source menu's
/// TL/POJ items and the shortcut hotkeys write straight to `UserDefaults`, and
/// so does anyone running `defaults write`. The keys and the fallbacks come
/// from the store's descriptors, so the form and the engine cannot disagree
/// about either.
///
/// The labels are Traditional-Chinese string literals, which SwiftUI reads as
/// localization keys: this package has no string catalog, so they render as
/// written, and adding one later needs no change here. macOS is not part of the
/// `i18n/` pipeline the iOS and Android apps are built from.
struct GeneralSettingsView: View {
    /// The form is a fixed-width column of controls — widening it would only
    /// add empty space. Read by `SettingsTabViewController` as this tab's
    /// window floor, so the window never opens narrower than its own content.
    ///
    /// `nonisolated` because that reader is the tab controller's plain
    /// `ContentTab` enum: a `View`'s statics are main-actor-isolated by
    /// default, and a constant needs no isolation to be safe.
    nonisolated static let formWidth: CGFloat = 380

    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    @AppStorage(SettingsStore.Keys.isTranslateSwapped.name)
    private var isTranslateSwapped = SettingsStore.Keys.isTranslateSwapped.defaultValue

    @AppStorage(SettingsStore.Keys.isOutputBothScripts.name)
    private var isOutputBothScripts = SettingsStore.Keys.isOutputBothScripts.defaultValue

    @AppStorage(SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    private var isLiteralRomanCandidateEnabled = SettingsStore.Keys.isLiteralRomanCandidateEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isFrequencyRecordingEnabled.name)
    private var isFrequencyRecordingEnabled = SettingsStore.Keys.isFrequencyRecordingEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isAssociationRecordingEnabled.name)
    private var isAssociationRecordingEnabled = SettingsStore.Keys.isAssociationRecordingEnabled.defaultValue

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
                Toggle("漢羅對調", isOn: $isTranslateSwapped)
                Toggle("漢羅並列", isOn: $isOutputBothScripts)
                Toggle("顯示羅馬字候選", isOn: $isLiteralRomanCandidateEnabled)
            } header: {
                Text("候選")
            }

            Section {
                Toggle("記錄選字詞頻", isOn: $isFrequencyRecordingEnabled)
                Toggle("記錄詞語關聯", isOn: $isAssociationRecordingEnabled)
            } header: {
                Text("學習")
            } footer: {
                Text("學習資料只存在本機,袂上傳。")
            }

            Section {
                // One row per action, off the same list the hotkey registration
                // uses, so a new action cannot appear in one and not the other.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    KeyboardShortcuts.Recorder(action.label, name: action.name) { _ in
                        ShortcutConflicts.resolve(after: action)
                    }
                }
            } header: {
                Text("快捷鍵")
            } footer: {
                Text("快捷鍵只在台語鍵盤使用中有效。")
            }
        }
        .formStyle(.grouped)
        .frame(width: Self.formWidth)
        .fixedSize(horizontal: false, vertical: true)
    }
}
