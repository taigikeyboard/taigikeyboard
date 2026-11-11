// The core back end's side of the seam, against answers the core cannot be
// made to give: internal failures, refusals, replies it never sends.

@testable import TaigiInputMethodCore
import XCTest

/// Each case puts its own transport in front of a `ComposingBackend` and
/// its own coordinator, so nothing here reaches the engine. The core's own
/// answers are covered by the controller suites and by
/// `macos/crates/taigi-macos-ffi`.
@MainActor
final class ComposingBackendTests: XCTestCase {
    private var sent: [Taigi_DesktopShell_DesktopRequest] = []
    private var answer: (Taigi_DesktopShell_DesktopRequest) -> [UInt8] = { _ in [] }
    private var isSwapCheckAsked = false
    private let session = ComposingSessionToken()

    /// A handover starts a fresh engine session under the settings of the
    /// controller's store — read like every other request's, not from the
    /// app's `.standard` domain and not from the last request (freeze contract
    /// item 4; USER 2026-10-03, option 1).
    func testActivateAndRelease_carryTheSettingsOfTheControllersStore() throws {
        restoreStandardSettingsAtTeardown()
        let store = try makeScratchSettingsStore()
        let runtime = try XCTUnwrap(TestDesktopCore.runtime, "the process's one Configure")
        let autoSpace = SettingsStore.Keys.isAutoSpaceEnabled.name
        let backend = ComposingBackend(
            coordinator: TestFixtures.makeCoordinator(),
            runtime: { runtime },
            transport: { [unowned self] bytes in
                if let request = try? Taigi_DesktopShell_DesktopRequest(serializedBytes: Data(bytes)) {
                    sent.append(request)
                }
                return Self.ok()
            },
        )

        UserDefaults.standard.set(false, forKey: autoSpace)
        store.userDefaults.set(true, forKey: autoSpace)
        backend.activate(session, settings: store)
        UserDefaults.standard.set(true, forKey: autoSpace)
        store.userDefaults.set(false, forKey: autoSpace)
        backend.release(session, settings: store)

        let carried = sent.map { request in
            request.settings.entries.first { $0.name == autoSpace }?.value.boolean
        }
        XCTAssertEqual(carried, [true, false], "each read from the controller's store when it is sent")
    }

    // MARK: - FAIL_INTERNAL (roadmap D4)

    func testAnInternalFailure_consumesTheKey_dropsTheCompositionAndCancelsOnce() throws {
        let backend = try activatedBackend(composing: true)
        answer = { Self.isCancel($0) ? Self.ok() : Self.failed(.failInternal) }

        let reply = try XCTUnwrap(backend.key(Self.key("a"), in: request()))

        XCTAssertTrue(reply.handled, "the engine may have moved: the host must not type the key again")
        XCTAssertEqual(reply.effects, [.clearMarkedText, .candidatesClosed])
        XCTAssertEqual(sent.filter(Self.isCancel).count, 1)
        XCTAssertFalse(backend.isComposing(session))
    }

    func testAnInternalFailure_withNothingComposing_clearsNoMarkedText() throws {
        let backend = try activatedBackend(composing: false)
        answer = { Self.isCancel($0) ? Self.ok() : Self.failed(.failInternal) }

        let effects = backend.commitForSymbolPicker(in: request())

        XCTAssertEqual(effects, [.candidatesClosed], "an empty setMarkedText some clients mishandle")
    }

    /// A reply that does not decode, an OK with no session reply, an effect
    /// this side cannot replay: each may follow work the reply cannot
    /// describe, so each is recovered from as `FAIL_INTERNAL` is.
    func testAnswersThatMayHideWork_areRecoveredFromLikeAnInternalFailure() throws {
        var noVariant = Taigi_DesktopShell_SessionReply()
        noVariant.effects = [Taigi_DesktopShell_Effect()]
        let answers: [(String, [UInt8])] = [
            ("undecodable", [0xFF, 0xFF, 0xFF]),
            ("no session reply", Self.encode(Taigi_DesktopShell_DesktopResponse())),
            ("an effect with no variant", Self.ok(noVariant)),
        ]
        for (name, bytes) in answers {
            let backend = try activatedBackend(composing: true)
            answer = { Self.isCancel($0) ? Self.ok() : bytes }

            let reply = backend.key(Self.key("a"), in: request())

            XCTAssertEqual(reply?.handled, true, name)
            XCTAssertEqual(reply?.effects, [.clearMarkedText, .candidatesClosed], name)
            XCTAssertEqual(sent.filter(Self.isCancel).count, 1, name)
        }
    }

    /// A represent has no client to clear marked text from and never changes
    /// the composition, so the list goes and the core's session is left be.
    func testAnInternalFailureOnARepresent_closesTheListOnly() throws {
        let backend = try activatedBackend(composing: true)
        answer = { _ in Self.failed(.failInternal) }

        XCTAssertEqual(backend.represent(refetch: true, in: request()), .closed)
        XCTAssertFalse(sent.contains(where: Self.isCancel))
    }

    // MARK: - Nothing ran

    /// Refused before dispatch: nothing to recover, and a refusal that kept
    /// recurring would otherwise eat every keystroke.
    func testARefusal_handsTheKeyBackAndKeepsTheComposition() throws {
        let backend = try activatedBackend(composing: true)
        for error: Taigi_Engine_ErrorCode in [.failInvariant, .failParse] {
            answer = { _ in Self.failed(error) }

            XCTAssertNil(backend.key(Self.key("a"), in: request()), "\(error)")
            XCTAssertTrue(backend.isComposing(session), "\(error)")
        }
        XCTAssertFalse(sent.contains(where: Self.isCancel))
    }

    func testAnIgnoredReply_isTheNonOwnerAnswer() throws {
        let backend = try activatedBackend(composing: false)
        var ignored = Taigi_DesktopShell_SessionReply()
        ignored.ignored = true
        answer = { _ in Self.ok(ignored) }

        XCTAssertNil(backend.key(Self.key("a"), in: request()))
    }

    // MARK: - What travels in

    /// The client check behind a swap is two client queries, asked only
    /// where a swap can happen: an arm, an attaching character, Auto-Space
    /// on.
    func testTheSwapCheck_isAskedOnlyWhereASwapCanHappen() throws {
        let backend = try activatedBackend(composing: false)
        let store = try makeScratchSettingsStore()
        let cases: [(key: String, autoSpace: Bool, asked: Bool)] = [
            ("?", true, true),
            ("a", true, false),
            ("?", false, false),
        ]
        for (key, autoSpace, asked) in cases {
            store.isAutoSpaceEnabled = autoSpace
            isSwapCheckAsked = false

            _ = backend.key(Self.key(key), in: request(settings: store, armed: true))

            XCTAssertEqual(isSwapCheckAsked, asked, "\(key) with Auto-Space \(autoSpace)")
            XCTAssertEqual(try lastPanel().swapAvailable, asked, "\(key) with Auto-Space \(autoSpace)")
        }
    }

    /// Outside TPS ⌃, is the host's chord, so it attaches nothing and no
    /// check is asked; a composing session commits with the character instead
    /// of passing it through, so nothing is asked either.
    func testTheSwapCheck_skipsAHostChord_andAComposingSession() throws {
        let store = try makeScratchSettingsStore()
        store.isAutoSpaceEnabled = true
        let chord = KeyEventSnapshot(characters: ",", modifiers: .control, isNamedSpecialKey: false)

        _ = try activatedBackend(composing: false)
            .key(chord, in: request(settings: store, armed: true))
        XCTAssertFalse(isSwapCheckAsked, "⌃, is the host's under TL")

        isSwapCheckAsked = false
        _ = try activatedBackend(composing: true)
            .key(Self.key("?"), in: request(settings: store, armed: true))
        XCTAssertFalse(isSwapCheckAsked, "composing: the ? commits with the composition")
    }

    /// TPS writes only full width, so a bare key attaches the glyph it maps
    /// to: `}` (no TPS layout key) attaches to nothing, its `』` does. Under
    /// TL `}` is asked about as typed — the negative control.
    func testTheSwapCheck_underTps_followsTheFullWidthGlyph() throws {
        let store = try makeScratchSettingsStore()
        store.isAutoSpaceEnabled = true

        _ = try activatedBackend(composing: false)
            .key(Self.key("}"), in: request(settings: store, armed: true))
        XCTAssertFalse(isSwapCheckAsked, "TL: } attaches to nothing")

        store.switchInputMode(.pick(.tps))
        _ = try activatedBackend(composing: false)
            .key(Self.key("}"), in: request(settings: store, armed: true))
        XCTAssertTrue(isSwapCheckAsked, "TPS: } writes an attaching 』")
    }

    /// The highlight goes whether or not a list is up — the picker commit is
    /// chosen by it — the slots only while one is.
    func testThePanel_carriesTheHighlightAlways_andTheSlotsWithAList() throws {
        let backend = try activatedBackend(composing: true)

        _ = backend.commitForSymbolPicker(in: request(isListOnScreen: false))
        XCTAssertEqual(try lastPanel().selectedIndex, 2)
        XCTAssertTrue(try lastPanel().slotIndices.isEmpty)

        _ = backend.commitForSymbolPicker(in: request(isListOnScreen: true))
        XCTAssertEqual(try lastPanel().slotIndices[0], 10)
        XCTAssertEqual(try lastPanel().slotIndices.count, HorizontalPageLayout.pageSize)
    }

    func testTheKeyEvent_carriesTheNamedKeysRawValue() throws {
        let backend = try activatedBackend(composing: true)
        let up = try KeyEventSnapshot(TestFixtures.arrowKeyDownEvent(.upArrow))

        _ = backend.key(up, in: request())

        guard case let .key(key) = sent.last?.request else { return XCTFail("no key request") }
        XCTAssertEqual(key.event.specialKey, UInt32(NSUpArrowFunctionKey))
        XCTAssertEqual(key.token, session.value)
    }

    // MARK: - Fixture

    /// A back end whose transport records every request and answers with
    /// `answer`, activated for `session` with a composition running or not.
    /// The swap gate is the core's verdict, so that request goes to the real
    /// core, under the settings of the request's store.
    private func activatedBackend(composing: Bool) throws -> ComposingBackend {
        let runtime = try XCTUnwrap(TestDesktopCore.runtime, "the process's one Configure")
        let backend = ComposingBackend(
            coordinator: TestFixtures.makeCoordinator(),
            runtime: { runtime },
            transport: { [unowned self] bytes in
                guard let request = try? Taigi_DesktopShell_DesktopRequest(serializedBytes: Data(bytes)) else {
                    return []
                }
                sent.append(request)
                if case .passThroughSwap = request.request {
                    return DesktopCoreBridge.send(bytes)
                }
                return answer(request)
            },
        )
        var reply = Taigi_DesktopShell_SessionReply()
        reply.isComposing = composing
        answer = { _ in Self.ok(reply) }
        backend.activate(session, settings: SettingsStore())
        XCTAssertEqual(backend.isComposing(session), composing)
        sent = []
        return backend
    }

    private func request(
        settings: SettingsStore? = nil,
        isListOnScreen: Bool = false,
        armed: Bool = false,
    ) -> ComposingRequest {
        // The client check, standing in for the controller's: answers yes
        // and notes that it was asked. Spelled out rather than a ternary over
        // a method reference, which crashes the Swift 6.3 type checker.
        var swapCheck: (@MainActor () -> Bool)?
        if armed {
            swapCheck = { [unowned self] in
                isSwapCheckAsked = true
                return true
            }
        }
        return ComposingRequest(
            session: session,
            settings: settings ?? SettingsStore(),
            panel: ComposingPanelState(
                isListOnScreen: isListOnScreen,
                selectedIndex: { 2 },
                indexForKeySlot: { $0 + 10 },
                canSwapPrecedingSpace: swapCheck,
            ),
        )
    }

    private func lastPanel() throws -> Taigi_DesktopShell_PanelState {
        switch sent.last?.request {
        case let .key(key): key.panel
        case let .commitForSymbolPicker(commit): commit.panel
        default: throw NoPanel()
        }
    }

    private struct NoPanel: Error {}

    private static func key(_ characters: String) -> KeyEventSnapshot {
        KeyEventSnapshot(characters: characters, modifiers: [], isNamedSpecialKey: false)
    }

    private static func isCancel(_ request: Taigi_DesktopShell_DesktopRequest) -> Bool {
        if case .cancel = request.request {
            true
        } else {
            false
        }
    }

    private static func ok(_ reply: Taigi_DesktopShell_SessionReply = .init()) -> [UInt8] {
        var response = Taigi_DesktopShell_DesktopResponse()
        response.session = reply
        return encode(response)
    }

    private static func failed(_ error: Taigi_Engine_ErrorCode) -> [UInt8] {
        var response = Taigi_DesktopShell_DesktopResponse()
        response.error = error
        return encode(response)
    }

    private static func encode(_ response: Taigi_DesktopShell_DesktopResponse) -> [UInt8] {
        (try? Array(response.serializedData())) ?? []
    }
}
