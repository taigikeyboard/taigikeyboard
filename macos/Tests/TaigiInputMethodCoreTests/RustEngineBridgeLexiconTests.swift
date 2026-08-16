// Proves the engine actually loads the dictionary the bundle ships.

@testable import TaigiInputMethodCore
import XCTest

final class RustEngineBridgeLexiconTests: XCTestCase {
    func testLexiconInstall_repositoryDictionaries_loadsRecords() throws {
        let stats = try XCTUnwrap(
            InstalledLexicon.installOnce(),
            "install returned no stats, so the engine has no lexicon",
        )

        // Exact counts change with every dictionary rebuild; what must hold is
        // that the engine read something. A zero here is the failure mode this
        // test exists for — an install that reports success having mapped an
        // empty or wrong file looks identical from the outside until the user
        // types and sees no candidates.
        XCTAssertGreaterThan(stats.dictionaryRecordCount, 0)
        XCTAssertGreaterThan(stats.prefixIndexEntryCount, 0)
    }
}
