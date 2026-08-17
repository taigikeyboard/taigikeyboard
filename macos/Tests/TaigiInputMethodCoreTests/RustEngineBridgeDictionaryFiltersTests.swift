// What the engine resolves the user's dictionary toggles into.

@testable import TaigiInputMethodCore
import XCTest

/// Real FFI round-trips: the bit layout belongs to Rust, so the only way to
/// pin the mapping is to ask the engine and assert on what comes back.
final class RustEngineBridgeDictionaryFiltersTests: XCTestCase {
    private func mask(_ toggles: DictionarySourceToggles) throws -> UInt32 {
        try XCTUnwrap(
            RustEngineBridge.lexiconDictionaryFilters(toggles: toggles),
            "the engine answered nothing for a well-formed toggle set",
        )
    }

    /// The defaults with one flag varied, so a case can name the toggle it is
    /// about instead of listing all 24.
    private func toggles(
        _ transform: (inout DictionarySourceToggles) -> Void,
    ) -> DictionarySourceToggles {
        var toggles = DictionarySourceToggles.defaults
        transform(&toggles)
        return toggles
    }

    /// The defaults must never resolve to `0`: the engine reads `0` as
    /// "platform did not wire this" and turns every source back on
    /// (`composing.proto:176-183`), so a `0` here would silently restore the
    /// pre-PR11 behaviour while looking wired.
    func testDefaultToggles_resolveToANonZeroMask() throws {
        XCTAssertNotEqual(try mask(.defaults), 0)
    }

    /// Turning a source off has to change the mask — otherwise the toggle is
    /// decoration.
    func testTurningEachSourceOff_changesTheMask() throws {
        let baseline = try mask(.defaults)

        XCTAssertNotEqual(try mask(toggles { $0.kautian = false }), baseline, "教育部辭典")
        XCTAssertNotEqual(try mask(toggles { $0.taigitv = false }), baseline, "台語新詞辭庫")
        XCTAssertNotEqual(try mask(toggles { $0.kungge = false }), baseline, "台語工藝詞庫")
        XCTAssertNotEqual(try mask(toggles { $0.stti = false }), baseline, "學科術語辭典")
        XCTAssertNotEqual(try mask(toggles { $0.khpoo = false }), baseline, "腔口補充資料")
        XCTAssertNotEqual(try mask(toggles { $0.lkk = false }), baseline, "LKK")
        XCTAssertNotEqual(try mask(toggles { $0.dev = false }), baseline, "詞庫增補檔案")
    }

    /// And turning an off-by-default source on has to change it the other way.
    func testTurningEachOptInSourceOn_changesTheMask() throws {
        let baseline = try mask(.defaults)

        XCTAssertNotEqual(try mask(toggles { $0.itaigi = true }), baseline, "iTaigi")
        XCTAssertNotEqual(try mask(toggles { $0.sitbut = true }), baseline, "台灣植物名彙")
        XCTAssertNotEqual(try mask(toggles { $0.taihoa = true }), baseline, "台華線頂對照典")
        XCTAssertNotEqual(try mask(toggles { $0.taijit = true }), baseline, "台日大辭典")
        XCTAssertNotEqual(try mask(toggles { $0.variant = true }), baseline, "異用字")
        XCTAssertNotEqual(try mask(toggles { $0.khiin = true }), baseline, "在來字")
    }

    /// Each toggle has to land on ITS OWN bit. "Different from the baseline"
    /// alone would still pass with two same-default sources wired to each
    /// other's field, so the sources are compared against each other as well.
    func testEachSource_ownsADistinctBit() throws {
        let masks = try [
            mask(toggles { $0.itaigi = true }),
            mask(toggles { $0.sitbut = true }),
            mask(toggles { $0.taihoa = true }),
            mask(toggles { $0.taijit = true }),
            mask(toggles { $0.variant = true }),
            mask(toggles { $0.khiin = true }),
        ]

        XCTAssertEqual(Set(masks).count, masks.count, "two sources are wired to one bit")
    }

    func testEachSubcollection_ownsADistinctBit() throws {
        let masks = try [
            mask(toggles { $0.kautianSubcollections.accentLukang = false }),
            mask(toggles { $0.kautianSubcollections.accentSansia = false }),
            mask(toggles { $0.kautianSubcollections.accentTaipak = false }),
            mask(toggles { $0.kautianSubcollections.accentGilan = false }),
            mask(toggles { $0.kautianSubcollections.accentTainan = false }),
            mask(toggles { $0.kautianSubcollections.accentKaohsiung = false }),
            mask(toggles { $0.kautianSubcollections.accentKinmen = false }),
            mask(toggles { $0.kautianSubcollections.accentMakung = false }),
            mask(toggles { $0.kautianSubcollections.accentSintik = false }),
            mask(toggles { $0.kautianSubcollections.accentTaichung = false }),
            mask(toggles { $0.kautianSubcollections.nameAppendix = false }),
        ]

        XCTAssertEqual(Set(masks).count, masks.count, "two 腔口 are wired to one bit")
    }

    // MARK: - The all-off state

    /// Switching every dictionary off is a state the settings window allows,
    /// and the engine's own answer for it is `0` — which the wire reserves for
    /// "platform did not wire this" and turns back into ALL sources. Sending
    /// that verbatim would hand the user every dictionary the moment they
    /// turned the last one off.
    func testEveryDictionaryOff_resolvesToZeroFromTheEngine() throws {
        XCTAssertEqual(try mask(allSourcesOff), 0)
    }

    func testEveryDictionaryOff_goesOnTheWireAsAMaskWithNoSources() {
        let sent = RustEngineBridge.enabledSourcesBitmask(for: allSourcesOff)

        XCTAssertNotEqual(sent, 0, "a zero would be read as 'not wired' and re-enable everything")
        XCTAssertEqual(sent, RustEngineBridge.noSourcesEnabledBitmask)
        XCTAssertEqual(sent & 0x1FFF, 0, "the source region has to stay empty")
    }

    func testAResolvedMask_goesOnTheWireUnchanged() throws {
        XCTAssertEqual(
            RustEngineBridge.enabledSourcesBitmask(for: .defaults),
            try mask(.defaults),
        )
    }

    private var allSourcesOff: DictionarySourceToggles {
        DictionarySourceToggles(
            kautian: false,
            taigitv: false,
            itaigi: false,
            sitbut: false,
            taihoa: false,
            taijit: false,
            kungge: false,
            stti: false,
            khpoo: false,
            variant: false,
            khiin: false,
            lkk: false,
            dev: false,
            kautianSubcollections: .defaults,
        )
    }

    /// The subcollection region is only meaningful while the master toggle is
    /// on, which is the engine's rule, not ours — asserting it here is what
    /// proves the nested message reached the engine at all.
    func testSubcollectionsMatter_onlyWhileKautianIsOn() throws {
        let allOn = try mask(.defaults)
        let oneAccentOff = try mask(toggles { $0.kautianSubcollections.accentGilan = false })
        XCTAssertNotEqual(oneAccentOff, allOn, "a 腔口 toggle has to reach the engine")

        let kautianOff = try mask(toggles { $0.kautian = false })
        let kautianOffAccentOff = try mask(toggles {
            $0.kautian = false
            $0.kautianSubcollections.accentGilan = false
        })
        XCTAssertEqual(
            kautianOffAccentOff,
            kautianOff,
            "with 教育部辭典 off its subcollections cannot change what is searched",
        )
    }

    func testNameAppendix_isItsOwnToggle() throws {
        XCTAssertNotEqual(
            try mask(toggles { $0.kautianSubcollections.nameAppendix = false }),
            try mask(.defaults),
        )
    }
}
