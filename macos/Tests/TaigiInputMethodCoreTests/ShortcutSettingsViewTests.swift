// The persistence and text contracts the 快捷鍵 pane is built on.

@testable import TaigiInputMethodCore
import XCTest

/// What a unit test can hold the pane to, given a SwiftUI form cannot be brought
/// up here: the raw values its `@AppStorage` bindings write, and the strings its
/// rows read. Whether each picker is actually wired to the right key, lists
/// every option, and tags them the right way round is not provable from outside
/// the view — that part is a render check (`docs/architecture/macos-roadmap.md`
/// dogfood list), not a case below.
@MainActor
final class ShortcutSettingsViewTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ShortcutSettingsViewTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    /// `@AppStorage` persists these raw values, so renaming a case silently
    /// resets every user who chose it back to the shipped default.
    func testBindingRawValues_stayStable() {
        XCTAssertEqual(ReturnKeyBehavior.allCases.map(\.rawValue), ["commitLiteral", "confirmHighlighted"])
        XCTAssertEqual(SpaceKeyBehavior.allCases.map(\.rawValue), ["confirmHighlighted", "nextCandidate"])
        XCTAssertEqual(BracketPagingBehavior.allCases.map(\.rawValue), ["enabled", "disabled"])
        XCTAssertEqual(TabCycleBehavior.allCases.map(\.rawValue), ["disabled", "enabled"])
        XCTAssertEqual(CandidateSlotModifier.allCases.map(\.rawValue), ["control", "option"])
    }

    /// A row whose text is missing in one language is a row that reads as an
    /// identifier — or as nothing — for the users who chose that language.
    func testEveryPaneString_resolvesInEveryDisplayLanguage() {
        for language in DisplayLanguage.selectableLanguages where language != .system {
            let store = TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
            for key in Self.paneStrings {
                let text = store.string(key)
                XCTAssertFalse(text.isEmpty, "\(key) has nothing to show in \(language)")
                XCTAssertNotEqual(text, key.rawValue, "\(key) fell back to its own identifier in \(language)")
            }
        }
    }

    /// Two rows of one picker reading alike would leave the user guessing which
    /// is which. The two pickers that share "send the highlighted candidate"
    /// share it on purpose — the key really does the same thing in both.
    func testPickerOptions_readDistinctlyWithinEachPicker() {
        let store = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        let pickers: [[StringKey]] = [
            [.macosBindingCommitLiteral, .macosBindingConfirmHighlighted],
            [.macosBindingConfirmHighlighted, .macosBindingNextCandidate],
            [.macosBindingTurnPage, .macosBindingTypeTheCharacter],
            [.macosBindingWalkCandidates, .macosBindingLeaveToApp],
        ]

        for options in pickers {
            let rows = options.map { store.string($0) }
            XCTAssertEqual(Set(rows).count, rows.count, "two options in one picker read the same: \(rows)")
        }
    }

    private static let paneStrings: [StringKey] = [
        .macosShortcutsTab,
        .macosShortcutsGlobalSection,
        .macosShortcutsComposingSection,
        .macosShortcutsFixedKeysNote,
        .macosBindingReturnKey,
        .macosBindingSpaceKey,
        .macosBindingBracketPaging,
        .macosBindingTabCycle,
        .macosBindingSlotModifier,
        .macosBindingCommitLiteral,
        .macosBindingConfirmHighlighted,
        .macosBindingNextCandidate,
        .macosBindingTurnPage,
        .macosBindingTypeTheCharacter,
        .macosBindingWalkCandidates,
        .macosBindingLeaveToApp,
        .macosBindingReturnFooter,
        .macosBindingSpaceFooter,
    ]
}
