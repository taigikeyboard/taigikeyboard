// The keys that pick a candidate out of the nine slots: the bare letters
// under Standard, the bare digits under Telex.

import AppKit
import Carbon.HIToolbox
@testable import TaigiInputMethodCore
import XCTest

/// What these pin: the shipped set, the window's labels against the keys
/// desktop-core picks with, and that no chord can sit on a slot key. Which
/// key picks which slot, and when, is desktop-core's (`keys/intent.rs`,
/// `keys/slot_key_set.rs`); the gate is asked through the seam
/// (`KeyRules`). The snapshots are built by hand where a layout is being
/// described: `⇧3` types `#` on a US layout.
@MainActor
final class CandidateSlotKeyTests: XCTestCase {
    private func snapshot(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        keyCode: Int? = nil,
    ) -> KeyEventSnapshot {
        KeyEventSnapshot(
            characters: characters,
            modifiers: modifiers,
            isNamedSpecialKey: false,
            keyCode: keyCode.map(UInt16.init),
        )
    }

    /// The set is the shipped one: a store with nothing recorded reads it.
    func testTheBareKeys_areTheShippedSet() throws {
        XCTAssertEqual(try makeScratchSettingsStore().toneInputScheme.slotKeySet, .bareKeys)
    }

    /// The labels the window draws are the keys the core picks with: the
    /// Shortcuts pane's slot row, which the core draws from its own slot key
    /// set, under each scheme — nine keys, none of them blank.
    func testTheWindowLabels_areTheCoresSlotKeys() throws {
        for scheme in ToneInputScheme.allCases {
            let store = try makeScratchSettingsStore()
            store.userDefaults.set(scheme.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)
            let labels = (0 ..< HorizontalPageLayout.pageSize).map { scheme.slotKeySet.label(forSlot: $0) }

            XCTAssertEqual(labels.count, 9)
            XCTAssertFalse(labels.contains(""), "\(scheme)")
            XCTAssertEqual(labels.joined(), try TestFixtures.composingShortcuts(in: store).slotKeys, "\(scheme)")
        }
    }

    // MARK: - What the recorder would store

    /// Every slot key of every set is a typing key to the gate, so no chord
    /// can ever sit on one — under either scheme, whichever is live when it
    /// is recorded.
    func testEverySlotKey_ofEverySet_isRefusedAsAChord() throws {
        for keySet in CandidateSlotKeySet.allCases {
            for slot in 0 ..< HorizontalPageLayout.pageSize {
                let key = keySet.label(forSlot: slot)
                // `bareKeysWithoutF` leaves its ninth slot unkeyed.
                if key.isEmpty {
                    continue
                }
                XCTAssertEqual(
                    try XCTUnwrap(KeyRules.chord(key: key, modifiers: [])),
                    .failure(.typesRomanization), "\(keySet) slot \(slot) (`\(key)`)",
                )
            }
        }
    }

    /// A shifted number-row key is refused as the digit it is, read off the
    /// key code since a US layout types `#` for it — the same answer the
    /// Carbon bridge gets when it hands the unmodified `3` with Shift, so a
    /// press no composing row can hold is one no global row bridges to.
    func testAShiftedDigit_isRefusedAsTheDigitItIs_onBothPaths() throws {
        XCTAssertEqual(
            KeyRules.press(snapshot("#", modifiers: .shift, keyCode: kVK_ANSI_3)),
            .refused(.typesRomanization),
        )
        XCTAssertEqual(try XCTUnwrap(KeyRules.chord(key: "3", modifiers: .shift)), .failure(.typesRomanization))
        // A symbol reached without the number row — a keypad or another
        // layout's own `#` key — is still the character it types.
        XCTAssertEqual(recorded(snapshot("#", modifiers: .shift)), "s|0023")
    }

    /// `⇧;` is the ninth slot key's Hanji/romanization chord. The character
    /// follows the selected layout: QWERTY `;`, Dvorak `z`, Colemak `p` positions.
    func testAShiftedSemicolon_isRefusedAsTheSlotKeyItIs() throws {
        for code in [kVK_ANSI_Semicolon, kVK_ANSI_Z, kVK_ANSI_P] {
            XCTAssertEqual(
                KeyRules.press(snapshot(":", modifiers: .shift, keyCode: code)),
                .refused(.typesRomanization),
            )
        }
        // A synthetic snapshot without a hardware key retains its character.
        XCTAssertEqual(recorded(snapshot(":", modifiers: .shift)), "s|003A")
    }

    /// Every other shifted key keeps the character it types, which is what
    /// its stored chords already hold.
    func testOtherShiftedKeys_stillRecordTheCharacterTheyType() throws {
        XCTAssertEqual(recorded(snapshot("{", modifiers: .shift)), "s|007B")
    }

    /// With Type - with f on under Standard, the window draws the core's
    /// shifted row: `f` gone, `z` on the fourth slot, the ninth blank — and
    /// pages eight, so that blank slot is never shown.
    func testTypeHyphenWithF_drawsTheCoresRowWithoutF() throws {
        let store = try makeScratchSettingsStore()
        store.userDefaults.set(true, forKey: SettingsStore.Keys.isFHyphenEnabled.name)
        XCTAssertEqual(store.candidateSlotKeySet, .bareKeysWithoutF)
        let labels = (0 ..< HorizontalPageLayout.pageSize).map { store.candidateSlotKeySet.label(forSlot: $0) }
        XCTAssertEqual(labels, ["q", "w", "d", "z", "x", "v", "y", ";", ""])
        XCTAssertEqual(labels.joined(), try TestFixtures.composingShortcuts(in: store).slotKeys)
        XCTAssertEqual(store.candidateSlotKeySet.pageSize, 8)
    }

    // MARK: - The labels

    func testTheLabels_nameTheKeysThemselves() {
        XCTAssertEqual((0 ..< 9).map { CandidateSlotKeySet.bareKeys.label(forSlot: $0) }, CandidateSlotKeySet.bareKeyRow)
        XCTAssertEqual(
            (0 ..< 9).map { CandidateSlotKeySet.digits.label(forSlot: $0) },
            ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
        )
    }

    /// The raw value `key` records as; nil when it records nothing.
    private func recorded(_ key: KeyEventSnapshot) -> String? {
        guard case let .recorded(chord)? = KeyRules.press(key) else { return nil }
        return chord.rawValue
    }
}
