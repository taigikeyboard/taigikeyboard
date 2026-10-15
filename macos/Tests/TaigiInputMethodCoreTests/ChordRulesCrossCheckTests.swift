// The Shortcuts pane's composing rows, through the seam, against the table
// desktop-core is held to.

@testable import TaigiInputMethodCore
import XCTest

/// The Swift half of the chord-rule cross-check: every `default` and
/// `resolve` row of `macos/crates/taigi-macos-ffi/fixtures/chord_rules.tsv`
/// read the way the Shortcuts pane reads its rows — written to a defaults
/// suite as `SettingsStore` writes them, carried to the core in the request's
/// settings snapshot, resolved there (`KeyRules.composingShortcuts(in:)`).
/// The rules themselves are the core's, and every row — `parse` and `match`
/// too — is checked against them natively (`src/chord_rules.rs`); this half
/// holds the stored form and the snapshot to the same hand-written answers.
@MainActor
final class ChordRulesCrossCheckTests: XCTestCase {
    func testThePaneReadsEveryDefaultAndResolveRowOfTheMacChordTable() throws {
        let table = try String(contentsOf: TestFixtures.chordRulesURL, encoding: .utf8)
        let rows = table.split(separator: "\n")
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }
            .map { $0.split(separator: "\t", omittingEmptySubsequences: false).map(String.init) }
        var kinds: [String: Int] = [:]
        let shipped = try TestFixtures.composingShortcuts(in: makeScratchSettingsStore())
        let fields = shipped.rows.flatMap(\.fields)
        for row in rows {
            switch row[0] {
            case "default":
                kinds[row[0], default: 0] += 1
                let field = try XCTUnwrap(fields.first { $0.settingsKey == Self.settingsKey(row[1]) }, row[1])
                XCTAssertEqual(field.shippedChord?.rawValue ?? "-", row[2], "\(row)")
            case "resolve":
                kinds[row[0], default: 0] += 1
                XCTAssertEqual(try resolved(row[1]), row[2], "\(row)")
            case "parse", "match":
                continue
            default:
                XCTFail("unknown row kind \(row[0])")
            }
        }
        XCTAssertEqual(kinds["default"], fields.count, "every field has its default row")
        XCTAssertGreaterThan(kinds["resolve", default: 0], 0)
    }

    /// A field's settings key from the name the table gives it
    /// (`confirmHighlighted.second`).
    private static let prefix = "composingShortcut."

    private static func settingsKey(_ name: String) -> String {
        prefix + name
    }

    /// Every field as the core resolves it from a defaults suite holding
    /// `stored`.
    private func resolved(_ stored: String) throws -> String {
        let store = try makeScratchSettingsStore()
        if stored != "-" {
            for assignment in stored.split(separator: ";") {
                let parts = assignment.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
                store.userDefaults.set(String(parts[1]), forKey: Self.settingsKey(String(parts[0])))
            }
        }
        return try TestFixtures.composingShortcuts(in: store).rows
            .flatMap(\.fields)
            .map { "\($0.settingsKey.dropFirst(Self.prefix.count))=\($0.chord?.rawValue ?? "-")" }
            .joined(separator: ";")
    }
}
