// The `.taigi` document: what it writes, what it accepts, and what it says
// when one category fails.

@testable import TaigiInputMethodCore
import XCTest

final class BackupServiceTests: XCTestCase {
    private var stores: UserDataStores!
    private var service: BackupService!

    override func setUpWithError() throws {
        try super.setUpWithError()
        stores = try TestFixtures.makeUserDataStores()
        service = BackupService(stores: stores, appVersion: "1.2.3")
    }

    private func decode(_ data: Data) throws -> BackupDocument {
        try JSONDecoder().decode(BackupDocument.self, from: data)
    }

    // MARK: - Export

    func testExport_writesTheSchemaTheOtherPlatformsRead() async throws {
        let document = try await decode(service.export())

        XCTAssertEqual(document.version, 2)
        XCTAssertEqual(document.platform, "macos")
        XCTAssertEqual(document.appVersion, "1.2.3")
    }

    func testExport_carriesEveryCategory() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "gua", hanzi: "我"),
        )
        stores.frequency.record(word: "我", tl: "guá")
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó"),
        ])

        let document = try await decode(service.export())

        XCTAssertEqual(document.customDictionary.map(\.hanzi), ["我"])
        XCTAssertEqual(document.userFrequency.map(\.word), ["我"])
        XCTAssertEqual(document.userAssociation.map(\.nextWord), ["好"])
    }

    /// The reading is what makes a frequency row identify a word, so it has to
    /// survive the file.
    func testExport_keepsBothReadingsOfOneWordApart() async throws {
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "重", tl: "tîng")

        let document = try await decode(service.export())

        XCTAssertEqual(Set(document.userFrequency.compactMap(\.tl)), ["tāng", "tîng"])
    }

    /// Two exports taken a moment apart carry the same rows. Not the same
    /// bytes — `exportedAt` moves — which is why this compares the decoded
    /// contents rather than claiming more than the format gives.
    func testExport_carriesTheSameRowsAcrossTwoExports() async throws {
        stores.frequency.record(word: "我", tl: "guá")
        let first = try await service.export()

        let second = try await service.export()

        // Only the timestamp may differ, and it is second-resolution — compare
        // everything else by decoding rather than by bytes.
        XCTAssertEqual(try decode(first).userFrequency, try decode(second).userFrequency)
    }

    // MARK: - Restore

    func testRestore_roundTripsThroughAFreshSetOfStores() async throws {
        try await stores.customDictionary.upsert(CustomDictionaryRow(roman: "gua", hanzi: "我"))
        stores.frequency.record(word: "我", tl: "guá")
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó"),
        ])
        let data = try await service.export()

        let restoredStores = try TestFixtures.makeUserDataStores()
        let result = try await BackupService(stores: restoredStores).restore(from: data)

        XCTAssertFalse(result.hasFailure)
        let customRows = try await restoredStores.customDictionary.allRows()
        let frequencyRows = try await restoredStores.frequency.allRows()
        let associationRows = try await restoredStores.association.rows()
        XCTAssertEqual(customRows.map(\.hanzi), ["我"])
        XCTAssertEqual(frequencyRows.map(\.tl), ["guá"])
        XCTAssertEqual(associationRows.map(\.pair.next), ["好"])
    }

    /// A version-1 file has no per-reading column. Its rows restore into the
    /// tolerant empty-reading bucket rather than being dropped or guessed at.
    func testRestore_acceptsAVersionOneFile() async throws {
        let data = try JSONEncoder().encode(BackupDocument(
            version: 1,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "ios",
            appVersion: "3.0.0",
            customDictionary: [],
            userFrequency: [.init(word: "我", tl: nil, count: 4, lastUsed: "")],
            userAssociation: [],
        ))

        let result = try await service.restore(from: data)

        XCTAssertEqual(result.frequency, .restored(1))
        let rows = try await stores.frequency.allRows()
        XCTAssertEqual(rows.map(\.tl), [""])
    }

    /// A file from a version this build has not seen is refused rather than
    /// half-read: a format that changed the meaning of a field looks exactly
    /// like one that added a field, from here.
    func testRestore_refusesANewerFormat() async throws {
        let data = try JSONEncoder().encode(BackupDocument(
            version: BackupService.currentVersion + 1,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "ios",
            appVersion: "9.0.0",
            customDictionary: [.init(roman: "gua", hanzi: "我")],
            userFrequency: [],
            userAssociation: [],
        ))

        do {
            _ = try await service.restore(from: data)
            XCTFail("a newer format has to be refused")
        } catch let error as BackupError {
            guard case .unsupportedVersion = error else { return XCTFail("wrong error: \(error)") }
        }

        let rows = try await stores.customDictionary.allRows()
        XCTAssertTrue(rows.isEmpty, "a refused file must not have written anything")
    }

    // MARK: - Cross-platform wire format

    /// A file exactly as iOS writes it — required `lastUsed` strings, an
    /// absent `prevTl`, a pre-R5 row with no `tl`. This is the whole point of
    /// the format, and a decoder that got any of these types wrong would
    /// refuse a backup the user made on their phone.
    func testRestore_readsAFileShapedLikeTheOnesIosWrites() async throws {
        let json = """
        {
          "version": 2,
          "exportedAt": "2026-08-17T00:00:00Z",
          "platform": "ios",
          "appVersion": "3.6.5",
          "customDictionary": [{"roman": "gua", "hanzi": "我"}],
          "userFrequency": [
            {"word": "我", "tl": "guá", "count": 7, "lastUsed": ""},
            {"word": "你", "count": 2, "lastUsed": ""}
          ],
          "userAssociation": [
            {"prevWord": "台語", "nextWord": "真好", "nextTl": "tsin-hó", "count": 3, "lastUsed": ""}
          ]
        }
        """

        let result = try await service.restore(from: Data(json.utf8))

        XCTAssertFalse(result.hasFailure, "a file iOS could have written was refused")
        XCTAssertEqual(result.customDictionary, .restored(1))
        XCTAssertEqual(result.frequency, .restored(2))
        XCTAssertEqual(result.association, .restored(1))
        let frequencyRows = try await stores.frequency.allRows()
        XCTAssertEqual(Set(frequencyRows.map(\.tl)), ["guá", ""], "a row with no reading was dropped")
        let associationRows = try await stores.association.rows()
        XCTAssertEqual(associationRows.first?.pair.previousTl, "", "an absent prevTl was not tolerated")
    }

    /// And the file this side writes has to satisfy that decoder: every field
    /// it requires, with the type it requires.
    func testExport_writesTheTypesTheOtherPlatformsRequire() async throws {
        stores.frequency.record(word: "我", tl: "guá")
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó"),
        ])

        let object = try await JSONSerialization.jsonObject(
            with: service.export(),
        ) as? [String: Any]

        let frequency = try XCTUnwrap((object?["userFrequency"] as? [[String: Any]])?.first)
        XCTAssertTrue(frequency["lastUsed"] is String, "lastUsed has to be the string iOS declares")
        let association = try XCTUnwrap((object?["userAssociation"] as? [[String: Any]])?.first)
        XCTAssertTrue(association["lastUsed"] is String, "the association row is missing lastUsed")
        XCTAssertNotNil(association["prevTl"])
    }

    // MARK: - Readings

    /// A version-2 file says its readings are canonical TL already, so they go
    /// in as they are: folding them again would rewrite the ones the two
    /// scripts spell differently.
    func testRestore_keepsAVersionTwoReadingVerbatim() async throws {
        let data = try JSONEncoder().encode(BackupDocument(
            version: 2,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "ios",
            appVersion: "3.0.0",
            customDictionary: [],
            userFrequency: [],
            userAssociation: [.init(
                prevWord: "登",
                prevTl: "teng",
                nextWord: "山",
                nextTl: "suann",
                count: 1,
                lastUsed: "",
            )],
        ))

        _ = try await service.restore(from: data)

        let rows = try await stores.association.rows()
        XCTAssertEqual(rows.first?.pair.previousTl, "teng")
    }

    /// A version-1 file predates the format saying which script it carried, so
    /// its readings are folded on the way in.
    func testRestore_foldsAVersionOneReadingToTl() async throws {
        let data = try JSONEncoder().encode(BackupDocument(
            version: 1,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "ios",
            appVersion: "2.0.0",
            customDictionary: [],
            userFrequency: [],
            userAssociation: [.init(
                prevWord: "食",
                prevTl: "chia̍h",
                nextWord: "飯",
                nextTl: "png",
                count: 1,
                lastUsed: "",
            )],
        ))

        _ = try await service.restore(from: data)

        let rows = try await stores.association.rows()
        XCTAssertEqual(
            rows.first?.pair.previousTl,
            "tsia̍h",
            "a POJ reading reached the store unfolded",
        )
    }

    func testRestore_refusesAVersionBelowOne() async throws {
        let data = try JSONEncoder().encode(BackupDocument(
            version: 0,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "unknown",
            appVersion: "0",
            customDictionary: [.init(roman: "gua", hanzi: "我")],
            userFrequency: [],
            userAssociation: [],
        ))

        do {
            _ = try await service.restore(from: data)
            XCTFail("version 0 is not a format this build knows")
        } catch let error as BackupError {
            guard case .unsupportedVersion = error else { return XCTFail("wrong error: \(error)") }
        }
    }

    /// Restoring merges rather than replaces: a count the user has built up
    /// past the backup's must not be walked backwards.
    func testRestore_mergesRatherThanReplaces() async throws {
        for _ in 0 ..< 5 {
            stores.frequency.record(word: "我", tl: "guá")
        }
        let data = try JSONEncoder().encode(BackupDocument(
            version: 2,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "android",
            appVersion: "3.0.0",
            customDictionary: [],
            userFrequency: [.init(word: "我", tl: "guá", count: 2, lastUsed: "")],
            userAssociation: [],
        ))

        _ = try await service.restore(from: data)

        let rows = try await stores.frequency.allRows()
        XCTAssertEqual(rows.first?.count, 5)
    }

    /// One category failing is reported as a failure, not as zero rows — and
    /// does not stop the others, because the file has already been read and
    /// the rest of it is still the user's data.
    func testRestore_reportsAFailingCategoryWithoutStoppingTheOthers() async throws {
        let directory = try TestFixtures.scratchDirectory()
        let brokenCustomDictionary = CustomDictionaryStore(directory: { directory })
        // Never opened, so every write to it throws.
        let mixedStores = UserDataStores(
            frequency: stores.frequency,
            association: stores.association,
            customDictionary: brokenCustomDictionary,
        )
        let data = try JSONEncoder().encode(BackupDocument(
            version: 2,
            exportedAt: "2026-01-01T00:00:00Z",
            platform: "ios",
            appVersion: "3.0.0",
            customDictionary: [.init(roman: "gua", hanzi: "我")],
            userFrequency: [.init(word: "我", tl: "guá", count: 3, lastUsed: "")],
            userAssociation: [],
        ))

        let result = try await BackupService(stores: mixedStores).restore(from: data)

        XCTAssertTrue(result.hasFailure)
        if case .restored = result.customDictionary {
            XCTFail("a failure was reported as a count")
        }
        XCTAssertEqual(result.frequency, .restored(1), "the categories that could restore did")
    }
}
