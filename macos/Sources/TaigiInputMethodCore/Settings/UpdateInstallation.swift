// Downloads the published update package and opens it in Installer.app on the user's word.

import AppKit
import Foundation

/// The in-app half of updating: fetches the package the manifest names, proves
/// it is this project's own, and hands it to Installer.app.
///
/// **Two presses, never one.** The download finishes whenever the network says
/// so, and by then the user may be back in their document typing. Launching
/// Installer.app at that moment would take the focus, the input client would
/// resign, and IMK would commit their half-typed word into the document
/// (`TaigiInputController.finishComposition`) — the same hazard that keeps the
/// automatic check to a notification. So arrival never launches anything: it
/// only turns the pane's button into 安裝, and the second press is the user
/// saying now is a good time. Nothing here is silent, and nothing needs a
/// password only because the package is home-domain (`release-app.sh`:
/// `enable_currentUserHome`, `auth="none"`) — it installs into
/// `~/Library/Input Methods`, which the user already owns.
///
/// Process-wide singleton for the same reason `UpdateChecker` is one: a staged
/// package is one fact about this process, not about whichever settings window
/// happens to be open, and closing the window mid-download must not throw the
/// download away.
///
/// Every answer is scoped to a version. A package staged for 3.6.6 says nothing
/// about 3.6.7, so once a check moves the pending version on, the old answer
/// reads as "nothing downloaded" rather than offering to install the wrong
/// release.
@Observable
@MainActor
final class UpdateInstallation {
    static let shared = UpdateInstallation()

    /// What the settings pane should offer about an update, and the note that
    /// goes with it.
    ///
    /// One value rather than a state and a separately chosen explanation, for
    /// the reason `UserDataPageChrome` gives for its own: a free control beside
    /// a free note admits a retry button under a "this cannot be trusted"
    /// message. The staged package rides in the two cases that have one, so the
    /// rule "a launch failure keeps the package, every other failure does not"
    /// is in the type rather than in four assignments that must agree.
    enum Offer: Equatable {
        /// This build cannot fetch the package itself — the manifest names none,
        /// or the running copy is signed ad hoc and so has no team to pin one
        /// against. Never stored: it is a fact about the build and the manifest,
        /// not about a download. The page has always worked and still does.
        case downloadPage
        case startDownload
        case downloading
        case install(URL)
        case downloadFailed
        /// It arrived but is not this project's signed installer. Never offer to
        /// install it — the download page is the way out.
        case packageRejected
        case installerOpenFailed(URL)

        /// The verified package this state is holding, if any.
        var stagedPackage: URL? {
            switch self {
            case let .install(package), let .installerOpenFailed(package): package
            default: nil
            }
        }

        /// The second line of the pane's row, shown only when something needs
        /// explaining — which means only the three failures. `downloading` says
        /// what it is with the spinner beside it, and a line of text repeating
        /// that is a line nobody needs to read.
        var noteKey: StringKey? {
            switch self {
            case .downloadFailed: .macosUpdateDownloadFailedNote
            case .packageRejected: .macosUpdatePackageRejectedNote
            case .installerOpenFailed: .macosUpdateInstallerOpenFailedNote
            case .downloadPage, .startDownload, .downloading, .install: nil
            }
        }
    }

    private static let logger = DebugLogger(category: "UpdateInstallation")

    private let stagePackage: @Sendable (_ version: String) throws -> URL
    private let downloadPackage: @Sendable (_ from: URL, _ to: URL) async throws -> Void
    private let verifyPackage: @Sendable (URL, UpdatePackageIdentity) throws -> Void
    private let expectedIdentity: @Sendable (String) -> UpdatePackageIdentity?
    private let launchInstaller: @MainActor (URL) -> Bool

    /// How far the package for `targetVersion` has got.
    private var progress: Offer = .startDownload

    /// The version `progress` is about.
    private var targetVersion: String?

    /// Which download `progress` is about.
    ///
    /// Counted rather than identified by version, because a version repeats:
    /// start 3.6.6, let a check move the target to 3.6.7, then move it back, and
    /// the first download's completion would find its own version current again
    /// and overwrite the state of the download that replaced it.
    private var currentDownload = 0

    /// Every collaborator is injectable so a test runs against its own
    /// directory with a canned package and never touches the network, the
    /// user's caches, or Installer.app.
    init(
        stagePackage: @escaping @Sendable (String) throws -> URL = UpdatePackageDownload.stage(version:),
        downloadPackage: @escaping @Sendable (URL, URL) async throws -> Void = UpdatePackageDownload.run,
        verifyPackage: @escaping @Sendable (URL, UpdatePackageIdentity) throws -> Void = UpdatePackageVerifier.verify,
        expectedIdentity: @escaping @Sendable (String) -> UpdatePackageIdentity? = { UpdatePackageIdentity.expected(forVersion: $0) },
        launchInstaller: @escaping @MainActor (URL) -> Bool = { NSWorkspace.shared.open($0) },
    ) {
        self.stagePackage = stagePackage
        self.downloadPackage = downloadPackage
        self.verifyPackage = verifyPackage
        self.expectedIdentity = expectedIdentity
        self.launchInstaller = launchInstaller
    }

    /// Whether this build can fetch and install `manifest`'s package itself.
    ///
    /// The capability, asked separately from the state, because the two answer
    /// different questions: a sheet deciding what its button means needs this
    /// one, and reading it off `offer(for:)` would make the meaning depend on
    /// whether a download happened to be running.
    func canInstallInApp(_ manifest: UpdateManifest) -> Bool {
        manifest.packageURL != nil && expectedIdentity(manifest.version) != nil
    }

    /// What the pane should offer about `manifest`.
    func offer(for manifest: UpdateManifest) -> Offer {
        guard canInstallInApp(manifest) else { return .downloadPage }
        return targetVersion == manifest.version ? progress : .startDownload
    }

    /// Throws away a staged package, unless it is for `version`.
    ///
    /// Called whenever a check moves the pending version on: a package for a
    /// release nobody is being offered any more is tens of megabytes with no way
    /// back to it, and this process can live for weeks. Naming the version that
    /// is still pending is what stops the daily check finding the same release
    /// again from throwing away the download the user is about to install.
    func discardStagedPackage(otherThan version: String? = nil) {
        if let version, targetVersion == version {
            return
        }
        reset()
    }

    /// Fetches and verifies the package `manifest` names.
    func startDownload(for manifest: UpdateManifest) {
        guard let packageURL = manifest.packageURL,
              let identity = expectedIdentity(manifest.version)
        else { return }
        // Idempotent for the version in hand: pressing again while it is
        // arriving, or while it is already staged, must not fetch it twice.
        switch offer(for: manifest) {
        case .downloading, .install, .installerOpenFailed: return
        case .downloadPage, .startDownload, .downloadFailed, .packageRejected: break
        }

        reset()
        targetVersion = manifest.version
        let download = currentDownload
        progress = .downloading

        let stagePackage = stagePackage
        let downloadPackage = downloadPackage
        let verifyPackage = verifyPackage
        let version = manifest.version
        Task {
            // Resolved before the transfer, and outside the `do` below, because
            // every way that transfer can end has to be able to throw the
            // directory away again.
            let package: URL
            do {
                package = try stagePackage(version)
            } catch {
                Self.logger.debug("staging failed: \(error.localizedDescription)")
                fail(.downloadFailed, for: download, discarding: nil)
                return
            }
            do {
                try await downloadPackage(packageURL, package)
                // Off the main actor: it waits on `pkgutil` and `xar`, and the
                // keyboard must not.
                try await Task.detached { try verifyPackage(package, identity) }.value
                finishDownload(download, at: package)
            } catch let rejection as UpdatePackageVerifier.Rejection {
                Self.logger.debug("package refused: \(String(describing: rejection))")
                fail(.packageRejected, for: download, discarding: package)
            } catch {
                Self.logger.debug("package download failed: \(error.localizedDescription)")
                fail(.downloadFailed, for: download, discarding: package)
            }
        }
    }

    /// Opens the staged package in Installer.app. The user's second press.
    func install() {
        guard let package = progress.stagedPackage else { return }
        // Left installable either way: Installer.app can be cancelled, so the
        // package is still worth opening again — and a launch that answered
        // false must say so rather than look like nothing happened, the same
        // rule `ExternalLinkButton` follows.
        progress = launchInstaller(package) ? .install(package) : .installerOpenFailed(package)
    }

    /// Records an arrival, unless the download it belongs to has since been
    /// replaced or discarded.
    private func finishDownload(_ download: Int, at package: URL) {
        guard download == currentDownload else {
            // Retired while it was arriving. Nothing will ever open it, and the
            // state that knew where it sat has already moved on, so it goes now
            // rather than waiting for the next launch to sweep the directory.
            UpdatePackageDownload.discard(package)
            return
        }
        progress = .install(package)
    }

    /// Records a failure and throws away whatever the attempt left on disk.
    ///
    /// The file goes whether or not this download is still the current one. A
    /// failed attempt is never opened, and nothing afterwards remembers where it
    /// sat — `downloadFailed` and `packageRejected` carry no URL, so a later
    /// `reset` could not find it either. Without this, every press of 閣試一擺
    /// would leave its directory behind for the rest of a run, and this process
    /// lives for weeks.
    private func fail(_ failure: Offer, for download: Int, discarding package: URL?) {
        if let package {
            UpdatePackageDownload.discard(package)
        }
        guard download == currentDownload else { return }
        progress = failure
    }

    /// Forgets everything about the current version and retires whatever is in
    /// flight, so a download that finishes afterwards cannot report into state
    /// it was just cleared from.
    private func reset() {
        if let staged = progress.stagedPackage {
            UpdatePackageDownload.discard(staged)
        }
        progress = .startDownload
        targetVersion = nil
        currentDownload += 1
    }
}

/// Fetching an update package: where it is put, and how it gets there.
///
/// Separate from `UpdateInstallation` because none of it belongs on the main
/// actor — it waits on the network and on the file system, while that class
/// exists to hold the state a SwiftUI view reads. It also owns the on-disk
/// layout whole, so the shape of a staging directory is decided in one file.
enum UpdatePackageDownload {
    /// Largest package this will keep. The 3.6.5 release is 47 MB, so this is
    /// generous room for growth and still far under "a hostile server filled
    /// the disk". It is checked after the transfer rather than during: the CDN
    /// serving our releases sends no `Content-Length` (measured), so there is no
    /// declared size to refuse up front, and `URLSession` reports no progress
    /// for a task driven by the async API — which is also why the pane shows an
    /// indeterminate spinner rather than a percentage.
    static let maximumPackageBytes: Int64 = 200 * 1024 * 1024

    /// `~/Library/Caches/<bundle id>/Updates`.
    ///
    /// Caches, unlike the learning databases in `UserDataDirectory`: a staged
    /// installer is reproducible by downloading it again, so it is exactly what
    /// the system may throw away when it needs the space.
    static func stagingDirectory() throws -> URL {
        try UserDataDirectory.container(in: .cachesDirectory)
            .appendingPathComponent("Updates", isDirectory: true)
    }

    /// Where the package for `version` will be written.
    ///
    /// A fresh, unpredictable subdirectory each time, with the package named
    /// plainly inside it: the directory is what makes the path unguessable,
    /// while the file keeps the name Installer.app puts in front of the user.
    static func stage(version: String) throws -> URL {
        try UserDataDirectory.created(
            stagingDirectory().appendingPathComponent(UUID().uuidString, isDirectory: true),
        )
        .appendingPathComponent("\(version).pkg")
    }

    /// Removes a staged package, and the directory that existed to hold it.
    static func discard(_ package: URL) {
        try? FileManager.default.removeItem(at: package.deletingLastPathComponent())
    }

    /// Removes every staged package. For launch, where anything still here
    /// belongs to a run that has ended: it was installed, in which case this
    /// process is already the new version, or it was not, in which case tens of
    /// megabytes should not sit in the caches waiting for a press that never
    /// came.
    static func removeStagedPackages() {
        guard let directory = try? stagingDirectory() else { return }
        try? FileManager.default.removeItem(at: directory)
    }

    enum Failure: LocalizedError {
        /// The server answered, but not with a package we asked for.
        case rejected
        /// More bytes arrived than `maximumPackageBytes` allows.
        case tooLarge

        var errorDescription: String? {
            switch self {
            case .rejected: "the server did not answer with a package over HTTPS"
            case .tooLarge: "the package is larger than \(maximumPackageBytes) bytes"
            }
        }
    }

    /// Streams the package to `destination`.
    ///
    /// A long resource timeout, because tens of megabytes on a slow line is
    /// normal rather than a hang.
    static func run(from url: URL, to destination: URL) async throws {
        let session = UpdateHTTP.session(requestTimeout: 30, resourceTimeout: 20 * 60)
        defer { session.finishTasksAndInvalidate() }

        let (temporary, response) = try await session.download(from: url)
        guard UpdateHTTP.isAcceptable(response) else {
            try? FileManager.default.removeItem(at: temporary)
            throw Failure.rejected
        }
        // Moved before anything else looks at it: `URLSession` owns that
        // temporary file only until this returns.
        try FileManager.default.moveItem(at: temporary, to: destination)

        let size = try FileManager.default.attributesOfItem(atPath: destination.path)[.size] as? Int64
        guard let size, size <= maximumPackageBytes else {
            try? FileManager.default.removeItem(at: destination)
            throw Failure.tooLarge
        }
        try markAsDownloaded(destination)
    }

    /// Records that the package came from the internet.
    ///
    /// `URLSession` sets no quarantine of its own — that is a Launch Services
    /// behaviour browsers get — so without this the provenance of a file this
    /// app fetched and then asks the system to open would simply be missing.
    /// Written per file rather than by switching on `LSFileQuarantineEnabled`,
    /// which would apply to everything this process ever creates, the learning
    /// databases included.
    private static func markAsDownloaded(_ package: URL) throws {
        var package = package
        var values = URLResourceValues()
        values.quarantineProperties = [
            kLSQuarantineTypeKey as String: kLSQuarantineTypeOtherDownload as String,
            kLSQuarantineAgentNameKey as String: "TaigiKeyboard",
        ]
        try package.setResourceValues(values)
    }
}
