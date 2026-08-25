// The auto-space decisions as pure functions: gate, hyphen rule, and the
// one-mutation insert augmentation.

@testable import TaigiInputMethodCore
import XCTest

final class AutoSpacePolicyTests: XCTestCase {
    // MARK: - Punctuation classification (glyph parity with iOS/Android)

    func testAttachingSet_matchesTheCrossPlatformRoster() {
        // trace: the 19 glyphs of ios/Sources/TaigiKeyboard/Input/AutoSpacePunctuation.swift
        let attaching = ["。", "！", "？", ".", "!", "?", "，", ",", "、", "；", ";", "：", ":", ")", "）", "]", "】", "」", "』"]
        for glyph in attaching {
            XCTAssertTrue(AutoSpacePunctuation.isAttaching(glyph), "\(glyph) must attach")
        }
    }

    func testOpeningBracketsAndAmbiguousQuotes_doNotAttach() {
        for glyph in ["(", "（", "[", "「", "『", "\"", "'"] {
            XCTAssertFalse(AutoSpacePunctuation.isAttaching(glyph), "\(glyph) must not attach")
        }
    }

    func testNonPunctuationAndMultiCharacterText_doNotAttach() {
        for text in ["a", "台", " ", "", "?!", "guá?"] {
            XCTAssertFalse(AutoSpacePunctuation.isAttaching(text), "'\(text)' must not attach")
        }
    }

    // MARK: - Gate

    func testGate_isOffWheneverTheSettingIsOff() {
        XCTAssertFalse(AutoSpacePolicy.isGateActive(
            isAutoSpaceEnabled: false, wroteRomanization: true,
        ))
        XCTAssertFalse(AutoSpacePolicy.isGateActive(
            isAutoSpaceEnabled: false, wroteRomanization: false,
        ))
    }

    func testGate_followsWhetherTheCommitWroteRomanization() {
        XCTAssertTrue(AutoSpacePolicy.isGateActive(
            isAutoSpaceEnabled: true, wroteRomanization: true,
        ))
        XCTAssertFalse(AutoSpacePolicy.isGateActive(
            isAutoSpaceEnabled: true, wroteRomanization: false,
        ))
    }

    /// The primary rendering is the output mode — and 括號標註 counts, because
    /// `tâi-gí (台語)` HAS the romanization in it.
    /// trace: iOS isAutoSpaceModeActive (ActionHandler+KeyActions.swift:152-156)
    func testWritesRomanization_primaryFollowsTheOutputMode() {
        XCTAssertTrue(AutoSpacePolicy.writesRomanization(
            script: .primary, isTranslateSwapped: false, isOutputBothScripts: false,
        ))
        XCTAssertFalse(AutoSpacePolicy.writesRomanization(
            script: .primary, isTranslateSwapped: true, isOutputBothScripts: false,
        ))
        XCTAssertTrue(AutoSpacePolicy.writesRomanization(
            script: .primary, isTranslateSwapped: true, isOutputBothScripts: true,
        ))
    }

    /// The 漢羅 key inverts the mode — and 括號標註 does NOT apply to it, since
    /// Space writes one script and never the bracketed pair. Without that the
    /// gate would space a hanji written by Space in romanization mode.
    func testWritesRomanization_alternateInvertsTheModeAndIgnoresBrackets() {
        XCTAssertTrue(AutoSpacePolicy.writesRomanization(
            script: .alternate, isTranslateSwapped: true, isOutputBothScripts: false,
        ))
        for bothScripts in [false, true] {
            XCTAssertFalse(
                AutoSpacePolicy.writesRomanization(
                    script: .alternate,
                    isTranslateSwapped: false,
                    isOutputBothScripts: bothScripts,
                ),
                "bothScripts: \(bothScripts) — Space wrote the hanji, not the pair",
            )
        }
    }

    // MARK: - Trailing space after a commit

    func testCommittedWord_earnsTheSpace_butATrailingHyphenSuppressesIt() {
        XCTAssertTrue(AutoSpacePolicy.shouldAppendSpace(afterCommitting: "guá"))
        XCTAssertTrue(AutoSpacePolicy.shouldAppendSpace(afterCommitting: "tâi-gí"))
        // A trailing hyphen is a syllable the user is about to continue.
        XCTAssertFalse(AutoSpacePolicy.shouldAppendSpace(afterCommitting: "tai-"))
        XCTAssertFalse(AutoSpacePolicy.shouldAppendSpace(afterCommitting: ""))
    }

    // MARK: - Insert augmentation (punctuation mid-composition)

    func testAttachingPunctuation_ridesTheCommitWithTheSpaceAfterIt() {
        // trace: "goá" composing + "?" → document "goá? " in one mutation
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert("?", afterComposition: "goá", isGateActive: true),
            AutoSpacePolicy.AugmentedInsert(text: "? ", leavesTrailingAutoSpace: true),
        )
    }

    func testNonAttachingText_getsTheLeadingSpaceInstead() {
        // trace: "goá" composing + "(" → "goá (" — opening brackets keep the
        // LEADING space (iOS S10)
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert("(", afterComposition: "goá", isGateActive: true),
            AutoSpacePolicy.AugmentedInsert(text: " (", leavesTrailingAutoSpace: false),
        )
    }

    func testATypedSpace_isNotDoubled_butStillArmsTheSwap() {
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert(" ", afterComposition: "goá", isGateActive: true),
            AutoSpacePolicy.AugmentedInsert(text: " ", leavesTrailingAutoSpace: true),
        )
    }

    func testATrailingHyphen_suppressesTheAugmentation() {
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert("?", afterComposition: "tai-", isGateActive: true),
            AutoSpacePolicy.AugmentedInsert(text: "?", leavesTrailingAutoSpace: false),
        )
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert("(", afterComposition: "tai-", isGateActive: true),
            AutoSpacePolicy.AugmentedInsert(text: "(", leavesTrailingAutoSpace: false),
        )
    }

    func testAnInactiveGate_leavesTheInsertAlone() {
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert("?", afterComposition: "goá", isGateActive: false),
            AutoSpacePolicy.AugmentedInsert(text: "?", leavesTrailingAutoSpace: false),
        )
        XCTAssertEqual(
            AutoSpacePolicy.augmentInsert(" ", afterComposition: "goá", isGateActive: false),
            AutoSpacePolicy.AugmentedInsert(text: " ", leavesTrailingAutoSpace: false),
        )
    }

    // MARK: - Stored default

    @MainActor
    func testAFreshInstall_startsWithAutoSpaceOn() throws {
        // macOS's own default (USER 2026-08-23) — iOS and Android start OFF.
        let store = try makeScratchSettingsStore()
        XCTAssertTrue(store.isAutoSpaceEnabled)

        store.isAutoSpaceEnabled = false
        XCTAssertFalse(store.isAutoSpaceEnabled, "an explicit OFF must stick")
    }
}
