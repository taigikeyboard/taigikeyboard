// Pins the candidate list's navigation contract: clamped ends, page-first
// paging, and page-relative direct selection.

import XCTest

@testable import TaigiInputMethodCore

final class CandidateListModelTests: XCTestCase {
    /// Distinguishable by `roman`, which is what the assertions read.
    private func model(count: Int) -> CandidateListModel {
        var model = CandidateListModel()
        model.replace(with: (0 ..< count).map { TestFixtures.candidate(roman: "c\($0)") })
        return model
    }

    // MARK: - Highlight

    func testFreshList_highlightsTheTopRankedCandidate() {
        let model = model(count: 5)

        XCTAssertEqual(model.highlighted?.roman, "c0")
        XCTAssertEqual(model.highlightedSlotInPage, 0)
    }

    func testReplace_returnsTheHighlightToTheTop() {
        var model = model(count: 5)
        model.moveHighlight(.forward)

        model.replace(with: [TestFixtures.candidate(roman: "new")])

        XCTAssertEqual(
            model.highlighted?.roman,
            "new",
            "a keystroke re-ranks the list, so a preserved index would land on an unrelated word",
        )
    }

    func testMoveHighlight_stepsOneCandidateEachWay() {
        var model = model(count: 5)

        model.moveHighlight(.forward)
        model.moveHighlight(.forward)
        XCTAssertEqual(model.highlighted?.roman, "c2")

        model.moveHighlight(.backward)
        XCTAssertEqual(model.highlighted?.roman, "c1")
    }

    func testMoveHighlight_clampsAtBothEndsRatherThanWrapping() {
        var model = model(count: 3)

        model.moveHighlight(.backward)
        XCTAssertEqual(model.highlighted?.roman, "c0", "backward from the top must not wrap to the end")

        for _ in 0 ..< 5 { model.moveHighlight(.forward) }
        XCTAssertEqual(
            model.highlighted?.roman,
            "c2",
            "forward past the last candidate must not jump back to the rank the user just rejected",
        )
    }

    func testMoveHighlight_emptyList_doesNothing() {
        var model = CandidateListModel()

        model.moveHighlight(.forward)

        XCTAssertNil(model.highlighted)
        XCTAssertNil(model.highlightedSlotInPage)
    }

    // MARK: - Paging

    func testPage_movesToTheFirstCandidateOfTheNextPage() {
        var model = model(count: 20)
        model.moveHighlight(.forward)
        model.moveHighlight(.forward)

        model.page(.forward)

        XCTAssertEqual(
            model.highlighted?.roman,
            "c9",
            "landing on the first slot keeps the page start, the highlight and the ⌃1 label together",
        )
        XCTAssertEqual(model.highlightedSlotInPage, 0)
    }

    func testPage_backwardFromTheSecondPage_returnsToTheFirstCandidate() {
        var model = model(count: 20)
        model.page(.forward)

        model.page(.backward)

        XCTAssertEqual(model.highlighted?.roman, "c0")
    }

    func testPage_pastEitherEnd_doesNothing() {
        var model = model(count: 12)

        model.page(.backward)
        XCTAssertEqual(model.highlighted?.roman, "c0")

        model.page(.forward)
        model.page(.forward)
        XCTAssertEqual(
            model.highlighted?.roman,
            "c9",
            "there is no third page, so the highlight stays where the second one put it",
        )
    }

    func testVisiblePage_showsOnlyTheHighlightedPage() {
        var model = model(count: 20)

        XCTAssertEqual(model.visiblePage.map(\.roman), (0 ..< 9).map { "c\($0)" })

        model.page(.forward)
        XCTAssertEqual(model.visiblePage.map(\.roman), (9 ..< 18).map { "c\($0)" })

        model.page(.forward)
        XCTAssertEqual(
            model.visiblePage.map(\.roman),
            ["c18", "c19"],
            "a final page shorter than nine must not read past the end of the list",
        )
    }

    // MARK: - Direct selection

    func testSelectSlotInPage_addressesWhatIsOnScreen() {
        var model = model(count: 20)
        model.page(.forward)

        let selected = model.selectSlotInPage(2)

        XCTAssertEqual(
            selected?.roman,
            "c11",
            "⌃3 selects the third candidate the user can see, not the third of the whole list",
        )
        XCTAssertEqual(model.highlighted?.roman, "c11")
    }

    func testSelectSlotInPage_emptySlotOfTheLastPage_selectsNothing() {
        var model = model(count: 11)
        model.page(.forward)

        XCTAssertNil(
            model.selectSlotInPage(5),
            "the last page has two candidates, so ⌃6 addresses an empty cell",
        )
        XCTAssertEqual(model.highlighted?.roman, "c9", "a miss must leave the highlight alone")
    }

    func testSelectSlotInPage_outsideThePage_selectsNothing() {
        var model = model(count: 20)

        XCTAssertNil(model.selectSlotInPage(CandidateListModel.pageSize))
        XCTAssertNil(model.selectSlotInPage(-1))
    }

    // MARK: - Reset

    func testReset_emptiesTheList() {
        var model = model(count: 5)

        model.reset()

        XCTAssertTrue(model.isEmpty)
        XCTAssertNil(model.highlighted)
        XCTAssertTrue(model.visiblePage.isEmpty)
    }
}
