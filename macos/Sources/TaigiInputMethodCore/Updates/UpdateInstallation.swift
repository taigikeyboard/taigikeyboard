// Downloads the published update package and opens it in Installer.app on the user's word.

import AppKit
import Foundation
import os

/// The in-app half of updating: fetches the package the manifest names, proves
/// it is this project's own, and hands it to Installer.app.
///
/// **Two presses, never one.** The download finishes whenever the network says
/// so, and by then the user may be back in their document typing. Launching
/// Installer.app at that moment would take the focus, the input client would
/// resign, and IMK would commit their half-typed word into the document
/// (`TaigiInputController.finishComposition`) — the same hazard that keeps the
/// automatic check to a notification. So arrival never launches anything: it
/// only turns the pane's button into Install, and the second press is the user
/// saying now is a good time. Nothing here is silent, and nothing needs a
/// password only because the package is home-domain (`lib/installer-package.sh`:
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
            case .downloadFailed, .packageRejected, .installerOpenFailed:
                .desktopUpdateInstallFailedNote
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
    /// `reset` could not find it either. Without this, every press of Try Again
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
    /// the disk". Enforced upfront when the server declares `Content-Length`,
    /// and chunk-by-chunk while streaming when it does not or understates it
    /// (matching `desktop/crates/taigi-desktop-update/src/transport.rs`).
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

    enum Failure: LocalizedError, Equatable {
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
        try await download(
            from: url,
            to: destination,
            maximumBytes: maximumPackageBytes,
            configuration: UpdateHTTP.configuration(requestTimeout: 30, resourceTimeout: 20 * 60),
        )
    }

    /// Streams `url` into `destination`, cancelling immediately if the declared
    /// length or received byte count exceeds `maximumBytes`.
    static func download(
        from url: URL,
        to destination: URL,
        maximumBytes: Int64,
        configuration: URLSessionConfiguration,
    ) async throws {
        precondition(maximumBytes >= 0, "maximumBytes must be non-negative")
        let queue = OperationQueue()
        queue.name = "TaigiKeyboard.UpdatePackageDownload"
        queue.maxConcurrentOperationCount = 1

        let delegate = BoundedDownloadDelegate(destination: destination, maximumBytes: maximumBytes)
        let session = URLSession(configuration: configuration, delegate: delegate, delegateQueue: queue)
        // `URLSession` holds a strong reference to its delegate until invalidated;
        // invalidating here releases both once the awaited transfer settles.
        defer { session.invalidateAndCancel() }

        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                delegate.start(url: url, in: session, continuation: continuation)
            }
        } onCancel: {
            delegate.cancel()
        }

        do {
            try markAsDownloaded(destination)
        } catch {
            try? FileManager.default.removeItem(at: destination)
            throw error
        }
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

/// Streams a `URLSessionDataTask` straight into `destination` while enforcing
/// an upper byte limit on both the response header and every incoming chunk.
///
/// File descriptor and filesystem operations stay on the serial delegate
/// `OperationQueue`; `cancel()` only sets a flag and cancels the task so
/// cancellation never races with `FileHandle` writes.
private final class BoundedDownloadDelegate: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    private struct State {
        var continuation: CheckedContinuation<Void, Error>?
        var task: URLSessionDataTask?
        var cancelled = false
        var acceptedResponse = false
        var fileHandle: FileHandle?
        var bytesWritten: Int64 = 0
        var failure: Error?
        var completed = false
    }

    private let destination: URL
    private let maximumBytes: Int64
    private let lock = OSAllocatedUnfairLock(initialState: State())

    init(destination: URL, maximumBytes: Int64) {
        self.destination = destination
        self.maximumBytes = maximumBytes
    }

    func start(
        url: URL,
        in session: URLSession,
        continuation: CheckedContinuation<Void, Error>,
    ) {
        let task = lock.withLockUnchecked { state -> URLSessionDataTask? in
            if state.cancelled {
                state.completed = true
                return nil
            }
            let created = session.dataTask(with: url)
            state.continuation = continuation
            state.task = created
            return created
        }
        guard let task else {
            continuation.resume(throwing: CancellationError())
            return
        }
        task.resume()
    }

    func cancel() {
        let task = lock.withLockUnchecked { state -> URLSessionDataTask? in
            state.cancelled = true
            return state.task
        }
        task?.cancel()
    }

    func urlSession(
        _: URLSession,
        dataTask _: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void,
    ) {
        let earlyAbort = lock.withLockUnchecked { state -> Bool in
            if state.cancelled || state.completed || state.failure != nil {
                return true
            }
            if state.acceptedResponse {
                state.failure = UpdatePackageDownload.Failure.rejected
                return true
            }
            return false
        }
        if earlyAbort {
            completionHandler(.cancel)
            return
        }

        guard UpdateHTTP.isAcceptable(response) else {
            recordFailureAndCancel(UpdatePackageDownload.Failure.rejected, completionHandler: completionHandler)
            return
        }
        if response.expectedContentLength > maximumBytes {
            recordFailureAndCancel(UpdatePackageDownload.Failure.tooLarge, completionHandler: completionHandler)
            return
        }

        let fd = destination.withUnsafeFileSystemRepresentation { path -> Int32 in
            guard let path else {
                errno = EINVAL
                return -1
            }
            return Darwin.open(path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0o600)
        }
        guard fd >= 0 else {
            let openError = POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
            recordFailureAndCancel(openError, completionHandler: completionHandler)
            return
        }
        let handle = FileHandle(fileDescriptor: fd, closeOnDealloc: true)

        let racedWithCancel = lock.withLockUnchecked { state -> Bool in
            guard !state.cancelled, !state.completed, state.failure == nil else {
                return true
            }
            state.acceptedResponse = true
            state.fileHandle = handle
            return false
        }
        if racedWithCancel {
            try? handle.close()
            try? FileManager.default.removeItem(at: destination)
            completionHandler(.cancel)
            return
        }
        completionHandler(.allow)
    }

    func urlSession(_: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        guard !data.isEmpty else { return }
        let chunkLength = Int64(data.count)
        enum ChunkDecision {
            case ignore
            case write(FileHandle)
            case exceedCap(FileHandle?)
        }

        let decision = lock.withLockUnchecked { state -> ChunkDecision in
            guard state.acceptedResponse, !state.cancelled, !state.completed, state.failure == nil,
                  let handle = state.fileHandle
            else {
                return .ignore
            }
            guard chunkLength <= maximumBytes - state.bytesWritten else {
                state.failure = UpdatePackageDownload.Failure.tooLarge
                state.fileHandle = nil
                return .exceedCap(handle)
            }
            state.bytesWritten += chunkLength
            return .write(handle)
        }

        switch decision {
        case .ignore:
            return
        case let .exceedCap(handle):
            try? handle?.close()
            try? FileManager.default.removeItem(at: destination)
            dataTask.cancel()
        case let .write(handle):
            do {
                try handle.write(contentsOf: data)
            } catch {
                let handleToClose = lock.withLockUnchecked { state -> FileHandle? in
                    if state.failure == nil {
                        state.failure = error
                    }
                    let current = state.fileHandle
                    state.fileHandle = nil
                    return current
                }
                try? handleToClose?.close()
                try? FileManager.default.removeItem(at: destination)
                dataTask.cancel()
            }
        }
    }

    func urlSession(_: URLSession, task _: URLSessionTask, didCompleteWithError error: Error?) {
        let finished = lock.withLockUnchecked { state -> (
            continuation: CheckedContinuation<Void, Error>,
            handle: FileHandle?,
            failure: Error?,
            cancelled: Bool,
            acceptedResponse: Bool,
        )? in
            guard !state.completed, let continuation = state.continuation else { return nil }
            state.completed = true
            state.continuation = nil
            let handle = state.fileHandle
            state.fileHandle = nil
            return (continuation, handle, state.failure, state.cancelled, state.acceptedResponse)
        }
        guard let finished else { return }

        if let failure = finished.failure {
            cleanup(finished.handle)
            finished.continuation.resume(throwing: failure)
        } else if finished.cancelled {
            cleanup(finished.handle)
            finished.continuation.resume(throwing: CancellationError())
        } else if let error {
            cleanup(finished.handle)
            finished.continuation.resume(throwing: error)
        } else if !finished.acceptedResponse {
            cleanup(finished.handle)
            finished.continuation.resume(throwing: UpdatePackageDownload.Failure.rejected)
        } else {
            do {
                try finished.handle?.close()
                finished.continuation.resume()
            } catch {
                try? FileManager.default.removeItem(at: destination)
                finished.continuation.resume(throwing: error)
            }
        }
    }

    private func recordFailureAndCancel(
        _ error: Error,
        completionHandler: @escaping @Sendable (URLSession.ResponseDisposition) -> Void,
    ) {
        lock.withLockUnchecked { state in
            if state.failure == nil {
                state.failure = error
            }
        }
        completionHandler(.cancel)
    }

    private func cleanup(_ handle: FileHandle?) {
        try? handle?.close()
        try? FileManager.default.removeItem(at: destination)
    }
}
