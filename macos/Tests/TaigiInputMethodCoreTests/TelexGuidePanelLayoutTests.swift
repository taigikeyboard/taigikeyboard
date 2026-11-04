// The Telex guide's card: its text keeps clear of the outline on every edge.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class TelexGuidePanelLayoutTests: XCTestCase {
    func testEveryLabel_keepsClearOfTheOutline() throws {
        let suiteName = "TelexGuidePanelLayoutTests.\(UUID().uuidString)"
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { userDefaults.removePersistentDomain(forName: suiteName) }
        let language = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)

        let panel = TelexGuidePanel.makePanel(inputMode: .tl, language: language)
        let card = try XCTUnwrap(panel.contentView)
        card.layoutSubtreeIfNeeded()

        let labels = textFields(in: card)
        XCTAssertFalse(labels.isEmpty)
        // A label's frame overhangs its text by a couple of points, so this
        // is under the margin the card asks for.
        let margin: CGFloat = 8
        for label in labels {
            let frame = label.convert(label.bounds, to: card)
            XCTAssertGreaterThanOrEqual(frame.minX, margin, label.stringValue)
            XCTAssertGreaterThanOrEqual(frame.minY, margin, label.stringValue)
            XCTAssertLessThanOrEqual(frame.maxX, card.bounds.maxX - margin, label.stringValue)
            XCTAssertLessThanOrEqual(frame.maxY, card.bounds.maxY - margin, label.stringValue)
        }
    }

    private func textFields(in view: NSView) -> [NSTextField] {
        view.subviews.flatMap { subview -> [NSTextField] in
            if let field = subview as? NSTextField {
                return [field]
            }
            return textFields(in: subview)
        }
    }
}
