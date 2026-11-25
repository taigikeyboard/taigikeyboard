import AppKit
import SwiftUI
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class SettingsRowLabelTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUp() async throws {
        try await super.setUp()
        suiteName = "SettingsRowLabelTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() async throws {
        userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    private func makeStore(_ language: DisplayLanguage = .hanji) -> DisplayLanguageStore {
        TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
    }

    func testCaptionResolution_prefersSelectedOptionAndFallsBackToRowKey() {
        let store = makeStore()
        for lang in DisplayLanguage.productionLanguages {
            store.setLanguage(lang)

            let numeric = SettingsRowLabel.caption(
                titleKey: .settingsToneInputScheme,
                selectedOptionKey: ToneInputScheme.standard.labelKey,
                in: store,
            )
            let telex = SettingsRowLabel.caption(
                titleKey: .settingsToneInputScheme,
                selectedOptionKey: ToneInputScheme.telex.labelKey,
                in: store,
            )
            XCTAssertEqual(numeric, store.caption(for: .settingsToneSchemeStandard), "\(lang.tag)")
            XCTAssertEqual(telex, store.caption(for: .settingsToneSchemeTelex), "\(lang.tag)")
            XCTAssertNotEqual(numeric, telex, "\(lang.tag)")

            let separator = SettingsRowLabel.caption(
                titleKey: .settingsSyllableSeparator,
                selectedOptionKey: SyllableSeparator.hyphen.displayNameKey,
                in: store,
            )
            XCTAssertEqual(separator, store.caption(for: .settingsSyllableSeparator), "\(lang.tag)")

            XCTAssertNil(
                SettingsRowLabel.caption(titleKey: .dictionaryCustomDictEnabled, in: store),
                "\(lang.tag)",
            )
        }
    }

    func testSettingsPanes_fitDetailWidthAcrossAllProductionLanguages() {
        let states: [(inputMode: InputMode, toneScheme: ToneInputScheme)] = [
            (.tl, .telex),
            (.tps, .standard),
        ]
        for lang in DisplayLanguage.productionLanguages {
            let store = makeStore(lang)
            for state in states {
                userDefaults.set(state.inputMode.rawValue, forKey: SettingsStore.Keys.inputMode.name)
                userDefaults.set(state.toneScheme.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)

                assertFitsDetailWidth(GeneralSettingsView(), store: store, context: "general-\(lang.tag)-\(state.inputMode.rawValue)")
                assertFitsDetailWidth(AppearanceSettingsView(), store: store, context: "appearance-\(lang.tag)-\(state.inputMode.rawValue)")
                assertFitsDetailWidth(
                    CustomDictionaryPage(client: FakeUserDataClient()),
                    store: store,
                    context: "customDict-\(lang.tag)-\(state.inputMode.rawValue)",
                )
            }
        }
    }

    func testSettingsRowLabel_rendersAndWrapsMultiLineCaptionsInGroupedForm() {
        for lang in DisplayLanguage.productionLanguages {
            let store = makeStore(lang)
            let uncaptionedHeight = assertFitsDetailWidth(
                Form {
                    Section {
                        Toggle(isOn: .constant(true)) {
                            SettingsRowLabel(titleKey: .settingsCAffricate)
                        }
                    }
                }
                .formStyle(.grouped),
                store: store,
                context: "uncaptioned-\(lang.tag)",
            ).height

            let singleLineHeight = assertFitsDetailWidth(
                Form {
                    Section {
                        Toggle(isOn: .constant(true)) {
                            SettingsRowLabel(titleKey: .settingsAutoSpace)
                        }
                    }
                }
                .formStyle(.grouped),
                store: store,
                context: "singleLine-\(lang.tag)",
            ).height

            let multiLineHeight = assertFitsDetailWidth(
                Form {
                    Section {
                        Picker(selection: .constant(SyllableSeparator.hyphen)) {
                            Text(store.string(.desktopTelexGuideHyphen)).tag(SyllableSeparator.hyphen)
                        } label: {
                            SettingsRowLabel(
                                titleKey: .settingsSyllableSeparator,
                                selectedOptionKey: SyllableSeparator.hyphen.displayNameKey,
                            )
                        }
                    }
                }
                .formStyle(.grouped),
                store: store,
                context: "multiLine-\(lang.tag)",
            ).height

            XCTAssertGreaterThan(singleLineHeight, uncaptionedHeight, "\(lang.tag) caption did not add row height")
            XCTAssertGreaterThan(multiLineHeight, singleLineHeight, "\(lang.tag) multi-line caption did not wrap")
        }
    }

    @discardableResult
    private func assertFitsDetailWidth(_ view: some View, store: DisplayLanguageStore, context: String) -> NSSize {
        let root = view
            .environment(store)
            .defaultAppStorage(userDefaults)
        let hosting = NSHostingView(rootView: root)
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: SettingsPaneLayout.detailWidth, height: 1000),
            styleMask: [.titled],
            backing: .buffered,
            defer: false,
        )
        window.contentView = hosting
        hosting.layoutSubtreeIfNeeded()
        window.layoutIfNeeded()
        let size = hosting.fittingSize
        XCTAssertLessThanOrEqual(
            size.width,
            SettingsPaneLayout.detailWidth + 1,
            "\(context) overflowed detailWidth",
        )
        XCTAssertGreaterThan(size.height, 0, "\(context) collapsed")
        for scrollView in TestFixtures.descendants(of: hosting, as: NSScrollView.self) {
            if let documentView = scrollView.documentView {
                XCTAssertLessThanOrEqual(
                    documentView.frame.width,
                    SettingsPaneLayout.detailWidth + 1,
                    "\(context) scroll document overflowed detailWidth",
                )
            }
        }
        return size
    }
}
