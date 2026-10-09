// The launch bring-up and the settings snapshot over the desktop shell seam
// (docs/architecture/macos-desktop-core-roadmap.md P6). The journal, the
// once-only Prepare and the refusals are pinned natively too, in
// `macos/crates/taigi-macos-ffi`.

import Foundation
@testable import TaigiInputMethodCore
import XCTest

final class DesktopCoreRuntimeTests: XCTestCase {
    private typealias Keys = SettingsStore.Keys

    private var suiteName = ""
    private var userDefaults: UserDefaults!

    override func setUpWithError() throws {
        suiteName = "DesktopCoreRuntimeTests-\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
    }

    private func descriptors() throws -> [Taigi_DesktopShell_SettingDescriptor] {
        try XCTUnwrap(TestDesktopCore.runtime, "Configure failed").settings
    }

    private func snapshotValue(_ name: String) throws -> Taigi_DesktopShell_SettingValue.OneOf_Value? {
        try DesktopCoreRuntime.settingsSnapshot(descriptors(), in: userDefaults)
            .entries.first { $0.name == name }?.value.value
    }

    // MARK: - Bring-up

    func testPrepare_repositoryDictionaries_loadsRecords() throws {
        let stats = try XCTUnwrap(
            InstalledLexicon.installOnce(),
            "Prepare returned no stats, so the engine has no lexicon",
        )

        // Exact counts change with every dictionary rebuild; what must hold is
        // that the engine read something. A zero here is the failure mode this
        // test exists for — an install that reports success having mapped an
        // empty or wrong file looks identical from the outside until the user
        // types and sees no candidates.
        XCTAssertGreaterThan(stats.dictionaryRecordCount, 0)
        XCTAssertGreaterThan(stats.prefixIndexEntryCount, 0)
    }

    /// Once per process: a repeat answers the first result.
    func testPrepare_repeated_answersTheFirstResult() throws {
        let runtime = try XCTUnwrap(TestDesktopCore.runtime)
        XCTAssertEqual(runtime.prepare(), runtime.prepare())
    }

    func testConfigure_secondTime_isRefused() throws {
        _ = try XCTUnwrap(TestDesktopCore.runtime, "the process's one Configure")
        let again = DesktopCoreRuntime.configure(
            .init(dataDirectory: nil, dictionariesDirectory: nil, dictionaryStamp: 1, systemLocale: ""),
        )
        XCTAssertNil(again)
    }

    /// An OK response carrying another kind of reply is a failure, not a
    /// default-valued reply of the kind asked for.
    func testRoundtrip_otherReplyKind_answersNil() {
        let version = DesktopCoreBridge.roundtrip(.version(.init()), op: "test") {
            if case let .version(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        XCTAssertNotNil(version)
        let mismatched = DesktopCoreBridge.roundtrip(.version(.init()), op: "test") {
            if case let .prepare(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        XCTAssertNil(mismatched)
    }

    /// A one-entry snapshot.
    private static func snapshot(_ name: String, boolean: Bool) -> Taigi_DesktopShell_SettingsSnapshot {
        var entry = Taigi_DesktopShell_SettingEntry()
        entry.name = name
        entry.value.boolean = boolean
        var snapshot = Taigi_DesktopShell_SettingsSnapshot()
        snapshot.entries = [entry]
        return snapshot
    }

    /// The snapshot rides in the request's own envelope — one round trip,
    /// applied with the request or not at all.
    func testEnvelope_carriesTheSnapshotWithTheRequest() throws {
        let snapshot = Self.snapshot(Keys.isAutoSpaceEnabled.name, boolean: true)
        let envelope = DesktopCoreBridge.envelope(.prepare(.init()), settings: snapshot)
        XCTAssertEqual(envelope.request, .prepare(.init()))
        XCTAssertEqual(envelope.settings, snapshot)
        XCTAssertFalse(DesktopCoreBridge.envelope(.prepare(.init()), settings: nil).hasSettings)
    }

    /// A snapshot the core refuses refuses its request: no reply.
    func testRoundtrip_refusedSnapshot_answersNil() {
        let snapshot = Self.snapshot("notASetting", boolean: true)
        let version = DesktopCoreBridge.roundtrip(.version(.init()), settings: snapshot, op: "test") {
            if case let .version(reply) = $0 {
                reply
            } else {
                nil
            }
        }
        XCTAssertNil(version)
    }

    // trace: Info.plist CFBundleVersion is a plain integer string ("30613");
    // the parse is `UInt32.init`, falling back to 1.
    func testDictionaryStamp_isTheBundleVersion() {
        let stamp = DesktopCoreRuntime.Configuration.dictionaryStamp
        XCTAssertEqual(stamp(["CFBundleVersion": "30613"]), 30613)
        XCTAssertEqual(stamp(["CFBundleVersion": "0"]), 0)
        XCTAssertEqual(stamp(["CFBundleVersion": "3.6.13"]), 1)
        XCTAssertEqual(stamp(["CFBundleVersion": 30613]), 1, "not a string")
        XCTAssertEqual(stamp(nil), 1)
    }

    // MARK: - The settings whitelist

    /// The core's whitelist names exactly the key-path settings this side
    /// stores, each with the default `SettingsStore` reads when it is absent.
    func testWhitelist_matchesSettingsStoreDefaults() throws {
        let booleans = [
            Keys.isHanjiFirst, Keys.isAutoSpaceEnabled, Keys.isCandidateWindowEnabled,
            Keys.isLiteralRomanCandidateEnabled, Keys.isNasalMarkerUppercaseEnabled, Keys.isCustomDictEnabled,
            Keys.isKautianEnabled, Keys.isTaigitvEnabled, Keys.isItaigiEnabled, Keys.isSitbutEnabled,
            Keys.isTaihoaEnabled, Keys.isTaijitEnabled, Keys.isKunggeEnabled, Keys.isSttiEnabled,
            Keys.isKhpooEnabled, Keys.isVariantEnabled, Keys.isKhiinEnabled, Keys.isLkkEnabled,
            Keys.isDevEnabled,
            Keys.isKautianAccentLukangEnabled, Keys.isKautianAccentSansiaEnabled,
            Keys.isKautianAccentTaipakEnabled, Keys.isKautianAccentGilanEnabled,
            Keys.isKautianAccentTainanEnabled, Keys.isKautianAccentKaohsiungEnabled,
            Keys.isKautianAccentKinmenEnabled, Keys.isKautianAccentMakungEnabled,
            Keys.isKautianAccentSintikEnabled, Keys.isKautianAccentTaichungEnabled,
            Keys.isKautianNameAppendixEnabled, Keys.isKautianAltReadingEnabled,
        ]
        var expected: [String: Taigi_DesktopShell_SettingValue.OneOf_Value?] = [
            Keys.inputMode.name: .text(Keys.inputMode.defaultValue.rawValue),
            Keys.candidateDisplayMode.name: .text(Keys.candidateDisplayMode.defaultValue.rawValue),
            Keys.syllableSeparator.name: .text(Keys.syllableSeparator.defaultValue.rawValue),
            Keys.toneInputScheme.name: .text(Keys.toneInputScheme.defaultValue.rawValue),
        ]
        for key in booleans {
            expected[key.name] = .boolean(key.defaultValue)
        }
        // A chord row has no stored default: absent is the action's own chord.
        // Spelled out: renaming one would drop every chord stored under it.
        for action in [
            "nextCandidate", "previousCandidate", "pageForward", "pageBackward",
            "confirmHighlighted", "commitLiteral", "commitAlternateScript",
        ] {
            expected["composingShortcut.\(action)"] = .some(nil)
        }

        let described = try descriptors()
        XCTAssertEqual(Set(described.map(\.name)), Set(expected.keys))
        XCTAssertEqual(described.count, expected.count, "a name described twice")
        for descriptor in described {
            let core = descriptor.hasDefaultValue ? descriptor.defaultValue.value : nil
            XCTAssertEqual(core, expected[descriptor.name] ?? nil, descriptor.name)
        }
    }

    // MARK: - The snapshot

    /// A boolean reaches the core as `SettingsStore` reads it — a stored 0 or
    /// 1 number as `false` / `true`, anything else it would not accept left
    /// out, so the core's default applies exactly where the Swift reader's
    /// does (inventory S26).
    func testSnapshot_booleanReadsAsSettingsStoreDoes() throws {
        let key = Keys.isHanjiFirst
        let stored: [Any] = [true, false, 0, 1, 2, 0.0, 1.0, 0.5, "true", "1"]
        for value in stored {
            userDefaults.set(value, forKey: key.name)
            let swift = SettingsStore(userDefaults: userDefaults).storedIsHanjiFirst
            let sent = try snapshotValue(key.name)
            if case .text = sent {
                XCTFail("stored \(value): a boolean sent as text")
            }
            let core = if case let .boolean(sent) = sent {
                sent
            } else {
                key.defaultValue
            }
            XCTAssertEqual(core, swift, "stored \(value)")
        }
    }

    /// Text goes as `string(forKey:)` reads it; a cleared chord row (`""`)
    /// stays distinct from an absent one.
    func testSnapshot_textKeepsAClearedChordApartFromAnAbsentOne() throws {
        let chord = "composingShortcut.pageForward"
        XCTAssertNil(try snapshotValue(chord))
        userDefaults.set("", forKey: chord)
        XCTAssertEqual(try snapshotValue(chord), .text(""))
        userDefaults.set("o|005D", forKey: chord)
        XCTAssertEqual(try snapshotValue(chord), .text("o|005D"))
        userDefaults.set("poj", forKey: Keys.inputMode.name)
        XCTAssertEqual(try snapshotValue(Keys.inputMode.name), .text("poj"))
    }

    /// Rebuilt on every push: a change is in the next one, a removal takes
    /// the entry out again (the core then reads the default).
    func testSnapshot_followsChangesAndRemovals() throws {
        let name = Keys.isAutoSpaceEnabled.name
        XCTAssertNil(try snapshotValue(name))
        userDefaults.set(true, forKey: name)
        XCTAssertEqual(try snapshotValue(name), .boolean(true))
        userDefaults.removeObject(forKey: name)
        XCTAssertNil(try snapshotValue(name))
    }

    /// Nothing outside the whitelist is sent.
    func testSnapshot_leavesOtherSettingsOut() throws {
        userDefaults.set("en", forKey: Keys.displayLanguage.name)
        XCTAssertNil(try snapshotValue(Keys.displayLanguage.name))
    }
}
