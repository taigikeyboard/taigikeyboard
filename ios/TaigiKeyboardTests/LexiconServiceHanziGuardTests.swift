import XCTest
@testable import TaigiKeyboard

/// `INVARIANT_LEX_HANZI_GUARD` — D-8 parity correction toward Android
/// (v3.5.6). Pinned at the iOS lexicon-service layer per Codex Mod 1 of the
/// 2026-05-01 auto-mode joint sign-off. See
/// `docs/architecture/behavioral-invariants.md` §14.
///
/// **Behavior**: when `LexiconService.search(inputType: .hanzi, ...)` is
/// invoked, it must return `[]` BEFORE consulting custom-dictionary,
/// system-dictionary, or association binaries. The guard sits at the top
/// of `search()` and short-circuits before any reader is touched.
///
/// **Pure XCTest** — no Rust runtime needed. The hanzi guard fires before
/// any bridge call (the bridge would fail without `lexiconInstall`, but
/// the guard never reaches that path). This makes the test self-contained
/// and bridge-mock-free.
final class LexiconServiceHanziGuardTests: XCTestCase {
    func test_hanziInputType_returnsEmpty_evenWithLiveCompositionRoot() async throws {
        let service = LexiconService()

        let result = try await service.search(
            for: "我",
            inputType: .hanzi,
            inputMode: .tl,
            limit: 50,
        )

        XCTAssertTrue(
            result.isEmpty,
            "INVARIANT_LEX_HANZI_GUARD: hanzi inputType must return [], "
                + "got \(result.count) rows",
        )
    }

    func test_emptyInput_returnsEmpty_regardlessOfInputType() async throws {
        // Sanity: the empty-input guard short-circuits before the hanzi guard,
        // and both return []. Neither path requires bridge state.
        let service = LexiconService()

        let resultRoman = try await service.search(
            for: "",
            inputType: .romanWithTone,
            inputMode: .tl,
            limit: 50,
        )
        let resultHanzi = try await service.search(
            for: "",
            inputType: .hanzi,
            inputMode: .tl,
            limit: 50,
        )

        XCTAssertTrue(resultRoman.isEmpty, "empty roman input → []")
        XCTAssertTrue(resultHanzi.isEmpty, "empty hanzi input → []")
    }
}
