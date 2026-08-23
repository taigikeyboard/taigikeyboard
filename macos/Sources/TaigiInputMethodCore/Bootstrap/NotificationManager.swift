// The one place this process talks to the system notification centre.

import AppKit
import UserNotifications

/// Posts notices, and answers whether one would actually reach the user.
///
/// Framework plumbing only: what a notice says, and which one this app has to
/// send, belong to the feature that sends it (`UpdateAnnouncement`).
///
/// Shaped after `references/MacishType/macos/MacishType/NotificationManager.swift`,
/// the one input method in the reference set that posts system notifications:
/// posting reads the authorization state but never asks for it, asking is its
/// own explicit step at a moment the user chose, and a reused identifier
/// replaces the previous notice instead of stacking.
///
/// Why a notification rather than a window: showing a window from an
/// `LSUIElement` agent means activating this app, which makes the focused
/// client resign and IMK tear the session down — committing whatever was
/// composing into the user's document (`TaigiInputController.finishComposition`).
/// A notification activates nothing, so typing is untouched by construction.
/// It also means the system's Focus and Do Not Disturb settings decide whether
/// now is a good moment, which no window of ours could ask.
///
/// Process-wide because `UNUserNotificationCenter.delegate` is weak and must be
/// set before launch finishes (Apple: "Assigning the delegate too late … may
/// result in missing incoming notifications") — a delegate nobody retains is a
/// notification tap that does nothing.
@MainActor
final class NotificationManager: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationManager()

    /// Whether a notice would be seen, as one answer.
    ///
    /// One definition, because two readers ask: the poster, deciding whether to
    /// send, and the settings pane, deciding whether to say notices are off.
    /// Split predicates let the pane stay quiet in exactly the state where
    /// posting always fails — authorized, with alerts turned off afterwards.
    enum Reachability {
        case deliverable
        /// Asked, and the answer is no: refused outright, or alerts turned off
        /// in System Settings afterwards.
        case blocked
        /// The system has never been asked, so nothing is being refused yet.
        case notAsked
    }

    /// Where a tapped notice sends the user. Carried in the notification's own
    /// payload rather than held here: a notice outlives this process, and a
    /// remembered URL could disagree with the one the user is looking at.
    nonisolated static let linkURLKey = "linkURL"

    private static let logger = DebugLogger(category: "Notifications")

    /// The system centre, or nil where there is none to talk to.
    ///
    /// `UNUserNotificationCenter.current()` raises
    /// `NSInternalInconsistencyException` ("bundleProxyForCurrentProcess is
    /// nil") — it aborts the process rather than failing — when the running
    /// binary is not an app bundle. The xctest runner is exactly that, so a
    /// unit test that renders the settings pane would take the whole suite
    /// down. A framework this app cannot reach is a reason to post nothing,
    /// never a reason to crash, so the condition is checked rather than hit.
    private lazy var center: UNUserNotificationCenter? = {
        guard Bundle.main.bundleURL.pathExtension == "app" else { return nil }
        return UNUserNotificationCenter.current()
    }()

    private override init() { super.init() }

    /// Installs the delegate. Called from `applicationDidFinishLaunching`,
    /// which is the deadline Apple documents for it.
    func registerDelegate() {
        center?.delegate = self
    }

    /// One read of the system's settings, answering both questions callers have.
    func reachability() async -> Reachability {
        // Nothing has been asked and nothing is being refused: the honest
        // answer where there is no notification centre at all.
        guard let center else { return .notAsked }
        let settings = await center.notificationSettings()
        switch settings.authorizationStatus {
        case .notDetermined:
            return .notAsked
        case .authorized, .provisional, .ephemeral:
            // Authorization alone is not enough: alerts can be turned off in
            // System Settings while the app stays authorized, and a notice
            // delivered silently to the list is not a reminder.
            return settings.alertSetting == .disabled ? .blocked : .deliverable
        default:
            return .blocked
        }
    }

    /// Asks the system for permission. Only ever called after the user has said
    /// yes to being asked (`UpdateNotificationOffer`), so the system prompt is
    /// never the first they hear of it.
    ///
    /// A thrown request is logged rather than reported: a failure to ask is not
    /// the user declining, and the next `reachability()` read is what any
    /// caller acts on anyway.
    func requestAuthorization() async {
        guard let center else { return }
        do {
            _ = try await center.requestAuthorization(options: [.alert])
        } catch {
            Self.logger.error("authorization request failed: \(error.localizedDescription)")
        }
    }

    /// Posts a notice, replacing any earlier one under the same identifier.
    /// Answers whether the system took it — a caller recording "this was said"
    /// does so only on a true, so a notice that reached nobody can be sent
    /// again once permission arrives.
    func post(identifier: String, title: String, body: String, linkURL: URL) async -> Bool {
        guard let center, await reachability() == .deliverable else { return false }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.userInfo = [Self.linkURLKey: linkURL.absoluteString]
        // Withdrawn explicitly before the new one is added rather than relying
        // on replacement alone: the delivered copy is what sits in Notification
        // Centre, and it names something no longer current.
        remove(identifier: identifier)
        let request = UNNotificationRequest(
            identifier: identifier,
            content: content,
            // Immediate: the caller already knows it has something to say, and
            // a trigger would only decide when to say it.
            trigger: nil,
        )
        do {
            try await center.add(request)
            return true
        } catch {
            Self.logger.error("posting notice '\(identifier)' failed: \(error.localizedDescription)")
            return false
        }
    }

    /// Takes a delivered notice back, for when what it said stops being true.
    /// Delivered only: nothing here posts with a trigger, so there is never a
    /// pending request to cancel.
    func remove(identifier: String) {
        center?.removeDeliveredNotifications(withIdentifiers: [identifier])
    }

    func openSystemNotificationSettings() {
        guard let url = URL(string: "x-apple.systempreferences:com.apple.Notifications-Settings.extension") else {
            return
        }
        NSWorkspace.shared.open(url)
    }

    // MARK: - UNUserNotificationCenterDelegate

    /// A tap opens the link the notice carried. The URL is re-read and
    /// re-validated from the notification's own payload: it crossed a process
    /// boundary, and only `https` is ever opened from here.
    nonisolated func userNotificationCenter(
        _: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
    ) async {
        guard response.actionIdentifier == UNNotificationDefaultActionIdentifier,
              let text = response.notification.request.content.userInfo[Self.linkURLKey] as? String,
              let url = URL(string: text), url.scheme?.lowercased() == "https"
        else { return }
        await MainActor.run { _ = NSWorkspace.shared.open(url) }
    }

    /// Nothing is shown while this app is frontmost. The only time it is, is
    /// when the user has the settings window open — where the same news is
    /// already a row on the page, and a banner restating it reads as the app
    /// talking to itself.
    nonisolated func userNotificationCenter(
        _: UNUserNotificationCenter,
        willPresent _: UNNotification,
    ) async -> UNNotificationPresentationOptions {
        []
    }
}
