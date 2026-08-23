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
/// Explained before it is asked, following
/// `references/MacishType/macos/MacishType/NotificationManager.swift:150-166` —
/// a bare system prompt with no reason attached is the one people decline.
@MainActor
enum UpdateNotificationOffer {
    /// Whether the offer still has to be made. Read synchronously at the call
    /// site so an opened settings window does no work at all once it has been.
    static func isPending(in settings: SettingsStore) -> Bool {
        !settings.hasOfferedUpdateNotifications
    }

    /// Makes the offer if it has never been made.
    static func offerIfNeeded(in settings: SettingsStore, parent: NSWindow) async {
        guard !settings.hasOfferedUpdateNotifications else { return }
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
        alert.messageText = language.string(.macosUpdateNotifyOfferTitle)
        alert.informativeText = language.string(.macosUpdateNotifyOfferMessage)
        alert.addButton(withTitle: language.string(.macosUpdateNotifyOfferEnable))
        alert.addButton(withTitle: language.string(.macosUpdateLaterAction))

        // A sheet on the settings window, never an app-modal alert: this is
        // only ever reached with that window up, which is the whole reason the
        // offer waits for it.
        guard await alert.beginSheetModal(for: parent) == .alertFirstButtonReturn else { return }
        await NotificationManager.shared.requestAuthorization()
    }
}
