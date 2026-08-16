// Pins the Chromium activation-deadlock rule.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Chromium hosts deadlock when an input method makes a synchronous
/// client round-trip inside `activateServer` (Chromium issue 503787240 —
/// azooKey-Desktop hit this in Chrome on JS-heavy pages). This drives the real
/// `activateServer` with a client that records every read, so the rule breaks
/// loudly the moment someone adds one.
final class ActivateServerClientQueryTests: XCTestCase {
    func testActivateServer_doesNotReadFromClient() throws {
        let client = RecordingTextInputClient()
        let controller = try TestFixtures.makeInputController()

        controller.activateServer(client)

        XCTAssertEqual(
            client.readCallCount,
            0,
            """
            activateServer read from the client (\(client.readCalls.joined(separator: ", "))). \
            A synchronous client round-trip during activation deadlocks Chromium hosts \
            (Chromium issue 503787240) — move the read to the first key event instead.
            """,
        )
    }
}
