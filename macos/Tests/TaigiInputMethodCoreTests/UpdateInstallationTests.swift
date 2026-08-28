import os
@testable import TaigiInputMethodCore
import XCTest

/// How the injected seams answer, and what they were asked to do.
///
/// Held apart from the test case because the seams are `@Sendable` closures and
/// an `XCTestCase` is not sendable — this is the sendable thing they capture.
private final class UpdateSeams: @unchecked Sendable {
    struct State {
        var downloadSucceeds = true
        var verificationSucceeds = true
        var launchSucceeds = true
        /// How many downloads have begun. Each one takes the next number, so a
        /// test can wait for one to be genuinely in flight rather than sleeping
        /// and hoping, and can name a particular one below.
        var attemptsBegun = 0
        /// The attempt to park mid-flight, so a test can decide when it lands
        /// relative to the ones started after it.
        var heldAttempt: Int?
        /// How many downloads have finished, whether they wrote a package or
        /// threw. A retired one throws — the directory it was given went with
        /// the staging root — so counting only successes would wait forever.
        var attemptsFinished = 0
        /// The team the running copy is pretending to be signed with. Nil is
        /// the development build, which has none.
        var teamIdentifier: String? = "TEAMID1234"
        var downloadedTo: [URL] = []
        var verified: [URL] = []
        var launched: [URL] = []
    }

    private let lock = NSLock()
    private var state = State()

    func read<Answer>(_ body: (State) -> Answer) -> Answer {
        lock.withLock { body(state) }
    }

    @discardableResult
    func write<Answer>(_ body: (inout State) -> Answer) -> Answer {
        lock.withLock { body(&state) }
    }
}

/// In-app update installation: what the settings pane is offered, how a
/// download moves through its phases, and the two rules that keep the offer
/// honest — a package is never installed unless it verified, and state for one
/// version never answers for another.
///
/// Every case runs against a temporary staging directory with injected
/// download, verification and launch seams: no network, no `pkgutil`, no
/// Installer.app.
@MainActor
final class UpdateInstallationTests: XCTestCase {
    private var stagingRoot: URL!
    private var seams: UpdateSeams!

    private let packagedManifest = UpdateManifest(
        version: "9.9.9",
        downloadPageURL: URL(string: "https://example.com/download")!,
        packageURL: URL(string: "https://example.com/TaigiKeyboard-9.9.9.pkg")!,
    )

    private let pagedManifest = UpdateManifest(
        version: "9.9.9",
        downloadPageURL: URL(string: "https://example.com/download")!,
        packageURL: nil,
    )

    override func setUpWithError() throws {
        try super.setUpWithError()
        stagingRoot = try TestFixtures.scratchDirectory()
        seams = UpdateSeams()
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: stagingRoot)
        super.tearDown()
    }

    private enum SeamFailure: Error {
        case download
    }

    private func makeInstallation() -> UpdateInstallation {
        let stagingRoot = stagingRoot!
        let seams = seams!
        return UpdateInstallation(
            stagePackage: { version in
                let directory = try UserDataDirectory.created(
                    stagingRoot.appendingPathComponent(UUID().uuidString, isDirectory: true),
                )
                return directory.appendingPathComponent("\(version).pkg")
            },
            downloadPackage: { _, destination in
                let attempt = seams.write { state -> Int in
                    state.attemptsBegun += 1
                    return state.attemptsBegun
                }
                defer { seams.write { $0.attemptsFinished += 1 } }
                while seams.read(\.heldAttempt) == attempt {
                    try await Task.sleep(for: .milliseconds(5))
                }
                guard seams.read(\.downloadSucceeds) else { throw SeamFailure.download }
                // A real download leaves a file behind, and the staging
                // lifecycle is about that file existing.
                try Data("pkg".utf8).write(to: destination)
                seams.write { $0.downloadedTo.append(destination) }
            },
            verifyPackage: { package, _ in
                seams.write { $0.verified.append(package) }
                guard seams.read(\.verificationSucceeds) else {
                    throw UpdatePackageVerifier.Rejection.untrusted
                }
            },
            expectedIdentity: { version in
                guard let team = seams.read(\.teamIdentifier) else { return nil }
                return UpdatePackageIdentity(
                    teamIdentifier: team,
                    bundleIdentifier: "com.example.TaigiKeyboard",
                    shortVersion: version,
                )
            },
            launchInstaller: { package in
                seams.write { $0.launched.append(package) }
                return seams.read(\.launchSucceeds)
            },
        )
    }

    /// Runs the download the pane's button starts and waits for it to settle.
    private func download(
        _ manifest: UpdateManifest,
        with installation: UpdateInstallation,
    ) async throws {
        installation.startDownload(for: manifest)
        try await wait(for: "the download to settle") {
            installation.offer(for: manifest) != .downloading
        }
    }

    /// Polls until `condition` holds. The class hands out no task, so an
    /// outcome is observed rather than awaited.
    private func wait(
        for description: String,
        until condition: () -> Bool,
    ) async throws {
        for _ in 0 ..< 400 {
            if condition() {
                return
            }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTFail("timed out waiting for \(description)")
    }

    // MARK: - What the pane is offered

    func testOffer_withoutAPackageURL_isTheDownloadPage() {
        // trace: every manifest published before in-app downloading shipped has
        // no packageURL, and those installs must keep being told where to go.
        XCTAssertEqual(makeInstallation().offer(for: pagedManifest), .downloadPage)
    }

    func testOffer_withoutASigningTeam_isTheDownloadPage() {
        // trace: a development build is signed ad hoc, so there is no team a
        // downloaded package could be pinned against. Refusing to fetch one is
        // the same answer as having nothing to fetch.
        seams.write { $0.teamIdentifier = nil }
        XCTAssertEqual(makeInstallation().offer(for: packagedManifest), .downloadPage)
    }

    func testOffer_withPackageAndTeam_startsAtDownload() {
        XCTAssertEqual(makeInstallation().offer(for: packagedManifest), .startDownload)
    }

    // MARK: - The download

    func testDownload_verifiedPackage_becomesInstallableAndOpensOnRequest() async throws {
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)

        XCTAssertEqual(seams.read(\.verified).count, 1)
        // Staged under the injected directory rather than anywhere the caller
        // chose, and named for the version the user is being offered.
        let staged = try XCTUnwrap(seams.read(\.downloadedTo).first)
        XCTAssertTrue(staged.path.hasPrefix(stagingRoot.path))
        XCTAssertEqual(staged.lastPathComponent, "9.9.9.pkg")
        // The offer carries the package it is offering, so no separate stored
        // URL can disagree with the state about what would be opened.
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(staged))

        // Arrival launched nothing: the second press is what opens Installer.
        XCTAssertTrue(seams.read(\.launched).isEmpty)
        installation.install()
        XCTAssertEqual(seams.read(\.launched), [staged])
        // Installer.app can be cancelled, so the package stays offered.
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(staged))
    }

    func testDownload_failedTransfer_offersARetry() async throws {
        seams.write { $0.downloadSucceeds = false }
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)

        XCTAssertEqual(installation.offer(for: packagedManifest), .downloadFailed)
        XCTAssertTrue(seams.read(\.verified).isEmpty)
        // Nothing to install, so pressing install must do nothing at all.
        installation.install()
        XCTAssertTrue(seams.read(\.launched).isEmpty)

        seams.write { $0.downloadSucceeds = true }
        try await download(packagedManifest, with: installation)
        let staged = try XCTUnwrap(seams.read(\.downloadedTo).first)
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(staged))
    }

    func testDownload_unverifiedPackage_isNeverInstallable() async throws {
        // trace: verification is the only thing standing between a rewritten
        // manifest and code landing in ~/Library/Input Methods, so a refusal
        // must leave nothing stageable behind.
        seams.write { $0.verificationSucceeds = false }
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)

        XCTAssertEqual(installation.offer(for: packagedManifest), .packageRejected)
        installation.install()
        XCTAssertTrue(seams.read(\.launched).isEmpty)
    }

    func testInstall_whenTheInstallerWillNotOpen_staysRetryable() async throws {
        seams.write { $0.launchSucceeds = false }
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)
        let staged = try XCTUnwrap(seams.read(\.downloadedTo).first)
        installation.install()

        XCTAssertEqual(installation.offer(for: packagedManifest), .installerOpenFailed(staged))
        // The package is verified and still staged, so the retry is a real one.
        seams.write { $0.launchSucceeds = true }
        installation.install()
        XCTAssertEqual(seams.read(\.launched).count, 2)
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(staged))
    }

    // MARK: - The state belongs to one version

    func testStagedPackage_doesNotAnswerForADifferentVersion() async throws {
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)
        let staged = try XCTUnwrap(seams.read(\.downloadedTo).first)
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(staged))

        // trace: a later check moved the pending version on. The package staged
        // for 9.9.9 must not be offered as an install of 9.9.10, which is what
        // an unscoped install state would do.
        let newer = UpdateManifest(
            version: "9.9.10",
            downloadPageURL: packagedManifest.downloadPageURL,
            packageURL: packagedManifest.packageURL,
        )
        XCTAssertEqual(installation.offer(for: newer), .startDownload)
    }

    func testSupersededDownload_cannotStageOverTheOneThatReplacedIt() async throws {
        // trace: 9.9.9 → 9.9.10 → 9.9.9 again, with the first download parked
        // until the other two have finished. By the time it lands, its own
        // version is current again — so a guard that compared versions rather
        // than identifying the download would accept it and overwrite the
        // package the third one staged.
        let newer = UpdateManifest(
            version: "9.9.10",
            downloadPageURL: packagedManifest.downloadPageURL,
            packageURL: packagedManifest.packageURL,
        )
        let installation = makeInstallation()

        seams.write { $0.heldAttempt = 1 }
        installation.startDownload(for: packagedManifest)
        try await wait(for: "the first download to park") {
            self.seams.read(\.attemptsBegun) == 1
        }

        try await download(newer, with: installation)
        try await download(packagedManifest, with: installation)
        let current = try XCTUnwrap(seams.read(\.downloadedTo).last)
        XCTAssertEqual(installation.offer(for: packagedManifest), .install(current))

        seams.write { $0.heldAttempt = nil }
        try await wait(for: "the parked download to land") {
            self.seams.read(\.attemptsFinished) == 3
        }
        // Slack for the landing to reach the main actor, where a stale report
        // would take effect if one were accepted.
        try await Task.sleep(for: .milliseconds(50))

        XCTAssertEqual(installation.offer(for: packagedManifest), .install(current))
        installation.install()
        XCTAssertEqual(seams.read(\.launched), [current])
    }

    func testDiscardDuringADownload_resetsAtOnceAndTheLandingChangesNothing() async throws {
        // trace: this is what a launch-time cleanup does to a download the user
        // has already started. The row must go back to offering one, not sit on
        // 咧下載… forever, and the retired download must not resurrect itself.
        let installation = makeInstallation()
        seams.write { $0.heldAttempt = 1 }
        installation.startDownload(for: packagedManifest)
        try await wait(for: "the download to park") { self.seams.read(\.attemptsBegun) == 1 }
        XCTAssertEqual(installation.offer(for: packagedManifest), .downloading)

        installation.discardStagedPackage()
        XCTAssertEqual(installation.offer(for: packagedManifest), .startDownload)

        seams.write { $0.heldAttempt = nil }
        try await wait(for: "the retired download to land") {
            self.seams.read(\.attemptsFinished) == 1
        }
        try await Task.sleep(for: .milliseconds(50))
        XCTAssertEqual(installation.offer(for: packagedManifest), .startDownload)
        installation.install()
        XCTAssertTrue(seams.read(\.launched).isEmpty)
    }

    func testFailedAttempts_leaveNothingOnDisk() async throws {
        // trace: three presses of 閣試一擺, three staging directories. A failure
        // that only changed the state would strand each one — `packageRejected`
        // carries no URL, so nothing afterwards knows where to look — and the
        // rejected package is a whole download, not an empty directory.
        seams.write { $0.verificationSucceeds = false }
        let installation = makeInstallation()
        for _ in 0 ..< 3 {
            try await download(packagedManifest, with: installation)
        }
        XCTAssertEqual(installation.offer(for: packagedManifest), .packageRejected)
        XCTAssertEqual(seams.read(\.downloadedTo).count, 3)

        let leftovers = try FileManager.default.contentsOfDirectory(atPath: stagingRoot.path)
        XCTAssertEqual(leftovers, [], "failed attempts left \(leftovers) behind")
    }

    func testDiscard_removesTheStagedPackageAndResets() async throws {
        let installation = makeInstallation()
        try await download(packagedManifest, with: installation)
        let staged = try XCTUnwrap(seams.read(\.downloadedTo).first)
        XCTAssertTrue(FileManager.default.fileExists(atPath: staged.path))

        installation.discardStagedPackage()
        XCTAssertEqual(installation.offer(for: packagedManifest), .startDownload)
        XCTAssertFalse(FileManager.default.fileExists(atPath: staged.path))
    }
}

/// What a package has to prove about itself, and the two archive readers that
/// find out. The subprocess halves need a signed package and are covered by
/// dogfood; these are the parts that decide what the parsed answer means.
final class UpdatePackageVerifierTests: XCTestCase {
    // MARK: - The identity an update has to match

    func testExpectedIdentity_withoutATeam_isAbsent() {
        // trace: an ad-hoc development build has no TeamIdentifier, so there is
        // nothing to pin a download against and no identity to hand out.
        XCTAssertNil(
            UpdatePackageIdentity.expected(forVersion: "3.6.6", teamIdentifier: nil),
        )
    }

    func testExpectedIdentity_takesTheVersionFromTheManifestAndTheRestFromTheBundle() throws {
        let identity = try XCTUnwrap(
            UpdatePackageIdentity.expected(forVersion: "3.6.6", teamIdentifier: "257NDUUBD3"),
        )
        XCTAssertEqual(identity.teamIdentifier, "257NDUUBD3")
        XCTAssertEqual(identity.shortVersion, "3.6.6")
        XCTAssertEqual(identity.bundleIdentifier, Bundle.main.bundleIdentifier)
    }

    // MARK: - Reading the archive

    /// The shape `productbuild` writes, trimmed to the elements that are read.
    private let distribution = Data("""
    <?xml version="1.0" encoding="utf-8" standalone="yes"?>
    <installer-gui-script minSpecVersion="2">
        <pkg-ref id="com.siansiansu.inputmethod.TaigiKeyboard" version="30605" auth="none">#component.pkg</pkg-ref>
        <pkg-ref id="com.siansiansu.inputmethod.TaigiKeyboard">
            <bundle-version>
                <bundle CFBundleShortVersionString="3.6.5" CFBundleVersion="30605"
                        id="com.siansiansu.inputmethod.TaigiKeyboard" path="TaigiKeyboard.app"/>
            </bundle-version>
        </pkg-ref>
    </installer-gui-script>
    """.utf8)

    func testDeclaredBundle_readsTheShortVersion_notTheMonotonicOne() throws {
        // trace: `pkg-ref version` is 30605, the CFBundleVersion the Installer
        // orders releases by; the manifest announces 3.6.5. Comparing the
        // manifest against the wrong one of those would refuse every package.
        let declared = try XCTUnwrap(
            UpdatePackageVerifier.declaredBundle(inDistribution: distribution),
        )
        XCTAssertEqual(declared.bundleIdentifier, "com.siansiansu.inputmethod.TaigiKeyboard")
        XCTAssertEqual(declared.shortVersion, "3.6.5")
    }

    func testDeclaredBundle_refusesAPackageThatInstallsMoreThanOneBundle() {
        let twoBundles = Data("""
        <installer-gui-script>
            <bundle CFBundleShortVersionString="3.6.5" id="com.example.one"/>
            <bundle CFBundleShortVersionString="3.6.5" id="com.example.two"/>
        </installer-gui-script>
        """.utf8)
        // This project ships exactly one bundle, so anything else is not the
        // package this flow was built for — refusing beats guessing which entry
        // to believe.
        XCTAssertNil(UpdatePackageVerifier.declaredBundle(inDistribution: twoBundles))
    }

    func testDeclaredBundle_refusesASecondBundleThatNamesNothing() {
        // trace: skipping an entry that lacks the attributes would leave one
        // complete bundle behind and count it as the only one — the "exactly
        // one" rule passing on a package that installs two.
        let oneCompleteOneNot = Data("""
        <installer-gui-script>
            <bundle CFBundleShortVersionString="3.6.5" id="com.example.one"/>
            <bundle path="Other.app"/>
        </installer-gui-script>
        """.utf8)
        XCTAssertNil(UpdatePackageVerifier.declaredBundle(inDistribution: oneCompleteOneNot))
    }

    func testDeclaredBundle_refusesJunk() {
        XCTAssertNil(UpdatePackageVerifier.declaredBundle(inDistribution: Data("not xml".utf8)))
    }

    func testSigningLeaf_isTheFirstCertificateOfTheSignature_notTheWholeChain() {
        let tableOfContents = Data("""
        <xar><toc><signature style="RSA"><KeyInfo><X509Data>
            <X509Certificate>LEAF</X509Certificate>
            <X509Certificate>INTERMEDIATE</X509Certificate>
            <X509Certificate>ROOT</X509Certificate>
        </X509Data></KeyInfo></signature></toc></xar>
        """.utf8)
        // The leaf is what signed the archive and what the team is read from;
        // the rest of the chain names Apple.
        XCTAssertEqual(
            UpdatePackageVerifier.signingLeafCertificates(inTableOfContents: tableOfContents),
            ["LEAF"],
        )
    }

    func testSigningLeaf_ignoresACertificatePlantedOutsideAnySignature() {
        // trace: this is the attack the ancestry check exists for. `pkgutil`
        // reports that *a* signature validated; if this read the document's
        // first certificate wherever it sat, a package validly signed by an
        // attacker could carry our certificate at the top and be read as ours.
        let tableOfContents = Data("""
        <xar><toc>
            <file><X509Certificate>DECOY</X509Certificate></file>
            <signature style="RSA"><KeyInfo><X509Data>
                <X509Certificate>ATTACKER</X509Certificate>
            </X509Data></KeyInfo></signature>
        </toc></xar>
        """.utf8)
        XCTAssertEqual(
            UpdatePackageVerifier.signingLeafCertificates(inTableOfContents: tableOfContents),
            ["ATTACKER"],
        )
    }

    func testSigningLeaf_reportsEverySignatureBlock() {
        // A package carries both the classic RSA signature and the CMS one.
        // Both are returned so the caller can require every signer to be ours —
        // one matching block is not evidence about the block beside it.
        let tableOfContents = Data("""
        <xar><toc>
            <signature style="RSA"><KeyInfo><X509Data>
                <X509Certificate>OURS</X509Certificate>
                <X509Certificate>APPLE</X509Certificate>
            </X509Data></KeyInfo></signature>
            <x-signature style="CMS"><KeyInfo><X509Data>
                <X509Certificate>THEIRS</X509Certificate>
            </X509Data></KeyInfo></x-signature>
        </toc></xar>
        """.utf8)
        XCTAssertEqual(
            UpdatePackageVerifier.signingLeafCertificates(inTableOfContents: tableOfContents),
            ["OURS", "THEIRS"],
        )
    }

    func testSigningLeaf_ofAnUnsignedArchiveIsEmpty() {
        let tableOfContents = Data("<xar><toc><files/></toc></xar>".utf8)
        XCTAssertTrue(
            UpdatePackageVerifier.signingLeafCertificates(inTableOfContents: tableOfContents).isEmpty,
        )
    }
}
