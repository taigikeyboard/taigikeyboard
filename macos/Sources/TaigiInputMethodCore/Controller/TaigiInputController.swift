// Per-session IMKInputController: routes key events into the composing engine.

import InputMethodKit
import KeyboardShortcuts

/// One instance per client text session. Owns no composition of its own — it
/// claims the process-wide engine while its session is focused, hands key
/// events to the `ComposingBackend`, and replays what it answers into its own
/// client and the candidate window.
///
/// `@objc(TaigiInputController)` pins the Objective-C runtime name that
/// `InputMethodServerControllerClass` looks up in the bundle's Info.plist.
@objc(TaigiInputController)
public final class TaigiInputController: IMKInputController {
    /// `static`: IMK builds one controller per client text session, and the
    /// category never varies, so a stored property would create an `os_log`
    /// handle per session.
    static let logger = DebugLogger(category: "InputController")

    /// This session's identity in the coordinator. Allocated at construction
    /// rather than read from the client, because identifying a client means
    /// asking it — see the activation rule on `activateServer` below.
    let sessionToken = ComposingSessionToken()

    /// Whether this controller last left a marked region in its client. Kept
    /// here rather than asked of the back end because the case that needs it is
    /// exactly the one where the back end no longer speaks for this session — see
    /// `finishComposition(into:)`.
    @MainActor
    private var isMarkedTextVisible = false

    /// What runs the key path: the composition, the candidate list, the
    /// commits. This controller replays what it answers (`replay`).
    @MainActor
    var backend: ComposingBackend {
        ComposingBackend.shared
    }

    /// Whether this session's candidate list is on screen — true once the
    /// window took a list, false from any dismissal on. Sent with every
    /// request (`ComposingPanelState.isListOnScreen`): the back end's list is
    /// only ever the one the user can see, so the arrows and the slot keys
    /// never reach candidates behind a window that is not up.
    @MainActor
    private var isCandidateListOnScreen = false

    /// Counts this controller's activations and teardowns, so a replay can
    /// tell that its reply belongs to a tenure that has since ended — even
    /// one a client callback ended and began again with the same token
    /// (`replay`).
    @MainActor
    private var tenure = 0

    /// An activated controller remains eligible for keys even if a system
    /// client activates afterward. Teardown revokes that eligibility.
    @MainActor
    private var isSessionActive = false

    /// Log delivery transitions, never individual keys or their contents.
    /// Reset on lifecycle changes so the first key establishes whether the
    /// host resumed delivery to this tenure at all.
    private enum KeyDelivery: String {
        case ready
        case inactiveSession
        case missingClient
    }

    @MainActor
    private var lastKeyDelivery: KeyDelivery?

    /// Where the candidate bar is shown. Backed by an optional so a test can
    /// substitute a double before the first key event: the shipped bar is an
    /// `NSPanel`, and the default cannot be written as a stored property's
    /// initial value because that expression is evaluated outside the main actor.
    @MainActor
    private var injectedPresenter: (any CandidatePresenter)?

    @MainActor
    var candidatePresenter: any CandidatePresenter {
        get { injectedPresenter ?? CandidatePanel.shared }
        set { injectedPresenter = newValue }
    }

    /// Where the symbol picker is shown — the second `CandidatePanel`, over
    /// its own owner slot (`CandidatePanel.symbolPicker`). Injectable for the
    /// same reason `candidatePresenter` is.
    @MainActor
    private var injectedSymbolPickerPresenter: (any CandidatePresenter)?

    @MainActor
    var symbolPickerPresenter: any CandidatePresenter {
        get { injectedSymbolPickerPresenter ?? CandidatePanel.symbolPicker }
        set { injectedSymbolPickerPresenter = newValue }
    }

    /// The list the open picker shows, cell by cell, and empty while no
    /// picker is up — the one piece of picker state the controller keeps:
    /// the selection is the window's (`CandidatePresenter`), as for the
    /// bar. Held rather than re-derived at pick time, so the index the
    /// window answers is read against the list it was shown, whatever the
    /// recents say by then.
    @MainActor
    var symbolPickerCells: [String] = []

    /// Whether the symbol picker is up for this session.
    @MainActor
    var isSymbolPickerOpen: Bool {
        !symbolPickerCells.isEmpty
    }

    /// Whether the open picker put its placeholder into the client
    /// (`presentSymbolPicker`) — it does not over a selection — so
    /// `dismissSymbolPicker` clears exactly what was written and never an
    /// empty region the client did not have.
    @MainActor
    var isSymbolPickerPlaceholderMarked = false

    /// The chord on the picker row, cached at activation and refreshed via
    /// KVO on `UserDefaults.standard` rather than decoded on every keystroke:
    /// the registry read decodes JSON out of `UserDefaults`, and opening
    /// Settings does not always deactivate and re-activate the session.
    @MainActor
    var symbolPickerShortcut: KeyboardShortcuts.Shortcut?

    private static let symbolPickerShortcutKey = SettingsKey(
        name: "KeyboardShortcuts_\(KeyboardShortcuts.Name.showSymbolPicker.rawValue)",
        defaultValue: "",
    )

    /// The display language the chrome renders in — the process-wide store,
    /// or the one a test injected.
    @MainActor
    private var displayLanguage: DisplayLanguageStore {
        displayLanguageOverride ?? DisplayLanguageStore.shared
    }

    /// Reads and writes the user's settings for the input-source menu. Its own
    /// instance rather than a shared one: the store holds no state, and the
    /// engine's copy is constructed at the composition root
    /// (`ComposingSessionCoordinator.shared`) — both read the same defaults
    /// domain, so a mode written here is what the next keystroke composes with.
    ///
    /// Settable so a test can point it at its own suite; the shipped one writes
    /// the settings of whoever is running the tests.
    var settings = SettingsStore()

    /// The display language the input-source menu renders in. Injectable for the same reason
    /// `settings` is: a test drives it from its own defaults suite rather than the machine's.
    ///
    /// `nil` means the process-wide store, which is what production runs with; only a test sets
    /// this. Optional rather than defaulted because the shared instance is main-actor-isolated and a
    /// stored default would have to be evaluated where this class is not.
    var displayLanguageOverride: DisplayLanguageStore?

    /// How a menu doorway puts the settings window up, and the update checker
    /// the Check for Updates row drives. Both injectable for the same reason
    /// `displayLanguageOverride` above is, and `nil` means production: a test
    /// that drove the shipped pair would order a real window in front of
    /// whoever is running it, and reach the network.
    @MainActor
    var settingsPresenterOverride: (@MainActor () -> Void)?

    @MainActor
    var updateCheckerOverride: UpdateChecker?

    /// Where the caret sat right after this controller's last auto-inserted
    /// trailing space — armed only when the client answered a collapsed
    /// selection there, and only until the very next key event, which either
    /// swaps an attaching punctuation with that space or invalidates it.
    /// Controller state rather than something read from the document because a
    /// committed document cannot be reliably read back under IMK; the stored
    /// caret is re-checked against the client before any rewrite, so a caret
    /// moved by a mouse click this keydown-only controller never saw degrades
    /// to no swap rather than to deleting a character that was not our space.
    /// Where the caret sat when this controller wrote a trailing auto space —
    /// the position the swap re-verifies against the client before it rewrites
    /// anything.
    ///
    /// Its EXISTENCE is the verdict: the arm is only ever set after a commit
    /// that wrote romanization earned its space, so the swap does not re-ask
    /// what the space was for. That is a FACT about a commit that already
    /// happened — the word in front of the caret does not become Hanji because
    /// the user changed the display mode afterwards. Only Auto-Space itself is
    /// re-read live at swap time, because turning the feature off should stop
    /// it.
    var armedAutoSpaceCaret: Int?

    /// Where the mode flash goes. `nil` means the shared HUD panel; a test injects a
    /// recorder, for the same reason `candidatePresenter` is injectable — the
    /// shipped one puts a real window on screen.
    @MainActor
    var modeFlashOverride: ((String) -> Void)?

    /// The client this session belongs to, learned at activation — which always
    /// precedes any key event, because a session that never activated never
    /// claimed the engine. `inputControllerWillClose()` gets no sender, and this
    /// is the client holding whatever marked region has to be finished there.
    /// Weak because the client belongs to the host, not to us.
    @MainActor
    weak var lastClient: (any IMKTextInput)?

    /// KVO on the Candidate Display key so an open bar is fetched again when the Appearance
    /// pane (or `defaults write`) changes it; armed in `activateServer`,
    /// released in `endSession` — the shortcut target's lifetime.
    @MainActor
    private var displayModeObservation: AnyObject?

    /// KVO on the Show Candidate Window key, armed and released with `displayModeObservation`:
    /// a flip mid-composition takes an open window down (off) or fetches
    /// one for the composition as it stands (on), rather than waiting for
    /// the next keystroke to notice — the window and the setting must not
    /// disagree while the user is looking at both.
    @MainActor
    private var candidateWindowObservation: AnyObject?

    /// KVO on the input mode, armed and released with `displayModeObservation`:
    /// a mode written from outside this session — the General pane, a
    /// `defaults write` — takes down what was raised under the old one, as
    /// the switch chords do (`switchInputMode(_:)`). The General pane takes
    /// focus first, which ends the session and commits the composition; a
    /// `defaults write` leaves it on screen, and across TPS the next key
    /// commits it (desktop TPS roadmap D5).
    @MainActor
    private var inputModeObservation: AnyObject?

    @MainActor
    private var keyboardLayoutObservation: AnyObject?

    /// Re-applied on every activation, even when this controller is reused.
    @MainActor
    private var appliedKeyboardLayout: KeyboardLayout?

    /// KVO on the TPS key panel's stored wish, armed and released with
    /// `displayModeObservation`: the chord in another session, or a reset of
    /// General, takes this session's panel up or down at once.
    @MainActor
    private var tpsKeyboardObservation: AnyObject?

    /// KVO on the Telex guide's stored wish, armed and released with
    /// `displayModeObservation`: the chord in another session, or a reset of
    /// General, takes this session's guide up or down at once.
    @MainActor
    private var telexGuideObservation: AnyObject?

    /// KVO on the display language, armed and released with
    /// `displayModeObservation`: an open guide is respelled in the new one.
    @MainActor
    private var displayLanguageObservation: AnyObject?

    /// KVO on the `KeyboardShortcuts` registry key for `.showSymbolPicker` in
    /// `UserDefaults.standard`, so rebinding or clearing the shortcut refreshes
    /// `symbolPickerShortcut` without waiting for re-activation.
    @MainActor
    private var symbolPickerShortcutObservation: AnyObject?

    /// Suppresses redundant async KVO callbacks for writes this controller just
    /// applied synchronously (`switchInputMode`, `cycleCandidateDisplayMode`).
    private let inputModeWriteGuard = SelfWriteGuard()
    private let displayModeWriteGuard = SelfWriteGuard()

    // MARK: - IMK entry points

    /// Keydown only — the default, restated rather than left implicit so a
    /// future edit meets the cost of widening it before paying it.
    ///
    /// It carried `flagsChanged` too until 2026-08-26, for a solo-Shift English (ABC)
    /// toggle this input method no longer implements — it has no English mode
    /// at all now (USER): a Mac switches input sources with ⌘Space, and
    /// switching hands the user the real ABC source rather than a
    /// pass-through imitation of one. Nothing reads a modifier transition any
    /// more, and a mask that keeps asking for them is an input method owning
    /// events it does not act on.
    ///
    /// Widening past keydown costs IMK's automatic `commitComposition:` on a
    /// click outside the composition (`IMKInputController.h:154-157`), which is
    /// why the `commitComposition(_:)` override below force-commits — kept even
    /// now that the mask is narrow again, because it is also what the manual
    /// commit paths call. `TaigiInputControllerTests` pins the mask.
    override public func recognizedEvents(_: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    /// CHROMIUM DEADLOCK RULE — never query the client synchronously from here
    /// (`attributesForCharacterIndex:lineHeightRectangle:`, `selectedRange`,
    /// `markedRange`, `length`, `attributedSubstringFromRange:`, …). Chromium
    /// hosts deadlock on a synchronous round-trip during activation
    /// (Chromium issue 503787240, hit by azooKey-Desktop). Pinned by
    /// `ActivateServerClientQueryTests`.
    override public func activateServer(_ sender: Any!) {
        onMainActor(sender) { controller, client in
            controller.tenure += 1
            let activatingTenure = controller.tenure
            controller.lastKeyDelivery = nil
            Self.logger.notice(
                "activate begin session=\(controller.sessionToken.value) tenure=\(activatingTenure) client=\(client != nil)",
            )
            controller.lastClient = client
            controller.isSessionActive = true
            let coordinator = ComposingSessionCoordinator.shared
            coordinator.registerActiveSession(controller, for: controller.sessionToken)
            controller.appliedKeyboardLayout = nil
            controller.applyKeyboardLayout()
            // Key-scoped KVO through the store rather than a notification, so
            // a `defaults write` from outside the process reaches the bar too.
            controller.displayModeObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.candidateDisplayMode,
                skipping: controller.displayModeWriteGuard,
                onMainActor: { [weak controller] in
                    controller?.representCandidates(refetch: true)
                },
            )
            controller.candidateWindowObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.isCandidateWindowEnabled,
                onMainActor: { [weak controller] in
                    controller?.applyCandidateWindowSettingChange()
                },
            )
            controller.inputModeObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.inputMode,
                skipping: controller.inputModeWriteGuard,
                onMainActor: { [weak controller] in
                    controller?.dismissPalettes()
                    controller?.applyKeyboardLayout()
                    controller?.syncTpsKeyboard()
                    controller?.syncTelexGuide()
                },
            )
            controller.keyboardLayoutObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.keyboardLayout,
                onMainActor: { [weak controller] in
                    controller?.applyKeyboardLayout()
                },
            )
            controller.tpsKeyboardObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.isTpsKeyboardShown,
                onMainActor: { [weak controller] in
                    controller?.syncTpsKeyboard()
                },
            )
            controller.telexGuideObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.isTelexGuideShown,
                onMainActor: { [weak controller] in
                    controller?.syncTelexGuide()
                },
            )
            controller.displayLanguageObservation = controller.settings.observeChanges(
                of: SettingsStore.Keys.displayLanguage,
                onMainActor: { [weak controller] in
                    // The store may not have heard yet; its own observation
                    // has no order against this one.
                    controller?.displayLanguage.syncFromSettings()
                    controller?.syncTelexGuide()
                },
            )
            controller.refreshSymbolPickerShortcut()
            controller.symbolPickerShortcutObservation = SettingsStore(userDefaults: .standard)
                .observeChangesImmediatelyIfMain(of: Self.symbolPickerShortcutKey) { [weak controller] in
                    controller?.refreshSymbolPickerShortcut()
                }
            // Whatever space a previous focus left armed was measured against
            // a document this activation may no longer be looking at.
            controller.armedAutoSpaceCaret = nil
            // Extra activations must not reset another live session's engine
            // or hide its candidates. A delivered key selects a different
            // active controller; a sole waiting one resumes after teardown.
            if !coordinator.hasActiveOwner || controller.backend.owns(controller.sessionToken) {
                controller.takeInputOwnership()
            } else {
                Self.logger.notice("activation waiting session=\(controller.sessionToken.value) tenure=\(activatingTenure)")
            }
            Self.logger.notice(
                "activate end session=\(controller.sessionToken.value) tenure=\(activatingTenure) currentTenure=\(controller.tenure) owns=\(controller.backend.owns(controller.sessionToken))",
            )
        }
    }

    @MainActor
    private func refreshSymbolPickerShortcut() {
        symbolPickerShortcut = KeyboardShortcuts.getShortcut(for: .showSymbolPicker)
    }

    /// Claims the engine and the shared UI for an eligible controller. Called
    /// for an unowned activation, actual key delivery, or the sole session
    /// left after an owner ends. No client query occurs here.
    @MainActor
    func takeInputOwnership() {
        guard isSessionActive else { return }
        backend.activate(sessionToken, settings: settings)
        ComposingSessionCoordinator.shared.registerShortcutTarget(self, for: sessionToken)
        candidatePresenter.hideForHandover()
        isCandidateListOnScreen = false
        symbolPickerPresenter.hideForHandover()
        symbolPickerCells = []
        isSymbolPickerPlaceholderMarked = false
        armedAutoSpaceCaret = nil
        syncTpsKeyboard()
        syncTelexGuide()
    }

    /// The session is losing focus. The composition is finished into the
    /// document rather than dropped — the user typed those characters — and
    /// ownership is given up in the same step: IMK sends no further key events
    /// to a deactivated session, so holding the engine after this point would
    /// only let a stray callback write into the session that takes over.
    override public func deactivateServer(_ sender: Any!) {
        onMainActor(sender) { controller, client in controller.endSession(client, reason: "deactivate") }
    }

    /// Deliberately does not call `super`. `IMKInputController`'s implementation
    /// re-enters the controller to flush its own notion of the composition;
    /// the engine owns this one. Reached when the user clicks outside the
    /// marked region, which the keydown-only event mask is what earns us.
    ///
    /// Ownership is kept: the session is still focused, and the user carries on
    /// typing into it after the click.
    override public func commitComposition(_ sender: Any!) {
        onMainActor(sender) { controller, client in
            Self.logger.notice("commitComposition session=\(controller.sessionToken.value) tenure=\(controller.tenure)")
            controller.finishComposition(into: client)
        }
    }

    /// The only teardown hook every controller is guaranteed to receive, and
    /// the last chance to tidy up after a session that is closing without ever
    /// having been deactivated: the composition has to be finished into the
    /// client here, or the user's characters vanish with the session — and the
    /// engine has to be released, or every session after this one is mute.
    ///
    /// This hook gets no sender, so it finishes into the client the session last
    /// wrote to — the one whose marked region is at stake.
    override public func inputControllerWillClose() {
        onMainActor(nil) { controller, _ in controller.endSession(controller.lastClient, reason: "close") }
    }

    /// Take down every window this input method is showing, and nothing else.
    ///
    /// Apple's contract is UI-only: the system sends this when its own interface
    /// needs the screen, not when the composition is over, so releasing the
    /// engine or committing here would throw away work the user is in the middle
    /// of. The candidate model is cleared with the window because the two are one
    /// state as far as the key contract is concerned — the arrows and the slot
    /// keys belong to a bar the user can see, and with the bar gone they go
    /// back to the host until the next keystroke fetches candidates again. The
    /// selection latch goes with them for the same reason (`dismissCandidates`):
    /// the digits were picking out of a list that is no longer on screen.
    override public func hidePalettes() {
        Self.logger.debug("hidePalettes")
        onMainActor(nil) { controller, _ in
            controller.dismissPalettes()
            TpsKeyboardPanel.shared.hide(ownedBy: controller.sessionToken)
            TelexGuidePanel.shared.hide(ownedBy: controller.sessionToken)
        }
        super.hidePalettes()
    }

    override public func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event else { return false }
        switch event.type {
        case .keyDown:
            // Snapshotted before the hop: `NSEvent` is a reference type that
            // cannot cross an isolation boundary.
            let key = KeyEventSnapshot(event)
            return onMainActor(sender) { controller, client in controller.handle(key, client: client) }
        default:
            return false
        }
    }

    /// One of the two switch chords, through the core's one mode writer
    /// (`SettingsStore.switchInputMode(_:)`). A composition across TPS stays
    /// on screen until the next key commits it (desktop TPS roadmap D5).
    @MainActor
    private func switchInputMode(_ request: InputModeSwitch) {
        Self.logger.debug("switch input mode \(request)")
        guard let mode = inputModeWriteGuard.perform(
            { settings.switchInputMode(request) },
            committed: { $0 != nil },
        ) else { return }
        // The candidates on screen were fetched under the old mode, and
        // the key contract lets Space commit whichever one is
        // highlighted; the picker was raised under it too. They go with
        // the mode that produced them here, and `inputModeWriteGuard`
        // suppresses the self-triggered KVO `dismissPalettes()` so a key
        // or picker opened immediately afterward cannot be torn down later.
        dismissPalettes()
        applyKeyboardLayout()
        // The TPS key panel comes and goes with TPS; its stored wish stays.
        syncTpsKeyboard()
        // The guide goes under TPS, and stays up for romanizations.
        syncTelexGuide()
        // Then the HUD, because the chord fires from anywhere and a
        // mode that changed with
        // no notice reads as the keyboard breaking — the next syllable
        // composes under rules nothing on screen said had changed
        // (USER 2026-08-26).
        flash(mode.displayNameKey)
    }

    /// The list and the picker taken down without touching the composition:
    /// what `hidePalettes` is asked for, and what `inputModeObservation` does
    /// for a mode written outside this session, each window having been raised
    /// under the old one. The TPS key panel and the Telex guide follow the mode
    /// on their own (`syncTpsKeyboard`, `syncTelexGuide`).
    @MainActor
    private func dismissPalettes() {
        dismissCandidates()
        dismissSymbolPicker()
    }

    /// The TPS key panel as the settings now say, for this session — only
    /// while it holds the engine, so a late observation in a session already
    /// handed over cannot take the panel back (`ComposingSessionCoordinator`).
    @MainActor
    private func syncTpsKeyboard() {
        guard backend.owns(sessionToken) else { return }
        if settings.isTpsKeyboardWanted {
            TpsKeyboardPanel.shared.show(ownedBy: sessionToken, target: self, generation: tenure)
        } else {
            TpsKeyboardPanel.shared.hideNow()
        }
    }

    /// The Telex guide as the settings now say, for this session — only while
    /// it holds the engine, as `syncTpsKeyboard`.
    @MainActor
    private func syncTelexGuide() {
        guard backend.owns(sessionToken) else { return }
        if settings.isTelexGuideWanted {
            TelexGuidePanel.shared.show(
                inputMode: settings.inputMode,
                language: displayLanguage,
                ownedBy: sessionToken,
            )
        } else {
            TelexGuidePanel.shared.hideNow()
        }
    }

    /// IMKInputSession.h requires the layout override on every activation.
    /// This setter asks no document questions, preserving the Chromium
    /// activation rule. The client receives events translated by macOS,
    /// including printable keys our composing classifier passes through.
    @MainActor
    private func applyKeyboardLayout() {
        guard isSessionActive, let client = lastClient else { return }
        let layout: KeyboardLayout = settings.inputMode == .tps ? .qwerty : settings.keyboardLayout
        guard appliedKeyboardLayout != layout else { return }
        client.overrideKeyboard(withKeyboardNamed: layout.inputSourceID)
        appliedKeyboardLayout = layout
    }

    // MARK: - Shortcut actions

    /// What a recorded chord does while this session owns the engine. A
    /// setting has one behaviour regardless of which surface changed it. What
    /// happens to the candidate bar follows what the setting invalidates: a
    /// romanization switch changes what a fetch would return, so its bar comes
    /// down (same rule as `switchInputMode(to:)`); the Hanji/romanization swap changes only
    /// how the same candidates display, so its bar stays and re-renders; the
    /// Candidate Display cycle changes which candidates exist, so its bar stays and is
    /// fetched again.
    @MainActor
    func performShortcutAction(_ action: ShortcutAction) {
        // Inert under the mode: no write, no flash, and nothing on screen
        // taken down — first, before the teardown below (Windows and Linux
        // gate there too).
        guard !action.isInert(under: settings.inputMode) else { return }
        // The two mode switches tear down in `switchInputMode(_:)`, once the
        // write has landed: a switch the core did not answer changes nothing.
        if action != .toggleRomanization, action != .toggleTps {
            // The picker: a Carbon chord bypasses the key path that would
            // otherwise take it down.
            dismissSymbolPicker()
        }
        switch action {
        case .openLastSettingsPane:
            // Handled process-wide by `ShortcutHotkeys.perform` before any
            // session is consulted: opening a window needs no client, and a
            // user with no focused Taigi session still expects the chord to
            // work. Named rather than defaulted so a new action cannot fall
            // silently into "do nothing".
            break
        case .toggleRomanization:
            switchInputMode(.toggleRomanization)
        case .toggleTps:
            switchInputMode(.toggleTps)
        case .toggleTranslateSwapped:
            // Inert under romanization-only — silently, no flash (USER
            // 2026-09-01, Q11; see `allowsSwapToggle`). Under Hanji with Romanization the chord
            // flips only the punctuation width (desktop-core `effective_full_width_punctuation`).
            guard settings.candidateDisplayMode.allowsSwapToggle else { return }
            // The bar STAYS: the SWAP changes how a candidate displays and
            // commits, so the list on screen is re-rendered, selection kept.
            // Dismissing here read as the window vanishing (real device,
            // 2026-08-21). With romanized words as candidates under side by
            // side it also changes which candidates exist, so the list is
            // fetched again and the selection follows its word
            // (`swapChangesTheList`, `CandidateListUpdate.selectedIndex`).
            let refetch = settings.swapChangesTheList
            settings.storedIsHanjiFirst.toggle()
            representCandidates(refetch: refetch)
            // Flashed even over an open list, as the display-mode cycle is
            // (USER 2026-10-31): the menu-bar icon is one static mark (D5),
            // and a re-rendered list does not name what a commit now writes.
            // Side by side only: under Hanji with Romanization the chord
            // flips the punctuation width and a commit still writes the
            // Hanji, so a flash would announce a change that did not happen.
            if settings.candidateDisplayMode == .sideBySide {
                flash(settings.storedIsHanjiFirst ? .settingsOutputScriptHanji : .settingsOutputScriptRoman)
            }
        case .cycleCandidateDisplayMode:
            // Inert only under TPS (gated above). Refetched and repainted
            // synchronously BEFORE flashing the mode banner (matching Windows
            // `session/mod.rs` and desktop-core `global.rs`), so the announced
            // mode never leads an unchanged strip; `displayModeWriteGuard`
            // suppresses the redundant self-triggered KVO refetch while leaving
            // external writes (Appearance pane, `defaults write`) updating the
            // open bar.
            // Flashed even over an open list, as the swap is: the strip
            // changes shape, and a strip that did so with no notice reads as
            // breakage — the same rule as the romanization switch.
            let next = settings.candidateDisplayMode.next
            displayModeWriteGuard.perform {
                settings.candidateDisplayMode = next
            }
            representCandidates(refetch: true)
            flash(next.displayNameKey)
        case .showTelexGuide:
            // The wish is stored for every session to read; this one's guide
            // follows now rather than on the observation's later hop. The
            // composition and its list are not touched.
            settings.isTelexGuideShown.toggle()
            syncTelexGuide()
        case .showSymbolPicker:
            // Matched in `handle`, never fired from here (`firesFromTheKeyPath`).
            break
        case .showTpsKeyboard:
            // The wish is stored for every session to read; this one's panel
            // follows now rather than on the observation's later hop. The
            // composition and its list are not touched.
            settings.isTpsKeyboardShown.toggle()
            syncTpsKeyboard()
        }
    }

    /// The list on screen again under the settings just written
    /// (`ComposingBackend.represent`): re-rendered after the Hanji/romanization
    /// swap, fetched again (`refetch`) after a Candidate Display change, which
    /// alters WHICH candidates exist. Repainted through `updateCells`, not
    /// anchored again: the Carbon hotkey and the settings observation have no
    /// client to ask for a caret rectangle — and need none, because the
    /// window is already anchored.
    @MainActor
    private func representCandidates(refetch: Bool) {
        switch backend.represent(refetch: refetch, in: request(client: nil, armedSwap: nil)) {
        case nil, .unchanged:
            break
        case let .changed(list):
            candidatePresenter.updateCells(windowContent(list), ownedBy: sessionToken)
        case .closed:
            dismissCandidates()
        }
    }

    /// A list as the window takes it — built the same way for the fresh-list
    /// path and the in-place update, so a repaint can never draw a different
    /// key row than a `show` of the same list would.
    @MainActor
    private func windowContent(_ list: CandidateListUpdate) -> CandidateWindowContent {
        CandidateWindowContent(
            cells: list.cells,
            selectedIndex: list.selectedIndex,
            // The set the user chose — the only keys that pick. A rebind
            // cannot strand a stale hint: reaching the shortcut pane moves
            // focus off the client, and `finishComposition` takes the bar
            // down with the session.
            slotKeySet: settings.candidateSlotKeySet,
            // The §34 literal is what the user is already typing, not an offer
            // to pick, so it takes no key and the keys start on the cell after
            // it (USER 2026-09-09).
            leadCellIsUnkeyed: list.leadsWithLiteralRoman,
        )
    }

    // MARK: - Main-actor work

    @MainActor
    private func handle(_ key: KeyEventSnapshot, client: IMKTextInput?) -> Bool {
        // Every key gets exactly one chance at the swap: the arm is consumed
        // here — before ANY early return, so an event this session cannot
        // handle still invalidates it — and only the auto-space paths of the
        // back end re-arm it (`replay`). A key that went anywhere else changed
        // the document or the caret, and a swap after that would be rewriting
        // text it never measured.
        let armedSwap = armedAutoSpaceCaret
        armedAutoSpaceCaret = nil

        // The event itself identifies the session being typed in. Activation
        // alone cannot: another active IMK client may have activated later.
        // A retired controller is never allowed to reclaim through a late key.
        if isSessionActive, client != nil, !backend.owns(sessionToken) {
            Self.logger.notice("key selects session=\(sessionToken.value) tenure=\(tenure)")
            takeInputOwnership()
        }
        let ownsSession = isSessionActive && backend.owns(sessionToken)
        let delivery: KeyDelivery = ownsSession ? (client == nil ? .missingClient : .ready) : .inactiveSession
        if lastKeyDelivery != delivery {
            lastKeyDelivery = delivery
            if delivery == .ready {
                Self.logger.notice("key delivery ready session=\(sessionToken.value) tenure=\(tenure)")
            } else {
                Self.logger.error(
                    "key delivery rejected session=\(sessionToken.value) tenure=\(tenure) reason=\(delivery.rawValue)",
                )
            }
        }
        guard ownsSession, let client else { return false }

        defer { isMarkedTextVisible = backend.isComposing(sessionToken) }

        // The picker chord toggles, so it is read before the picker's own
        // keys: with the list up it is the second way out (Escape is the
        // first), and a held chord — the keyboard's auto-repeat — is one
        // press, not a flicker of open and closed. The chord itself touches
        // neither the document nor the caret, so the arm it consumed on entry
        // goes back; a commit on the way to opening re-arms or clears it.
        if isSymbolPickerChord(key) {
            armedAutoSpaceCaret = armedSwap
            guard !key.isRepeat else { return true }
            if isSymbolPickerOpen {
                dismissSymbolPicker()
            } else {
                openSymbolPicker(client: client)
            }
            return true
        }
        // With the picker up, every key is the picker's first: it is a list
        // to pick from, read before the composing contract so the slot keys
        // and the arrows reach it rather than a bar that is not showing.
        // A key the picker has no use for takes it down and goes on below.
        // The arm goes back for the same reason as above; only a pick that
        // writes consumes it (`insertSymbol`), and a key that falls through
        // clears it again so the contract below sees what it always does.
        if isSymbolPickerOpen {
            armedAutoSpaceCaret = armedSwap
            if handleSymbolPickerKey(key, client: client) {
                return true
            }
            armedAutoSpaceCaret = nil
        }

        var handled = false
        send(client: client, armedSwap: armedSwap) { request in
            backend.key(key, in: request).map { reply in
                handled = reply.handled
                return reply.effects
            }
        }
        return handled
    }

    /// Sends one request made with `client` and replays the answer, with the
    /// same arm on both sides. False when this session does not own the
    /// engine — nothing was answered, nothing replayed.
    @MainActor
    @discardableResult
    func send(
        client: IMKTextInput,
        armedSwap: Int?,
        _ call: (ComposingRequest) -> [ComposingEffect]?,
    ) -> Bool {
        guard let effects = call(request(client: client, armedSwap: armedSwap)) else { return false }
        replay(effects, into: client, armedSwap: armedSwap)
        return true
    }

    /// A request from this session: its window and, with a client and an arm
    /// (the one this key consumed on entry, not whatever is stored by now),
    /// the swap's client check.
    @MainActor
    private func request(client: IMKTextInput?, armedSwap: Int?) -> ComposingRequest {
        var canSwap: (@MainActor () -> Bool)?
        if let armedSwap, let client {
            canSwap = { Self.canSwapAutoSpace(armedAt: armedSwap, in: client) }
        }
        return ComposingRequest(
            session: sessionToken,
            settings: settings,
            panel: ComposingPanelState(
                isListOnScreen: isCandidateListOnScreen,
                selectedIndex: { [unowned self] in
                    candidatePresenter.selectedCandidateIndex(ownedBy: sessionToken)
                },
                indexForKeySlot: { [unowned self] slot in
                    candidatePresenter.candidateIndex(forKeySlot: slot, ownedBy: sessionToken)
                },
                canSwapPrecedingSpace: canSwap,
            ),
        )
    }

    /// Does what the back end answered to a request made with `client`, in
    /// its order, to the client and the window (a represent has its own,
    /// `representCandidates`).
    @MainActor
    private func replay(_ effects: [ComposingEffect], into client: IMKTextInput, armedSwap: Int?) {
        // Where a swap in this reply left the caret: the re-arm that follows
        // it is arithmetic, not another `selectedRange()` query — the
        // rewrite's end is fully determined by the range just replaced, and
        // the next swap re-verifies the position against the client anyway,
        // so a client that moved the caret degrades to no swap.
        var caretAfterSwap: Int?
        // Where this reply's writes begin, asked before the first of them,
        // and how far they have carried the caret: an arm after a commit is
        // arithmetic too (`writeStart(in:)`).
        let writeStart = effects.contains(.armSwap) ? Self.writeStart(in: client) : nil
        var writtenUTF16 = 0
        let writer = ClientWriter(client: client)
        let replayTenure = tenure
        for effect in effects {
            // A client callback can take the focus away mid-replay; what the
            // reply still holds was meant for a tenure that is over — its
            // teardown already finished the composition — or for a session
            // another controller now drives (roadmap D3, stale replies).
            guard tenure == replayTenure, backend.owns(sessionToken) else {
                Self.logger.notice(
                    "replay discarded session=\(sessionToken.value) tenure=\(replayTenure) currentTenure=\(tenure) owns=\(backend.owns(sessionToken))",
                )
                return
            }
            switch effect {
            case let .setMarkedText(text, caretUTF16):
                writer.setMarkedText(text, caretUTF16: caretUTF16)
            case .clearMarkedText:
                writer.clearMarkedText()
            case let .insertText(text):
                // One `insertText` at the insertion point — a commit, the
                // auto space, a mapped punctuation or a symbol alike.
                writer.insertText(text)
                writtenUTF16 += (text as NSString).length
            case let .swapPrecedingSpace(replacement):
                // Only ever answered after `canSwapPrecedingSpace` found the
                // arm and the space still in front of the caret.
                guard let armedSwap else { preconditionFailure("a swap answered with no arm") }
                client.insertText(replacement, replacementRange: Self.autoSpaceRange(armedAt: armedSwap))
                caretAfterSwap = armedSwap - 1 + (replacement as NSString).length
            case .armSwap:
                if let caretAfterSwap {
                    armedAutoSpaceCaret = caretAfterSwap
                } else if let writeStart, writeStart + writtenUTF16 > 0 {
                    armedAutoSpaceCaret = writeStart + writtenUTF16
                }
            case let .candidatesChanged(list):
                presentCandidates(list, client: client)
            case .candidatesClosed:
                dismissCandidates()
            case let .navigate(direction):
                // The window interprets the direction for its layout and
                // repaints itself — it is authoritative for the selection.
                candidatePresenter.navigate(direction, ownedBy: sessionToken)
            case let .tpsKeyboardKeyTyped(row, cap):
                TpsKeyboardPanel.shared.flash(row: row, cap: cap, ownedBy: sessionToken)
            }
        }
    }

    /// Announces a mode the user just switched into, through the injected
    /// recorder in tests and the shared HUD in production.
    ///
    /// The romanization switch, the Candidate Display cycle and — with no
    /// list on screen — the Hanji/romanization swap. The English (ABC) toggle raised
    /// this too, until this input method stopped having an English mode
    /// (USER 2026-08-26). Switching input sources is the system's business
    /// and it draws its own indicator; a second one of ours over it would be
    /// announcing a mode nothing here owns.
    @MainActor
    private func flash(_ mode: StringKey) {
        let text = displayLanguage.string(mode)
        if let modeFlashOverride {
            modeFlashOverride(text)
        } else {
            ModeFlashPanel.shared.flash(text)
        }
    }

    // MARK: - Candidates

    /// Puts the list on screen, anchored to the caret. The window selects its
    /// first candidate — a fresh keystroke re-ranks the whole list, so a held
    /// position would sit on an unrelated word.
    @MainActor
    private func presentCandidates(_ list: CandidateListUpdate, client: IMKTextInput) {
        guard let caretRect = caretRect(in: client, anchorEnd: list.anchorEndUTF16)
        else {
            // A client that cannot say where its caret is cannot host a bar that
            // points at it, and one parked in the corner of the screen is worse
            // than none: it would claim to describe text somewhere else entirely.
            //
            // The list is dropped with the window, not merely hidden. The key
            // contract turns on `isShowingCandidates`, so a model kept alive
            // behind a hidden bar would swallow the arrows and let Space commit a
            // candidate the user cannot see.
            Self.logger.debug("no caret rectangle from the client — candidates stay hidden")
            dismissCandidates()
            return
        }

        candidatePresenter.show(
            windowContent(list),
            anchoredTo: caretRect,
            hostWindowLevel: client.windowLevel(),
            // Feeds the Multicolour accent resolution: when the system has no
            // fixed accent, the highlight takes the host app's own. Safe to ask
            // here — this runs inside a key event, like every client query.
            hostBundleIdentifier: client.bundleIdentifier(),
            ownedBy: sessionToken,
        )
        isCandidateListOnScreen = true
    }

    /// Brings the window into line with the Show Candidate Window setting the General pane (or
    /// `defaults write`) just flipped, for a composition that is running.
    ///
    /// Off takes the list and the window down together (`dismissCandidates`)
    /// and leaves the marked text alone: the user's characters are the
    /// engine's, and only the presentation was switched off. On waits for the
    /// next keystroke, which fetches for the composition as it stands: unlike
    /// the display-mode refetch there is no anchored window to repaint in
    /// place, so the caret would have to be asked for. An idle session has
    /// nothing to fetch or hide.
    @MainActor
    private func applyCandidateWindowSettingChange() {
        // Off takes the bar down at once; on waits for the next keystroke,
        // which fetches anyway. Fetching here would mean asking the client
        // for its caret outside a key event — the one IMK query this
        // controller otherwise never makes — for a window the next key
        // brings up on its own.
        guard !settings.isCandidateWindowEnabled else { return }
        dismissCandidates()
    }

    /// Takes the window down; the back end drops its list on the next
    /// request (`isCandidateListOnScreen`).
    @MainActor
    private func dismissCandidates() {
        isCandidateListOnScreen = false
        candidatePresenter.hide(ownedBy: sessionToken)
    }

    /// A click on a cap of the TPS key panel (desktop TPS roadmap D6): its
    /// glyph typed as the layout key would type it, through the core's own
    /// check of the mode and the glyph (`ComposingBackend.tpsKeyboardPress`).
    /// A mouse path into the composition, so it is checked the way
    /// `macos-roadmap.md` asks of one: the tenure the panel was shown in is
    /// still this one, this session still owns the engine, and its client
    /// is still alive — and the mode is still TPS, so a click that raced a
    /// switch touches nothing (the core refuses it too). The guide and the
    /// picker go down first, as for any key — and the check runs again after
    /// them: clearing the picker's placeholder is a client call, which can
    /// end this tenure. The auto-space arm is spent, since a glyph lands at
    /// the caret.
    @MainActor
    func typeTpsKeyboardGlyph(_ glyph: String, generation: Int) {
        let liveClient = { [self] () -> IMKTextInput? in
            guard generation == tenure, backend.owns(sessionToken) else { return nil }
            return lastClient
        }
        guard settings.inputMode == .tps, liveClient() != nil else { return }
        armedAutoSpaceCaret = nil
        dismissSymbolPicker()
        guard let client = liveClient() else { return }
        defer { isMarkedTextVisible = backend.isComposing(sessionToken) }
        send(client: client, armedSwap: nil) { backend.tpsKeyboardPress(glyph, in: $0) }
    }

    /// Where the character of the marked text that ends at `anchorEnd` is
    /// drawn, in screen coordinates — the composition's last character, or
    /// under TPS the last character before the caret.
    ///
    /// Walks back from there until the client answers with a real rectangle,
    /// matching McBopomofo
    /// (`references/McBopomofo/Source/InputMethodController.swift:886-891`).
    /// Index 0 would be wrong twice over: it is the START of the marked region
    /// rather than the caret, so the bar would drift further from the insertion
    /// point the longer the composition got, and some clients answer for that
    /// index with a zero rectangle they will happily give a later one for.
    ///
    /// "The client did not answer" is read as a rectangle left entirely at zero,
    /// not merely one at the screen origin: a caret really drawn at `(0, 0)` —
    /// the bottom-left corner of the leftmost display — still reports its line
    /// height, and rejecting it would hide the bar for a client that answered
    /// perfectly well. McBopomofo tests the origin alone
    /// (`InputMethodController.swift:886`); this is the same walk with the
    /// narrower rejection.
    ///
    /// Safe to ask here and only here: the deadlock this call causes in Chromium
    /// hosts is specific to activation (see `activateServer`).
    @MainActor
    func caretRect(in client: IMKTextInput, anchorEnd: Int) -> CGRect? {
        var index = max(anchorEnd - 1, 0)
        while index >= 0 {
            var lineHeightRect = CGRect.zero
            _ = client.attributes(forCharacterIndex: index, lineHeightRectangle: &lineHeightRect)
            if lineHeightRect != .zero {
                return lineHeightRect
            }
            index -= 1
        }
        return nil
    }

    /// Finishes the composition into `client` and gives up the engine, for a
    /// session that is going away.
    @MainActor
    private func endSession(_ client: IMKTextInput?, reason: String) {
        tenure += 1
        let endingTenure = tenure
        isSessionActive = false
        ComposingSessionCoordinator.shared.unregisterActiveSession(sessionToken)
        lastKeyDelivery = nil
        Self.logger.notice(
            "end begin reason=\(reason) session=\(sessionToken.value) tenure=\(endingTenure) owns=\(backend.owns(sessionToken)) marked=\(isMarkedTextVisible) client=\(client != nil)",
        )
        // Owner-guarded, beside the bar's own dismissal in
        // `finishComposition`: a guide belongs to the session that raised it,
        // and goes when that session's focus does.
        TelexGuidePanel.shared.hide(ownedBy: sessionToken)
        TpsKeyboardPanel.shared.hide(ownedBy: sessionToken)
        finishComposition(into: client)
        // Committing into an IMK client can re-enter activation before the
        // write returns. The same controller then owns a new tenure; this
        // teardown must not release it or remove its settings observations.
        guard tenure == endingTenure else {
            Self.logger.notice(
                "end superseded session=\(sessionToken.value) tenure=\(endingTenure) currentTenure=\(tenure) owns=\(backend.owns(sessionToken))",
            )
            return
        }
        backend.release(sessionToken, settings: settings)
        displayModeObservation = nil
        candidateWindowObservation = nil
        inputModeObservation = nil
        keyboardLayoutObservation = nil
        appliedKeyboardLayout = nil
        tpsKeyboardObservation = nil
        telexGuideObservation = nil
        displayLanguageObservation = nil
        symbolPickerShortcutObservation = nil
        Self.logger.notice("end complete session=\(sessionToken.value) tenure=\(endingTenure)")
        ComposingSessionCoordinator.shared.resumeWaitingSession()
    }

    /// Writes whatever is composing into `client` and leaves it with no marked
    /// region.
    ///
    /// The two branches are not interchangeable. While this session owns the
    /// engine, committing is right: the user typed those characters and they
    /// belong in the document. Once another session has taken the engine, the
    /// composition is gone from Rust and only this client's marked region
    /// remains — committing is no longer possible, so the leftover is cleared
    /// instead. Clearing is skipped when nothing was marked: some clients
    /// mishandle an empty `setMarkedText` at teardown (McBopomofo issue #346,
    /// `references/McBopomofo/Source/InputMethodController.swift:485-489`).
    @MainActor
    private func finishComposition(into client: IMKTextInput?) {
        // Before the client check: the bar belongs to this session whether or not
        // it still has a client to write into, and a session on its way out that
        // leaves one on screen leaves it there for good. The picker likewise —
        // and on a click outside, where this is also reached, the caret it was
        // anchored to has moved.
        dismissCandidates()
        dismissSymbolPicker()
        // Focus is moving or the user clicked — either way the caret the swap
        // was measured against is gone. (No auto space is appended here
        // either: lifecycle commits are not a finished word.)
        armedAutoSpaceCaret = nil
        guard let client else { return }
        // Clear the outgoing mark before calling the client. A callback can
        // reactivate this controller and type a new composition; clearing in
        // a defer would erase that new tenure's marked-text bookkeeping.
        let wasMarkedTextVisible = isMarkedTextVisible
        isMarkedTextVisible = false

        guard !send(client: client, armedSwap: nil, { backend.commitComposition(in: $0) }),
              wasMarkedTextVisible
        else { return }
        ClientWriter(client: client).clearMarkedText()
    }

    // MARK: - Main-actor assertion

    /// Runs `body` on the main actor, where the composing session lives.
    ///
    /// `assumeIsolated` does not hop — it checks that the current executor is
    /// already the main one and traps if it is not. That is deliberate. IMK
    /// delivers its callbacks on the main run loop in practice but declares
    /// none of them isolated, and an override cannot add isolation its
    /// superclass declaration lacks, so the assumption cannot be expressed in
    /// the signature. Asserting it here keeps the composing types genuinely
    /// main-actor-isolated and turns a violated platform assumption into an
    /// immediate crash rather than a silent data race. Hopping asynchronously
    /// is not an option either: `handle(_:client:)` has to answer IMK
    /// synchronously with whether it consumed the key.
    /// `body` takes the controller as a parameter rather than capturing `self`:
    /// a closure that captured it could not cross into the main-actor context
    /// without the compiler treating a non-`Sendable` controller as sent.
    func onMainActor<T: Sendable>(
        _ sender: Any!,
        _ body: @MainActor @Sendable (TaigiInputController, IMKTextInput?) -> T,
    ) -> T {
        let arguments = CallbackArguments(controller: self, client: sender as? IMKTextInput)
        return MainActor.assumeIsolated { body(arguments.controller, arguments.client) }
    }

    /// What InputMethodKit hands a callback, carried into the assertion above.
    ///
    /// `@unchecked Sendable` because the compiler cannot check what holds here:
    /// these values are only ever read inside the enclosing callback, which the
    /// assertion has just established is running on the main actor, and neither
    /// is stored anywhere that outlives the call.
    private struct CallbackArguments: @unchecked Sendable {
        let controller: TaigiInputController
        let client: IMKTextInput?
    }
}

/// The method lives in the class body (it needs the private candidate state);
/// the conformance is stated here where it reads as the contract it is.
extension TaigiInputController: InputSessionTarget {}

extension TaigiInputController: TpsKeyboardTarget {}
