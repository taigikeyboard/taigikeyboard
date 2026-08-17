// Which dictionaries the keyboard draws candidates from.

import AppKit
import SwiftUI

/// The dictionary source toggles, in the sections iOS groups them into
/// (`ios/.../App/Tabs/Dictionary/DictionaryTab.swift`).
///
/// `@AppStorage` per toggle rather than one snapshot object: each row is an
/// independent setting the engine live-reads, and binding them individually is
/// what makes a `defaults write` — or a future settings import — show up in the
/// form without anything having to be told.
struct DictionaryTogglesView: View {
    @AppStorage(SettingsStore.Keys.isKautianEnabled.name)
    private var isKautianEnabled = SettingsStore.Keys.isKautianEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaigitvEnabled.name)
    private var isTaigitvEnabled = SettingsStore.Keys.isTaigitvEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKunggeEnabled.name)
    private var isKunggeEnabled = SettingsStore.Keys.isKunggeEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isSttiEnabled.name)
    private var isSttiEnabled = SettingsStore.Keys.isSttiEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isItaigiEnabled.name)
    private var isItaigiEnabled = SettingsStore.Keys.isItaigiEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaijitEnabled.name)
    private var isTaijitEnabled = SettingsStore.Keys.isTaijitEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isTaihoaEnabled.name)
    private var isTaihoaEnabled = SettingsStore.Keys.isTaihoaEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isSitbutEnabled.name)
    private var isSitbutEnabled = SettingsStore.Keys.isSitbutEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isVariantEnabled.name)
    private var isVariantEnabled = SettingsStore.Keys.isVariantEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKhiinEnabled.name)
    private var isKhiinEnabled = SettingsStore.Keys.isKhiinEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKhpooEnabled.name)
    private var isKhpooEnabled = SettingsStore.Keys.isKhpooEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isLkkEnabled.name)
    private var isLkkEnabled = SettingsStore.Keys.isLkkEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isDevEnabled.name)
    private var isDevEnabled = SettingsStore.Keys.isDevEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isKautianAccentLukangEnabled.name)
    private var isLukangEnabled = SettingsStore.Keys.isKautianAccentLukangEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentSansiaEnabled.name)
    private var isSansiaEnabled = SettingsStore.Keys.isKautianAccentSansiaEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTaipakEnabled.name)
    private var isTaipakEnabled = SettingsStore.Keys.isKautianAccentTaipakEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentGilanEnabled.name)
    private var isGilanEnabled = SettingsStore.Keys.isKautianAccentGilanEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTainanEnabled.name)
    private var isTainanEnabled = SettingsStore.Keys.isKautianAccentTainanEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentKaohsiungEnabled.name)
    private var isKaohsiungEnabled = SettingsStore.Keys.isKautianAccentKaohsiungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentKinmenEnabled.name)
    private var isKinmenEnabled = SettingsStore.Keys.isKautianAccentKinmenEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentMakungEnabled.name)
    private var isMakungEnabled = SettingsStore.Keys.isKautianAccentMakungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentSintikEnabled.name)
    private var isSintikEnabled = SettingsStore.Keys.isKautianAccentSintikEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianAccentTaichungEnabled.name)
    private var isTaichungEnabled = SettingsStore.Keys.isKautianAccentTaichungEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isKautianNameAppendixEnabled.name)
    private var isNameAppendixEnabled = SettingsStore.Keys.isKautianNameAppendixEnabled.defaultValue

    var body: some View {
        Form {
            Section {
                Toggle("教育部臺灣台語常用詞辭典", isOn: $isKautianEnabled)
                Group {
                    Toggle("鹿港偏泉腔", isOn: $isLukangEnabled)
                    Toggle("三峽偏泉腔", isOn: $isSansiaEnabled)
                    Toggle("臺北偏泉腔", isOn: $isTaipakEnabled)
                    Toggle("宜蘭偏漳腔", isOn: $isGilanEnabled)
                    Toggle("臺南混合腔", isOn: $isTainanEnabled)
                    Toggle("高雄混合腔", isOn: $isKaohsiungEnabled)
                    Toggle("金門偏泉腔", isOn: $isKinmenEnabled)
                    Toggle("馬公偏泉腔", isOn: $isMakungEnabled)
                    Toggle("新竹偏泉腔", isOn: $isSintikEnabled)
                    Toggle("臺中偏漳腔", isOn: $isTaichungEnabled)
                    Toggle("姓名附錄", isOn: $isNameAppendixEnabled)
                }
                .padding(.leading, Metrics.subcollectionIndent)
                // Disabled, not cleared: the master switch says whether this
                // dictionary is searched at all, and turning it back on has to
                // return the 腔口 the user had chosen rather than all of them.
                .disabled(!isKautianEnabled)

                Toggle("台語新詞辭庫", isOn: $isTaigitvEnabled)
                Toggle("台語工藝詞庫", isOn: $isKunggeEnabled)
                Toggle("學科術語辭典", isOn: $isSttiEnabled)
            } header: {
                Text("教育部")
            } footer: {
                ExternalLinkButton(title: "教典網站", url: Self.kautianURL)
            }

            Section {
                Toggle("iTaigi 華台對照典", isOn: $isItaigiEnabled)
                Toggle("台日大辭典", isOn: $isTaijitEnabled)
                Toggle("台華線頂對照典", isOn: $isTaihoaEnabled)
                Toggle("台灣植物名彙", isOn: $isSitbutEnabled)
            } header: {
                Text("其他辭典")
            }

            Section {
                Toggle("異用字", isOn: $isVariantEnabled)
                Toggle("在來字", isOn: $isKhiinEnabled)
                Toggle("腔口補充資料", isOn: $isKhpooEnabled)
                Toggle("LKK 漢羅合用建議用字", isOn: $isLkkEnabled)
                Toggle("詞庫增補檔案", isOn: $isDevEnabled)
            } header: {
                Text("補充資料")
            } footer: {
                Text("關掉全部辭典,拍字就袂有詞庫候選。")
            }
        }
        .formStyle(.grouped)
    }

    private enum Metrics {
        static let subcollectionIndent: CGFloat = 16
    }

    private static let kautianURL = URL(string: "https://sutian.moe.edu.tw/")
}

/// A link out to the web, with the failure shown rather than swallowed.
///
/// `NSWorkspace.open` answers `false` when nothing could handle the URL, and a
/// button that silently does nothing is indistinguishable from a broken one.
struct ExternalLinkButton: View {
    let title: String
    let url: URL?

    @State private var didFail = false

    var body: some View {
        Button {
            guard let url, NSWorkspace.shared.open(url) else {
                didFail = true
                return
            }
        } label: {
            Label(title, systemImage: "arrow.up.forward.square")
        }
        .buttonStyle(.link)
        .alert("拍袂開網頁", isPresented: $didFail) {
            Button("好") {}
        } message: {
            Text(url?.absoluteString ?? "")
        }
    }
}
