// Drops every font-derived cache when Core Text says the registration list moved.

import AppKit
import Combine
import CoreText

/// Watches `kCTFontManagerRegisteredFontsChangedNotification` for the whole
/// process and forgets what was resolved or measured in a face before it.
///
/// Three caches derive from the registration list: the faces `RegisteredFace`
/// resolved, the column floors `CandidateMetrics` measured in them, and the
/// candidate panels whose cells were built in them. A typeface installed or
/// removed in Font Book moves the list without touching any selection value —
/// a family the OS REPLACED under the same name in particular keeps
/// `CandidateFontSelection.installed(family:)` equal to itself, so the metrics
/// comparison in `CandidatePanel.panel(for:)` would go on serving cells set in
/// the old face. All three are dropped together rather than per family: the
/// notification is rare, one panel rebuild on the next show is what it costs,
/// and telling which family moved is more code than rebuilding. A panel that
/// is on screen stays up until then (`CandidatePanel.forgetCachedPanels`).
///
/// Font Book's changes arrive through the distributed centre (user and session
/// scope); this process's own registrations — the bundled roster at launch, a
/// custom font on import — arrive through the local one. Both are observed.
@MainActor
enum FontRegistryObserver {
    static let notificationName = Notification.Name(kCTFontManagerRegisteredFontsChangedNotification as String)

    /// The centres Core Text posts the notification through: the local one for
    /// this process's registrations, the distributed one for Font Book's.
    private static let centers: [NotificationCenter] = [.default, DistributedNotificationCenter.default()]

    /// One notification per change from either centre — what a view that
    /// lists the families re-reads on (`FontManagementPage`).
    static var registrationListMoved: AnyPublisher<Void, Never> {
        Publishers.MergeMany(centers.map { $0.publisher(for: notificationName).map { _ in () } })
            .eraseToAnyPublisher()
    }

    /// Starts observing for the life of the process. Called once, at launch
    /// (`AppDelegate`).
    static func install() {
        for center in centers {
            center.addObserver(forName: notificationName, object: nil, queue: nil) { _ in
                // Deferred to a main-actor turn of its own: Core Text posts the
                // local notification from inside the registration call, and
                // `CustomFontLibrary.remove` is mid-flight then.
                Task { @MainActor in forgetFontDerivedCaches() }
            }
        }
    }

    private static func forgetFontDerivedCaches() {
        RegisteredFace.forgetResolvedFaces()
        CandidateMetrics.forgetPrimaryColumnFloors()
        for panel in CandidatePanel.allInstances {
            panel.forgetCachedPanels()
        }
    }
}
