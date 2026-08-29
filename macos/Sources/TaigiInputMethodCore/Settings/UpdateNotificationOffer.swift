// Asks — once — whether the user wants to hear about new versions.

import AppKit

/// Puts the notification offer to the user, at the one moment it is safe to ask.
///
/// Timing is the whole design here. The system's permission prompt needs this
/// app frontmost to be seen, and activating an `LSUIElement` input method makes
/// the focused client resign — which commits whatever was composing into the
/// user's document (`TaigiInputController.finishComposition`). So the offer is
/// made when the settings window has *already* been brought up by the user:
/// this app is frontmost because they put it there, and nothing is composing.
///
/// The alternative considered and rejected was first launch, which reaches more
/// people: this process first starts when a keystroke wakes it after install,
/// so "first launch" can land mid-word.
///
/// Asked with the question and nothing else. It used to carry a body as well,
/// following `references/MacishType/macos/MacishType/NotificationManager.swift:150-166`
/// — a bare system prompt with no reason attached is the one people decline —
/// but the reason was already the question: 「有新版本的時陣通知你?」 says what
/// arrives and when, and a second line restating it explained nothing. The
/// sentence about nothing typed being sent went with it (USER 2026-08-28):
/// this asks for permission to post a notification, and answering a privacy
/// question nobody asked raises the doubt rather than settling it.
@MainActor
enum UpdateNotificationOffer {
    /// Whether an offer is already on its way to the screen. The stored flag
    /// below cannot answer that on its own: it is written after the first
    /// `await`, so two calls in the same run-loop turn both read "never
    /// offered" and both raise a sheet. "Once ever" is this type's invariant,
    /// so the in-flight half of it is kept here rather than at whichever call
    /// site happens to be able to arrive twice.
    private static var isInFlight = false

    /// Whether the offer still has to be made. Read synchronously at the call
    /// site so an opened settings window does no work at all once it has been.
    static func isPending(in settings: SettingsStore) -> Bool {
        !settings.hasOfferedUpdateNotifications
    }

    /// Makes the offer if it has never been made.
    static func offerIfNeeded(in settings: SettingsStore, parent: NSWindow) async {
        guard !settings.hasOfferedUpdateNotifications, !isInFlight else { return }
        isInFlight = true
        defer { isInFlight = false }
        let reachability = await NotificationManager.shared.reachability()
        // Written after the status read but before the alert: waiting for the
        // answer would re-offer to anyone who dismisses by closing the window,
        // which is the nagging this flag exists to prevent — while writing it
        // before the read would burn the one offer if the window closed during
        // it, having shown nothing.
        settings.hasOfferedUpdateNotifications = true
        // Nothing to offer when the system has already been answered — a user
        // who allowed or refused notifications in System Settings has said so.
        guard reachability == .notAsked else { return }

        let language = DisplayLanguageStore.shared
        let alert = NSAlert()
        alert.messageText = language.string(.desktopUpdateNotifyOfferTitle)
        alert.addButton(withTitle: language.string(.desktopUpdateNotifyOfferEnable))
        alert.addButton(withTitle: language.string(.desktopUpdateLaterAction))

        // A sheet on the settings window, never an app-modal alert: this is
        // only ever reached with that window up, which is the whole reason the
        // offer waits for it.
        guard await alert.beginSheetModal(for: parent) == .alertFirstButtonReturn else { return }
        await NotificationManager.shared.requestAuthorization()
    }
}
