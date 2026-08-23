// Checks the published release manifest for a newer version and tells the user — notify-only, never installs.

import AppKit
import Foundation

/// The published release manifest: the newest downloadable version and the page
/// it is downloaded from. Served as a static JSON file, so publishing a release
/// is editing one committed file — no feed generator, no signing key. Safe to be
/// unsigned because nothing here is executed: the checker only compares the
/// version and opens the page in the browser.
struct UpdateManifest: Equatable {
    let version: String
    let downloadPageURL: URL

    enum ManifestError: Error {
        case malformed
    }

    /// The wire shape, used in both directions: what the network serves, and
    /// what a remembered pending update is stored as. One shape means one
    /// validation path — a manifest read back from `UserDefaults` is checked
    /// exactly as strictly as one that arrived over HTTPS.
    private struct Wire: Codable {
        let version: String
        let downloadPageURL: URL
    }

    /// Decodes and validates in one step, so no caller can hold a manifest the
    /// checker would refuse to act on. Unknown fields are ignored — the wire
    /// format may grow, old installs must keep reading it.
    static func decode(_ data: Data) throws -> UpdateManifest {
        guard let wire = try? JSONDecoder().decode(Wire.self, from: data),
              DottedVersion(wire.version) != nil,
              wire.downloadPageURL.scheme?.lowercased() == "https"
        else {
            throw ManifestError.malformed
        }
        return UpdateManifest(version: wire.version, downloadPageURL: wire.downloadPageURL)
    }

    func encoded() throws -> Data {
        try JSONEncoder().encode(Wire(version: version, downloadPageURL: downloadPageURL))
    }

    /// Where the manifest is published. Served from the repository until the
    /// download website exists; when it does, this constant and the manifest's
    /// own `downloadPageURL` move there together.
    static let publishedURL =
        URL(string: "https://raw.githubusercontent.com/taigikeyboard/taigikeyboard/main/macos/updates/latest.json")!

    /// One bounded GET of the published manifest, off the main actor — the read
    /// and the decode belong on the cooperative pool, not interleaved with key
    /// handling. Ephemeral so nothing about the check persists; both timeouts
    /// set because `timeoutIntervalForRequest` alone only bounds the wait for
    /// the next byte, not the whole transfer.
    static func fetchPublished() async throws -> UpdateManifest {
        let maximumBytes = 64 * 1024
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 5
        configuration.timeoutIntervalForResource = 15
        let session = URLSession(configuration: configuration)
        defer { session.finishTasksAndInvalidate() }
        let (bytes, response) = try await session.bytes(from: publishedURL)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200,
              response.url?.scheme?.lowercased() == "https",
              // The declared length rejects an oversized body before a byte of
              // it is read; the loop below is the backstop for a server that
              // declares nothing or lies.
              http.expectedContentLength <= Int64(maximumBytes)
        else {
            throw ManifestError.malformed
        }
        var data = Data()
        for try await byte in bytes {
            data.append(byte)
            guard data.count <= maximumBytes else { throw ManifestError.malformed }
        }
        return try decode(data)
    }
}

/// A dotted numeric version ("3.6.5"), ordered numerically per component so
/// "3.6.10" outranks "3.6.9", with trailing zeros equal ("3.6.5" == "3.6.5.0").
///
/// Deliberately rejects anything else — suffixes, signs, empty segments,
/// oversized components: the remote half of a comparison is network data, and a
/// version this type cannot parse must read as "no update" rather than as
/// whatever a lenient parse would invent.
struct DottedVersion: Comparable {
    let components: [Int]

    /// Bounds on network-supplied text: nothing this project ships comes near
    /// them, so exceeding one reads as garbage, not as a big release.
    private static let maximumComponentCount = 8
    private static let maximumComponentValue = 1_000_000_000

    init?(_ text: String) {
        let segments = text.split(separator: ".", omittingEmptySubsequences: false)
        guard segments.count <= Self.maximumComponentCount else { return nil }
        var parsed: [Int] = []
        for segment in segments {
            // `allSatisfy` on top of `Int.init` because the latter accepts a
            // leading sign ("+6"); the former alone accepts digits Int rejects.
            // An empty segment fails at `Int.init`.
            guard segment.allSatisfy(\.isNumber),
                  let value = Int(segment), value <= Self.maximumComponentValue
            else { return nil }
            parsed.append(value)
        }
        components = parsed
    }

    static func < (lhs: DottedVersion, rhs: DottedVersion) -> Bool {
        for index in 0 ..< max(lhs.components.count, rhs.components.count) {
            let left = index < lhs.components.count ? lhs.components[index] : 0
            let right = index < rhs.components.count ? rhs.components[index] : 0
            if left != right { return left < right }
        }
        return false
    }

    /// Padded like `<`, so "3.6.5" and "3.6.5.0" are the same version — not
    /// the synthesized array equality, which would call them different.
    static func == (lhs: DottedVersion, rhs: DottedVersion) -> Bool {
        !(lhs < rhs) && !(rhs < lhs)
    }
}

/// Fetches the manifest, decides whether the user should hear about it, and
/// hands the announcement to an injected seam.
///
/// Process-wide singleton on purpose: the daily schedule, the in-flight guard,
/// and the pending-update state are one process-wide fact however many sessions
/// and settings windows are alive.
///
/// The automatic path is always on (USER 2026-08-23: no toggle), connects at
/// most once a day — the stamp is written before the fetch, so failures wait
/// out the same day rather than retrying — and its only visible effect is a
/// system notification, once per version. It never opens a window and never
/// activates this app, which is what keeps a composition in the user's document
/// out of reach (`TaigiInputController.finishComposition`).
///
/// The manual path — the settings window's 檢查更新 button — reports every
/// outcome in an alert. Safe there and nowhere else: the settings window is
/// already frontmost, so nothing is being composed.
@Observable
@MainActor
final class UpdateChecker {
    static let shared = UpdateChecker()

    /// How often the automatic path may connect. Read by `AppDelegate`, whose
    /// timer runs at this same cadence — the stamp remains the guard across
    /// relaunches and timer drift.
    static let checkInterval: TimeInterval = 24 * 60 * 60

    /// What one finished check found.
    enum Outcome: Equatable {
        case updateAvailable(UpdateManifest)
        case upToDate
        case failed
    }

    private static let logger = DebugLogger(category: "UpdateChecker")

    private let settings: SettingsStore
    private let installedVersionText: String
    private let fetchManifest: @Sendable () async throws -> UpdateManifest
    private let announce: @MainActor (UpdateManifest) async -> Bool
    private let withdrawAnnouncement: @MainActor () -> Void
    private let presentManualOutcome: @MainActor (Outcome, _ installedVersion: String) -> Void

    private var isCheckInFlight = false

    /// A manual press that landed while a check was already running: the outcome
    /// of that running check is then delivered as the manual answer, instead of
    /// the press silently doing nothing.
    private var isManualOutcomeWanted = false

    /// The newer version this install has been told about, or nil. Observed by
    /// the settings pane, which is the surface that stays accurate with no
    /// network and after a missed notification.
    private(set) var pendingUpdate: UpdateManifest?

    /// Every collaborator is injectable so a test runs against its own defaults
    /// suite, a canned manifest, and a recording announcer; production runs on
    /// the defaults.
    init(
        settings: SettingsStore = SettingsStore(),
        installedVersionText: String = AppVersion.installed,
        fetchManifest: @escaping @Sendable () async throws -> UpdateManifest = UpdateManifest.fetchPublished,
        announce: @escaping @MainActor (UpdateManifest) async -> Bool = UpdateAnnouncement.post,
        withdrawAnnouncement: @escaping @MainActor () -> Void = UpdateAnnouncement.withdraw,
        presentManualOutcome: @escaping @MainActor (Outcome, String) -> Void = UpdateAlertPresenter.present,
    ) {
        self.settings = settings
        self.installedVersionText = installedVersionText
        self.fetchManifest = fetchManifest
        self.announce = announce
        self.withdrawAnnouncement = withdrawAnnouncement
        self.presentManualOutcome = presentManualOutcome
        let storedManifest = settings.updatePendingManifest
        pendingUpdate = Self.validPendingUpdate(storedManifest, installedVersion: installedVersionText)
        // A pending update the user has since installed by other means is stale
        // the moment this process starts under the new version — waiting for
        // the next successful check would leave the settings pane advertising
        // an update to the copy already running, and would leave a delivered
        // notice sitting in Notification Centre for as long as the machine is
        // offline. Both go together, because they are one claim.
        if storedManifest != nil, pendingUpdate == nil {
            clearPendingUpdate()
            withdrawAnnouncement()
        }
    }

    /// The stored pending update, dropped unless it is still newer than what is
    /// installed. Reading and validating together, so no caller can act on a
    /// remembered version that this build has already caught up with.
    private static func validPendingUpdate(
        _ data: Data?,
        installedVersion: String,
    ) -> UpdateManifest? {
        guard let data,
              let manifest = try? UpdateManifest.decode(data),
              let remote = DottedVersion(manifest.version),
              let installed = DottedVersion(installedVersion),
              remote > installed
        else { return nil }
        return manifest
    }

    /// The scheduled entry point (`AppDelegate`'s launch check and daily
    /// timer). Reads one setting and at most schedules a background fetch;
    /// it never blocks and never presents from here.
    @discardableResult
    func checkAutomatically() -> Task<Void, Never>? {
        guard Date() >= settings.updateNextCheckDate else { return nil }
        return startCheck(isManualCheck: false)
    }

    /// The settings-window button. No throttle guard: every outcome — update,
    /// up to date, failure — is shown.
    @discardableResult
    func checkManually() -> Task<Void, Never>? {
        startCheck(isManualCheck: true)
    }

    private func startCheck(isManualCheck: Bool) -> Task<Void, Never>? {
        guard !isCheckInFlight else {
            if isManualCheck { isManualOutcomeWanted = true }
            return nil
        }
        isCheckInFlight = true
        // Stamped before the fetch, unconditionally: one interval for every
        // outcome — success, failure, hang — is what holds the automatic path
        // to one connection a day. A faster retry after a failure would buy an
        // hour on a notify-only reminder and cost that bound.
        settings.updateNextCheckDate = Date().addingTimeInterval(Self.checkInterval)
        // Strong capture on purpose: a started check runs to completion and
        // reports, and the task holds this object only until it does.
        return Task {
            let outcome: Outcome
            do {
                outcome = classify(try await fetchManifest())
            } catch {
                Self.logger.debug("manifest fetch failed: \(error.localizedDescription)")
                outcome = .failed
            }
            isCheckInFlight = false
            await deliver(outcome, isManualCheck: isManualCheck)
        }
    }

    private func classify(_ manifest: UpdateManifest) -> Outcome {
        // A bundle version this cannot parse is a build error, but the reader
        // is release code: staying silent beats alerting about every version.
        guard let installed = DottedVersion(installedVersionText),
              let remote = DottedVersion(manifest.version)
        else { return .upToDate }
        return remote > installed ? .updateAvailable(manifest) : .upToDate
    }

    private func deliver(_ outcome: Outcome, isManualCheck: Bool) async {
        switch outcome {
        case let .updateAvailable(manifest):
            recordPendingUpdate(manifest)
        case .upToDate:
            // The installed copy is current, so any notice still sitting in
            // Notification Centre is now false and goes with the state.
            clearPendingUpdate()
            withdrawAnnouncement()
        case .failed:
            // A failed check proves nothing about what is published; whatever
            // was pending stays pending.
            break
        }

        let answersManualPress = isManualCheck || isManualOutcomeWanted
        isManualOutcomeWanted = false
        if answersManualPress {
            presentManualOutcome(outcome, installedVersionText)
            return
        }

        // The automatic path announces a version once. "Once" is what makes a
        // fast release train bearable; the settings pane and Notification
        // Centre are what a missed banner falls back on.
        guard case let .updateAvailable(manifest) = outcome,
              !hasAlreadyAnnounced(manifest.version)
        else { return }
        // Recorded only once the system actually took it: a notice blocked by
        // a permission that is later granted must still be able to arrive.
        if await announce(manifest) {
            settings.updateLastNotifiedVersion = manifest.version
        }
    }

    /// Whether this version has already had its one announcement.
    ///
    /// Compared as versions rather than as strings, so a manifest that starts
    /// spelling the same release "4.0.0" instead of "4.0" does not read as news
    /// — the ordering that decided it was newer treats those two as equal, and
    /// the two comparisons must not disagree.
    private func hasAlreadyAnnounced(_ version: String) -> Bool {
        // `version` always parses: it came from `.updateAvailable`, which
        // `classify` only produces once `DottedVersion` accepted it. So the
        // only unparsable side is a never-written key on a fresh install.
        guard let announced = DottedVersion(settings.updateLastNotifiedVersion),
              let candidate = DottedVersion(version)
        else { return false }
        return candidate == announced
    }

    private func recordPendingUpdate(_ manifest: UpdateManifest) {
        pendingUpdate = manifest
        settings.updatePendingManifest = try? manifest.encoded()
    }

    private func clearPendingUpdate() {
        pendingUpdate = nil
        settings.updatePendingManifest = nil
    }

}

/// The shipped announcement: a system notification, worded from the live
/// display language.
///
/// The feature half of the notification, matching `UpdateAlertPresenter` on the
/// other path: `NotificationManager` knows the framework, this knows which
/// notice this app sends and what it says.
@MainActor
enum UpdateAnnouncement {
    /// One identifier for the whole feature, so a newer release's notice
    /// replaces the one before it. Frequent releases must never pile up, and
    /// the older notice names a download that is no longer the latest.
    private static let identifier = "update-available"

    static func post(_ manifest: UpdateManifest) async -> Bool {
        let language = DisplayLanguageStore.shared
        language.syncFromSettings()
        return await NotificationManager.shared.post(
            identifier: identifier,
            title: language.string(.macosUpdateAvailableTitle),
            body: language.resolver.macosUpdateNotificationBody(version: manifest.version),
            linkURL: manifest.downloadPageURL,
        )
    }

    static func withdraw() {
        NotificationManager.shared.remove(identifier: identifier)
    }
}

/// The manual check's answer: an `NSAlert` per outcome, worded from the live
/// display language. Reached only from the settings window's button, where the
/// window is already frontmost and nothing is being composed — the automatic
/// path never comes here.
@MainActor
enum UpdateAlertPresenter {
    static func present(_ outcome: UpdateChecker.Outcome, installedVersion: String) {
        let language = DisplayLanguageStore.shared
        language.syncFromSettings()

        switch outcome {
        case let .updateAvailable(manifest):
            let alert = NSAlert()
            alert.messageText = language.string(.macosUpdateAvailableTitle)
            alert.informativeText = language.resolver.macosUpdateAvailableMessage(
                latest: manifest.version,
                current: installedVersion,
            )
            alert.addButton(withTitle: language.string(.macosUpdateDownloadAction))
            alert.addButton(withTitle: language.string(.macosUpdateLaterAction))
            guard alert.runModal() == .alertFirstButtonReturn else { return }
            // Same rule as `ExternalLinkButton`: a button that silently does
            // nothing is indistinguishable from a broken one.
            guard !NSWorkspace.shared.open(manifest.downloadPageURL) else { return }
            inform(
                language.string(.macosOpenURLFailed),
                detail: manifest.downloadPageURL.absoluteString,
                language: language,
            )

        case .upToDate:
            inform(language.string(.macosUpdateUpToDateTitle), language: language)

        case .failed:
            inform(
                language.string(.macosUpdateCheckFailedTitle),
                detail: language.string(.macosUpdateCheckFailedMessage),
                language: language,
            )
        }
    }

    /// One button, nothing to decide — the shape three of the four alerts here
    /// share.
    private static func inform(_ title: String, detail: String? = nil, language: DisplayLanguageStore) {
        let alert = NSAlert()
        alert.messageText = title
        if let detail { alert.informativeText = detail }
        alert.addButton(withTitle: language.string(.commonOk))
        alert.runModal()
    }
}
