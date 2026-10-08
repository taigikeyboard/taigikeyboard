@testable import TaigiKeyboard
import XCTest

/// Tests for the Layout tab's `LayoutChoice`: shelf grouping, which card shows the checkmark, and
/// the layout + input-mode writes a card applies through the TPS state machine.
final class LayoutChoiceTests: XCTestCase {
    private var suiteName: String!
    private var defaults: UserDefaults!
    private var settings: SharedSettings!

    override func setUp() {
        super.setUp()
        suiteName = "LayoutChoiceTests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
        settings = SharedSettings(userDefaults: defaults)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        settings = nil
        suiteName = nil
        super.tearDown()
    }

    func testLayoutChoice_shelves_splitLayoutsBySharedKeyTable() {
        let shelves = LayoutChoice.shelves.map { shelf in
            (shelf.titleKey, shelf.choices.map { "\($0.layout.rawValue)/\($0.script.map { "\($0)" } ?? "-")" })
        }
        XCTAssertEqual(shelves.map(\.0), [.layoutCommonLayoutsSection, .settingsTlMode, .settingsPojMode, .settingsTpsMode])
        XCTAssertEqual(shelves.map(\.1), [
            ["phahTaigi/-"],
            ["qwerty/tl", "moe1/tl", "moe2/tl"],
            ["qwerty/poj", "moe1/poj", "moe2/poj"],
            ["tps/-"],
        ])
    }

    func testLayoutChoice_previewImageName_pojCardShowsPojTable() {
        XCTAssertEqual(LayoutChoice(layout: .moe2, script: .poj).previewImageName, "layout_moe2_poj_preview")
        XCTAssertEqual(LayoutChoice(layout: .moe2, script: .tl).previewImageName, "layout_moe2_preview")
        XCTAssertEqual(LayoutChoice(layout: .phahTaigi, script: nil).previewImageName, "layout_phahtaigi_preview")
    }

    func testKeyboardLayoutType_previewImageNameForMode_pojOnlyWhereTableDiffers() {
        XCTAssertEqual(KeyboardLayoutType.qwerty.previewImageName(for: .poj), "layout_standard_poj_preview")
        XCTAssertEqual(KeyboardLayoutType.moe2.previewImageName(for: .poj), "layout_moe2_poj_preview")
        XCTAssertEqual(KeyboardLayoutType.moe2.previewImageName(for: .tl), "layout_moe2_preview")
        XCTAssertEqual(KeyboardLayoutType.tps.previewImageName(for: .poj), "layout_tps_preview")
        XCTAssertEqual(KeyboardLayoutType.phahTaigi.previewImageName(for: .poj), "layout_phahtaigi_preview")
        XCTAssertEqual(KeyboardLayoutType.moe1.previewImageName(for: .english), "layout_moe1_preview")
    }

    func testLayoutChoice_isSelected_scriptCardsNeedTheirOwnMode() {
        let cases: [(LayoutChoice, KeyboardLayoutType, InputMode, Bool)] = [
            (LayoutChoice(layout: .qwerty, script: .tl), .qwerty, .tl, true),
            (LayoutChoice(layout: .qwerty, script: .tl), .qwerty, .poj, false),
            (LayoutChoice(layout: .qwerty, script: .tl), .qwerty, .english, false),
            (LayoutChoice(layout: .qwerty, script: .poj), .qwerty, .poj, true),
            (LayoutChoice(layout: .qwerty, script: .poj), .moe1, .poj, false),
            (LayoutChoice(layout: .phahTaigi, script: nil), .phahTaigi, .poj, true),
            (LayoutChoice(layout: .phahTaigi, script: nil), .phahTaigi, .english, true),
            (LayoutChoice(layout: .tps, script: nil), .tps, .tps, true),
        ]
        for (choice, layout, mode, expected) in cases {
            XCTAssertEqual(
                choice.isSelected(layout: layout, inputMode: mode),
                expected,
                "\(choice) with layout=\(layout) mode=\(mode)",
            )
        }
    }

    func testLayoutChoice_apply_pojCardSetsLayoutAndPojMode() {
        LayoutChoice(layout: .moe1, script: .poj).apply(to: settings)
        XCTAssertEqual(settings.keyboardLayoutType, .moe1)
        XCTAssertEqual(settings.inputMode, .poj)

        LayoutChoice(layout: .moe1, script: .tl).apply(to: settings)
        XCTAssertEqual(settings.keyboardLayoutType, .moe1)
        XCTAssertEqual(settings.inputMode, .tl)
    }

    func testLayoutChoice_apply_englishModeSwitchesToCardScript() {
        settings.inputMode = .english
        LayoutChoice(layout: .qwerty, script: .tl).apply(to: settings)
        XCTAssertEqual(settings.inputMode, .tl)
    }

    func testLayoutChoice_apply_leavingTpsViaPojCardEndsInPoj() {
        // trace: tl → TPS card saves inputModeBeforeTps=tl; POJ card: layout write restores tl, then poj.
        LayoutChoice(layout: .tps, script: nil).apply(to: settings)
        XCTAssertEqual(settings.inputMode, .tps)

        LayoutChoice(layout: .qwerty, script: .poj).apply(to: settings)
        XCTAssertEqual(settings.keyboardLayoutType, .qwerty)
        XCTAssertEqual(settings.inputMode, .poj)
    }

    func testLayoutChoice_apply_universalCardKeepsModeAndRestoresAfterTps() {
        settings.inputMode = .poj
        LayoutChoice(layout: .phahTaigi, script: nil).apply(to: settings)
        XCTAssertEqual(settings.inputMode, .poj)

        LayoutChoice(layout: .tps, script: nil).apply(to: settings)
        LayoutChoice(layout: .phahTaigi, script: nil).apply(to: settings)
        XCTAssertEqual(settings.keyboardLayoutType, .phahTaigi)
        XCTAssertEqual(settings.inputMode, .poj)
    }
}
