// What the two learning databases keep, against real SQLite.

@testable import TaigiInputMethodCore
import XCTest

/// Drives the stores through their public surface with a scratch directory per
/// case. Real files rather than doubles: everything these types do is SQL, and
/// the identity rule they exist to enforce lives in a UNIQUE constraint.
final class LearningStoreTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        stores = try TestFixtures.makeUserDataStores()
    }

    override func tearDown() {
        stores = nil
        super.tearDown()
    }

    // MARK: - Frequency

    func testRecord_countsRepeatedCommitsOfTheSameReading() throws {
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "重", tl: "tāng")

        let rows = try waitForFrequencyRows(of: ["重"], untilCountIs: 1)
        XCTAssertEqual(rows.first?.count, 2, "the second commit adds to the row the first one made, rather than making another")
    }

    /// Core Principle #7: a Taiwanese word is the `(漢字, canonical TL)` pair.
    /// 重/tîng (重複) and 重/tāng (重量) are different words, so committing one
    /// must not promote the other.
    func testRecord_keepsTheTwoReadingsOfOneHanjiApart() throws {
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "重", tl: "tîng")

        let rows = try waitForFrequencyRows(of: ["重"], untilCountIs: 2)
        XCTAssertEqual(
            Dictionary(uniqueKeysWithValues: rows.map { ($0.tl, $0.count) }),
            ["tāng": 2, "tîng": 1],
            "one row per reading, each with its own count",
        )
    }

    func testRecord_aCandidateWithNoReadingGetsItsOwnRowRatherThanJoiningOne() throws {
        stores.frequency.record(word: "重", tl: "")
        stores.frequency.record(word: "重", tl: "tāng")

        let rows = try waitForFrequencyRows(of: ["重"], untilCountIs: 2)
        XCTAssertEqual(
            Set(rows.map(\.tl)),
            ["", "tāng"],
            "an empty reading is its own bucket, not a merge into a real one",
        )
    }

    func testRows_asksForSeveralWordsAtOnceAndReportsWhatItFound() throws {
        stores.frequency.record(word: "台", tl: "tâi")
        stores.frequency.record(word: "語", tl: "gí")

        let rows = try waitForFrequencyRows(of: ["台", "語", "袂學過"], untilCountIs: 2)
        XCTAssertEqual(
            Set(rows.map(\.word)),
            ["台", "語"],
            "a word with no history is absent rather than reported with a zero count — "
                + "the engine reads an absent entry as neutral",
        )
    }

    /// The engine ranks on recency, and the field it reads is milliseconds. A
    /// `> 0` assertion would pass just as happily on seconds, which is a
    /// thousand-fold error that would put every learned word's last use in
    /// 1970 and flatten the decay curve.
    func testRows_reportsTheLastUseInMillisecondsSinceTheEpoch() throws {
        let nowMillis = Int64(Date().timeIntervalSince1970 * 1000)
        stores.frequency.record(word: "台", tl: "tâi")

        let rows = try waitForFrequencyRows(of: ["台"], untilCountIs: 1)
        let lastUsed = try XCTUnwrap(rows.first).lastUsedMillis
        // SQLite stores the timestamp to the second, so the reported value can
        // sit up to a second behind the clock read above.
        XCTAssertGreaterThan(lastUsed, nowMillis - 2000)
        XCTAssertLessThan(lastUsed, nowMillis + 2000)
    }

    func testRecord_ignoresAnEmptyWord() throws {
        stores.frequency.record(word: "", tl: "tâi")
        stores.frequency.record(word: "台", tl: "tâi")

        let rows = try waitForFrequencyRows(of: ["", "台"], untilCountIs: 1)
        XCTAssertEqual(rows.map(\.word), ["台"])
    }

    // MARK: - Association

    func testRecordPairs_storesEachBigramOnce() throws {
        stores.association.record([
            AssociationPair(previous: "台", previousTl: "tâi", next: "語", nextTl: "gí"),
            AssociationPair(previous: "語", previousTl: "gí", next: "課", nextTl: "khò"),
        ])

        let rows = try waitForAssociationRows(untilCountIs: 2)
        XCTAssertEqual(
            Set(rows.map { "\($0.pair.previous)→\($0.pair.next)" }),
            ["台→語", "語→課"],
        )
    }

    func testRecordPairs_repeatedBigramUpdatesTheRowItAlreadyHas() throws {
        let pair = AssociationPair(previous: "台", previousTl: "tâi", next: "語", nextTl: "gí")
        stores.association.record([pair])
        stores.association.record([pair])

        let rows = try waitForAssociationRows(untilCountIs: 1)
        XCTAssertEqual(rows.count, 1, "the UNIQUE key is what stops one bigram becoming two rows")
        XCTAssertEqual(
            rows.first?.count,
            2,
            "the repeat has to raise the count — a row that never counts up ranks like a "
                + "bigram seen once no matter how often it is typed",
        )
    }

    /// The next word is keyed by its reading as well as its 漢字 (Core Principle
    /// #7), so the same 漢字 read two ways is two observations.
    func testRecordPairs_theSameNextHanjiUnderTwoReadingsIsTwoRows() throws {
        stores.association.record([
            AssociationPair(previous: "看", previousTl: "khuànn", next: "重", nextTl: "tāng"),
            AssociationPair(previous: "看", previousTl: "khuànn", next: "重", nextTl: "tîng"),
        ])

        let rows = try waitForAssociationRows(untilCountIs: 2)
        XCTAssertEqual(Set(rows.map(\.pair.nextTl)), ["tāng", "tîng"])
    }

    /// The PREVIOUS word is half of a bigram's identity, and it is a
    /// `(漢字, canonical TL)` pair like any other word (Core Principle #7). 重
    /// read as 重複's tîng and 重 read as 重量's tāng are different predecessors,
    /// so the two observations must not merge into one row.
    func testRecordPairs_theSamePreviousHanjiUnderTwoReadingsIsTwoRows() throws {
        stores.association.record([
            AssociationPair(previous: "重", previousTl: "tîng", next: "複", nextTl: "hi̍k"),
            AssociationPair(previous: "重", previousTl: "tāng", next: "複", nextTl: "hi̍k"),
        ])

        let rows = try waitForAssociationRows(untilCountIs: 2)
        XCTAssertEqual(
            Set(rows.map(\.pair.previousTl)),
            ["tîng", "tāng"],
            "both readings survive as their own row — the shipped iOS and Android tables "
                + "key without prev_tl and would have merged these",
        )
        XCTAssertEqual(rows.map(\.count), [1, 1], "neither row absorbed the other's count")
    }

    func testRecordPairs_dropsAHalfEmptyPairRatherThanStoringIt() throws {
        stores.association.record([
            AssociationPair(previous: "", previousTl: "", next: "語", nextTl: "gí"),
            AssociationPair(previous: "台", previousTl: "tâi", next: "", nextTl: ""),
            AssociationPair(previous: "台", previousTl: "tâi", next: "語", nextTl: "gí"),
        ])

        let rows = try waitForAssociationRows(untilCountIs: 1)
        XCTAssertEqual(rows.map(\.pair.previous), ["台"])
    }

    // MARK: - Waiting

    /// Writes are queued and reads run on the same queue, so a read issued after
    /// a write sees it — but only once the write has been queued from THIS
    /// thread, which it has by the time `record` returns. The retry loop covers
    /// the store still opening.
    private func waitForFrequencyRows(
        of words: [String],
        untilCountIs expected: Int,
        timeout: TimeInterval = 5,
    ) throws -> [FrequencyRow] {
        try waitFor(untilCountIs: expected, timeout: timeout) {
            stores.frequency.rows(forWords: words)
        }
    }

    private func waitForAssociationRows(
        untilCountIs expected: Int,
        timeout: TimeInterval = 5,
    ) throws -> [AssociationRow] {
        try waitFor(untilCountIs: expected, timeout: timeout) {
            stores.association.allRows()
        }
    }

    private func waitFor<Row>(
        untilCountIs expected: Int,
        timeout: TimeInterval,
        _ read: () -> [Row]?,
    ) throws -> [Row] {
        let arrived = TestFixtures.spinRunLoop(
            until: { read()?.count == expected },
            timeout: timeout,
        )
        guard arrived else {
            XCTFail("the store never reported \(expected) rows within \(timeout)s")
            return []
        }
        return try XCTUnwrap(read())
    }
}
