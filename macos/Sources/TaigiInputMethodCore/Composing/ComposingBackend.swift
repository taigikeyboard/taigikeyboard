// The key path: desktop-core over the shell seam.

import Foundation

/// Runs one session's key path in desktop-core over the shell seam
/// (`desktop_request_bytes`; docs/architecture/macos-desktop-core-roadmap.md
/// D3): classifies a key, drives the engine, and answers with what the
/// controller has to do to its client and its candidate window — in order,
/// as a list it replays after the call returns.
///
/// The controller keeps everything that is IMKit or AppKit: the client calls
/// themselves, the windows, the caret rectangle, the auto-space arm and the
/// swap's client check, the symbol picker, the Telex guide, the global
/// actions. The core never touches a client; it records, and the controller
/// replays (D3, record-then-replay). One per process (`shared`): the engine
/// it drives is process-wide.
///
/// Every request but `activate` is for the session that owns the engine; one
/// from a session that does not is answered `nil` and does nothing — the
/// non-owner rule of `ComposingSessionCoordinator.owns`.
///
/// What the core cannot know is asked here before a request is encoded: the
/// window's selection and slots, and the auto-space swap's client check.
/// The settings travel inside each request, read from the request's own
/// store, so the core classifies under exactly what the controller reads.
@MainActor
final class ComposingBackend {
    /// One encoded request in, one encoded response out.
    typealias Transport = ([UInt8]) -> [UInt8]

    /// The process's key path.
    static let shared = ComposingBackend(coordinator: .shared)

    private static let logger = DebugLogger(category: "ComposingBackend")

    /// Who owns the engine, and the shortcut-target registry the controller
    /// registers with right after `activate` — which accepts only this
    /// owner (roadmap D3, "What stays in the controller"). The core keeps
    /// the same record from the same `Activate` and `Release`.
    private let coordinator: ComposingSessionCoordinator

    /// The process's runtime, asked per request: its settings whitelist is
    /// what a snapshot is built from.
    private let runtime: () -> DesktopCoreRuntime?

    /// `DesktopCoreBridge.send`; a test substitutes it to answer what the
    /// core cannot be made to.
    private let transport: Transport

    /// The owner's composition as the last reply described it.
    private var isOwnerComposing = false

    init(
        coordinator: ComposingSessionCoordinator,
        runtime: @escaping () -> DesktopCoreRuntime? = { DesktopCoreRuntime.configured },
        transport: @escaping Transport = DesktopCoreBridge.send,
    ) {
        self.coordinator = coordinator
        self.runtime = runtime
        self.transport = transport
    }

    /// Whether `session` drives the engine now.
    func owns(_ session: ComposingSessionToken) -> Bool {
        coordinator.owns(session)
    }

    /// Whether `session` owns the engine and a composition is running.
    func isComposing(_ session: ComposingSessionToken) -> Bool {
        owns(session) && isOwnerComposing
    }

    /// `session` takes the engine (`takeInputOwnership`); a session that already
    /// holds it keeps its composition. Any list held for a session is dropped:
    /// the window went down with the handover.
    ///
    /// Claims `ComposingSessionCoordinator` too: the shortcut-target registry
    /// the controller registers with next accepts only its owner.
    ///
    /// An `Activate` that takes the engine from another session drops that
    /// session's composition and sends no `ClearMarkedText`: the region is
    /// the outgoing controller's, cleared on its own way out (`endSession`).
    ///
    /// Read under `settings`, the controller's store, like every other
    /// request: a handover starts a fresh engine session under the settings
    /// that store holds.
    func activate(_ session: ComposingSessionToken, settings: SettingsStore) {
        coordinator.claim(session)
        var message = Taigi_DesktopShell_ActivateRequest()
        message.token = session.value
        // A session that already held the engine keeps its composition.
        if case let .reply(_, _, isComposing) = exchange(
            .activate(message), settings: snapshot(in: settings.userDefaults), op: "activate",
        ) {
            isOwnerComposing = isComposing
        } else {
            isOwnerComposing = false
        }
    }

    /// `session` gives the engine up, if it holds it. A session that does not
    /// own the engine has nothing to give up — in the core either, which
    /// keeps the same record.
    /// Read under `settings`, the controller's store, as `activate` is.
    func release(_ session: ComposingSessionToken, settings: SettingsStore) {
        guard coordinator.release(session) else { return }
        isOwnerComposing = false
        var message = Taigi_DesktopShell_ReleaseRequest()
        message.token = session.value
        _ = exchange(.release(message), settings: snapshot(in: settings.userDefaults), op: "release")
    }

    /// One key, classified by the core under the request's settings snapshot.
    func key(_ key: KeyEventSnapshot, in request: ComposingRequest) -> ComposingKeyReply? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_KeyRequest()
        message.token = request.session.value
        message.event = Taigi_DesktopShell_KeyEvent(key)
        // A swap is a pass-through key's, and only an idle session passes an
        // attaching character through: composing, it commits with it.
        let isIdle = !isComposing(request.session)
        message.panel = Self.panel(
            request,
            swapping: isIdle ? Self.swapCandidate(for: key, settings: request.settings) : nil,
        )
        return session(.key(message), in: request, op: "key")
    }

    /// The lifecycle commit (`commitComposition`, deactivation, close): the
    /// composition as typed, with no auto space — the user did not finish a
    /// word there — and no list effect: the controller took the window down
    /// first.
    func commitComposition(in request: ComposingRequest) -> [ComposingEffect]? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_CommitCompositionRequest()
        message.token = request.session.value
        message.panel = Self.panel(request, swapping: nil)
        return session(.commitComposition(message), in: request, op: "commitComposition")?.effects
    }

    /// The commit the symbol-picker chord runs before the picker opens: the
    /// highlighted cell while the window shows one, the composition as typed
    /// (with its auto space) otherwise.
    func commitForSymbolPicker(in request: ComposingRequest) -> [ComposingEffect]? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_CommitForSymbolPickerRequest()
        message.token = request.session.value
        message.panel = Self.panel(request, swapping: nil)
        return session(.commitForSymbolPicker(message), in: request, op: "commitForSymbolPicker")?.effects
    }

    /// A symbol the picker wrote: at the caret as one string, or swapped with
    /// the auto space a commit left.
    func insertSymbol(_ symbol: String, in request: ComposingRequest) -> [ComposingEffect]? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_InsertSymbolRequest()
        message.token = request.session.value
        message.symbol = symbol
        message.panel = Self.panel(request, swapping: symbol)
        return session(.insertSymbol(message), in: request, op: "insertSymbol")?.effects
    }

    /// A click on a cap of the TPS key panel: `glyph` typed through the
    /// engine's `TpsKey`, whatever the window would make of the physical
    /// key. Not handled — no effects — outside TPS or for a glyph the layout
    /// does not type.
    func tpsKeyboardPress(_ glyph: String, in request: ComposingRequest) -> [ComposingEffect]? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_TpsKeyboardPressRequest()
        message.token = request.session.value
        message.glyph = glyph
        message.panel = Self.panel(request, swapping: nil)
        return session(.tpsKeyboardPress(message), in: request, op: "tpsKeyboardPress")?.effects
    }

    /// The list on screen again under the settings in force now, after a
    /// switch outside the key path changed them: fetched again (`refetch`,
    /// Candidate Display) or the same list re-rendered (the Hanji/romanization
    /// swap). No list on screen, nothing to do.
    ///
    /// After `FAIL_INTERNAL` the list goes and nothing else: a represent
    /// has no client to clear marked text from, and a refetch never changes
    /// the composition, so the core's is left alone.
    func represent(refetch: Bool, in request: ComposingRequest) -> CandidateListRepresentation? {
        guard owns(request.session) else { return nil }
        var message = Taigi_DesktopShell_RepresentRequest()
        message.token = request.session.value
        message.refetch = refetch
        message.panel = Self.panel(request, swapping: nil)
        switch exchange(.represent(message), settings: snapshot(in: request.settings.userDefaults), op: "represent") {
        case let .reply(_, effects, isComposing):
            isOwnerComposing = isComposing
            switch effects.first {
            case nil:
                return .unchanged
            case let .candidatesChanged(list):
                return .changed(list)
            default:
                return .closed
            }
        case .failedInternally:
            return .closed
        case .nothing:
            return nil
        }
    }

    // MARK: - The seam

    /// What became of one request.
    private enum Outcome {
        /// The owner's reply, every effect translated.
        case reply(handled: Bool, effects: [ComposingEffect], isComposing: Bool)
        /// Nothing ran, or the session does not own the engine: nothing to
        /// replay, and the key goes to the host.
        case nothing
        /// The engine may have moved and the reply cannot say how (D4).
        case failedInternally
    }

    /// A session request under the settings `request` carries. After
    /// `FAIL_INTERNAL` the key is consumed — handed back, the host would type
    /// it twice — and the composition and the list are dropped on both
    /// sides (roadmap D4).
    private func session(
        _ message: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        in request: ComposingRequest,
        op: String,
    ) -> ComposingKeyReply? {
        switch exchange(message, settings: snapshot(in: request.settings.userDefaults), op: op) {
        case let .reply(handled, effects, isComposing):
            isOwnerComposing = isComposing
            return ComposingKeyReply(handled: handled, effects: effects)
        case .nothing:
            return nil
        case .failedInternally:
            return ComposingKeyReply(handled: true, effects: recoverFromInternalFailure(session: request.session))
        }
    }

    /// The whitelisted settings as `defaults` holds them now; `nil` —
    /// logged — before the runtime exists, which the core would refuse
    /// anyway.
    private func snapshot(in defaults: UserDefaults) -> Taigi_DesktopShell_SettingsSnapshot? {
        guard let runtime = runtime() else {
            Self.logger.error("no desktop-core runtime — the request goes without settings")
            return nil
        }
        return runtime.settingsSnapshot(in: defaults)
    }

    /// Classified by whether the engine may have run: a request refused
    /// before dispatch (`FAIL_PARSE`, `FAIL_INVARIANT`, an encode failure)
    /// changed nothing; a response that does not decode, an OK without a
    /// session reply, or an effect this side cannot replay may follow work
    /// the reply does not describe. Logged with the op and the error, never
    /// the request — it holds what the user typed.
    private func exchange(
        _ message: Taigi_DesktopShell_DesktopRequest.OneOf_Request,
        settings: Taigi_DesktopShell_SettingsSnapshot?,
        op: String,
    ) -> Outcome {
        let response: Taigi_DesktopShell_DesktopResponse
        switch DesktopCoreBridge.response(to: message, settings: settings, op: op, transport: transport) {
        case let .success(decoded):
            response = decoded
        case .failure(.notSent):
            return .nothing
        case .failure(.undecodable):
            return .failedInternally
        }
        switch response.error {
        case .ok:
            guard case let .session(reply) = response.reply else {
                Self.logger.error("[\(op)] OK without a session reply")
                return .failedInternally
            }
            if reply.ignored {
                Self.logger.notice("[\(op)] core ignored session request")
                return .nothing
            }
            let effects = reply.effects.compactMap(Self.effect)
            guard effects.count == reply.effects.count else {
                Self.logger.error("[\(op)] an effect this side cannot replay")
                return .failedInternally
            }
            return .reply(handled: reply.handled, effects: effects, isComposing: reply.isComposing)
        case .failInternal:
            Self.logger.error("[\(op)] core failed internally")
            return .failedInternally
        default:
            Self.logger.error("[\(op)] core refused: \(response.error)")
            return .nothing
        }
    }

    /// Cancels the session in the core — once, whatever it answers — and
    /// answers what the controller does to its own side: the marked text the
    /// last reply left, and the list.
    private func recoverFromInternalFailure(session: ComposingSessionToken) -> [ComposingEffect] {
        let wasComposing = isOwnerComposing
        isOwnerComposing = false
        var cancel = Taigi_DesktopShell_CancelRequest()
        cancel.token = session.value
        cancel.panel = Taigi_DesktopShell_PanelState()
        _ = exchange(.cancel(cancel), settings: nil, op: "cancel")
        return (wasComposing ? [.clearMarkedText] : []) + [.candidatesClosed]
    }

    // MARK: - Requests

    /// The window's state, every question asked now, before the request
    /// crosses. The highlight is asked whether or not a list is up — the core's
    /// picker commit reads it; the slots only while one is. The swap's
    /// client check — two client queries — is asked only for `attaching`,
    /// the text a swap would write, and only when a swap can happen: an arm,
    /// an attaching character, Auto-Space on.
    private static func panel(
        _ request: ComposingRequest,
        swapping attaching: @autoclosure () -> String?,
    ) -> Taigi_DesktopShell_PanelState {
        let state = request.panel
        var panel = Taigi_DesktopShell_PanelState()
        panel.isListOnScreen = state.isListOnScreen
        if let selected = state.selectedIndex() {
            panel.selectedIndex = UInt32(selected)
        }
        if state.isListOnScreen {
            for slot in 0 ..< HorizontalPageLayout.pageSize {
                if let index = state.indexForKeySlot(slot) {
                    panel.slotIndices[UInt32(slot)] = UInt32(index)
                }
            }
        }
        if let canSwapPrecedingSpace = state.canSwapPrecedingSpace,
           request.settings.isAutoSpaceEnabled,
           let attaching = attaching(),
           RustEngineBridge.isAttachingPunctuation(attaching)
        {
            panel.swapAvailable = canSwapPrecedingSpace()
        }
        return panel
    }

    /// What a pass-through `key` would attach to the armed space: the
    /// character it types — or, under TPS, which writes only full width, the
    /// punctuation it maps to, which is what the swap writes then.
    private static func swapCandidate(for key: KeyEventSnapshot, settings: SettingsStore) -> String? {
        let inputMode = settings.inputMode
        guard let typed = key.documentText(inputMode: inputMode) else { return nil }
        guard inputMode == .tps else { return typed }
        return FullWidthPunctuation.mapped(typed) ?? typed
    }

    // MARK: - Effects

    private static func effect(_ effect: Taigi_DesktopShell_Effect) -> ComposingEffect? {
        switch effect.effect {
        case let .setMarkedText(marked):
            .setMarkedText(marked.text, caretUTF16: Int(marked.caretUtf16))
        case .clearMarkedText_p:
            .clearMarkedText
        case let .insertText(insert):
            .insertText(insert.text)
        case let .swapPrecedingSpace(swap):
            .swapPrecedingSpace(replacement: swap.replacement)
        case .armSwap:
            .armSwap
        case let .candidatesChanged(list):
            .candidatesChanged(CandidateListUpdate(
                cells: list.cells.map {
                    CandidateCellContent(text: $0.text, annotation: $0.annotation)
                },
                leadsWithLiteralRoman: list.leadsWithLiteralRoman,
                anchorEndUTF16: Int(list.anchorEndUtf16),
            ))
        case .candidatesClosed:
            .candidatesClosed
        case let .navigate(navigate):
            CandidateNavigation(navigate.direction).map(ComposingEffect.navigate)
        case let .tpsKeyboardKeyTyped(typed):
            .tpsKeyboardKeyTyped(row: Int(typed.row), cap: Int(typed.cap))
        case nil:
            nil
        }
    }
}
