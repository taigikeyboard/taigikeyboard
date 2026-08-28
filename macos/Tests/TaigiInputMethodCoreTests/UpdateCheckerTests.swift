@testable import TaigiInputMethodCore
import XCTest

/// The notify-only update check: version ordering, manifest validation, and the
/// guards that hold the automatic path to one daily reminder. Every case runs
/// against its own `UserDefaults` suite with an injected manifest and a
/// recording presenter — no network, no AppKit alert.
@MainActor
final class UpdateCheckerTests: XCTestCase {
    /// Force-unwrapped rather than seeded with `UserDefaults.standard`: a
    /// failed `setUp` should stop the test, not quietly write the real domain.
    private var suiteName: String!
    private var userDefaults: UserDefaults!

    /// What each path did. `presented` is the manual alert, `announced` the
    /// automatic notification — a test that means "stayed silent" has to say
    /// which one, because neither implies the other.
    private var presented: [UpdateChecker.Outcome] = []
    private var announced: [UpdateManifest] = []
    private var withdrawCount = 0

    /// What the notification centre is pretending to answer: false is a notice
    /// the user never saw, which must not count as announced.
    private var deliverySucceeds = true

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "UpdateCheckerTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        presented = []
        announced = []
        withdrawCount = 0
        deliverySucceeds = true
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    // MARK: - DottedVersion

    func testDottedVersion_ordersNumericallyPerComponent() {
        // trace: index 2 compares 10 vs 9 numerically — a string compare would
        // call "10" < "9" and invert this.
        XCTAssertTrue(DottedVersion("3.6.9")! < DottedVersion("3.6.10")!)
        // trace: index 1 decides 7 > 6 before the shorter version runs out.
        XCTAssertTrue(DottedVersion("3.6.99")! < DottedVersion("3.7")!)
        XCTAssertFalse(DottedVersion("3.6.5")! < DottedVersion("3.6.5")!)
    }

    func testDottedVersion_trailingZerosAreEqual() {
        // trace: "3.6.5" pads to [3,6,5,0] against [3,6,5,0] — same version.
        XCTAssertEqual(DottedVersion("3.6.5"), DottedVersion("3.6.5.0"))
        XCTAssertFalse(DottedVersion("3.6.5")! < DottedVersion("3.6.5.0")!)
        XCTAssertFalse(DottedVersion("3.6.5.0")! < DottedVersion("3.6.5")!)
    }

    func testDottedVersion_rejectsAnythingButDottedIntegers() {
        for malformed in ["", "3..5", ".6", "3.", "v3.6", "3.6.5-beta", "3.+6", " 3.6", "99999999999"] {
            XCTAssertNil(DottedVersion(malformed), "'\(malformed)' should not parse")
        }
    }

    // MARK: - Manifest decoding

    func testManifestDecode_readsVersionAndPage_ignoringUnknownFields() throws {
        let data = Data("""
        {"version": "3.6.6", "downloadPageURL": "https://example.com/download", "future": 1}
        """.utf8)
        let manifest = try UpdateManifest.decode(data)
        XCTAssertEqual(manifest.version, "3.6.6")
        XCTAssertEqual(manifest.downloadPageURL.absoluteString, "https://example.com/download")
        // Absent is the shape of every manifest published before in-app
        // downloading shipped, and those installs must keep working.
        XCTAssertNil(manifest.packageURL)
    }

    func testManifestDecode_readsPackageURL() throws {
        let data = Data("""
        {"version": "3.6.6", "downloadPageURL": "https://example.com/download",
         "packageURL": "https://example.com/TaigiKeyboard-3.6.6.pkg"}
        """.utf8)
        let manifest = try UpdateManifest.decode(data)
        XCTAssertEqual(
            manifest.packageURL?.absoluteString,
            "https://example.com/TaigiKeyboard-3.6.6.pkg",
        )
    }

    func testManifestDecode_dropsUnusablePackageURL_butKeepsTheManifest() throws {
        // trace: the package is an added convenience on top of a manifest whose
        // required half is intact, so one publishing mistake in it degrades to
        // notify-only rather than silencing update checks for every install.
        // Every shape a mistake can take, including the ones that would make a
        // synthesized decoder throw before the scheme is ever looked at.
        let mistakes = [
            #""packageURL": "http://example.com/TaigiKeyboard-3.6.6.pkg""#,
            #""packageURL": 42"#,
            #""packageURL": {"url": "https://example.com/p.pkg"}"#,
            #""packageURL": ["https://example.com/p.pkg"]"#,
            #""packageURL": null"#,
            #""packageURL": """#,
        ]
        for mistake in mistakes {
            let data = Data("""
            {"version": "3.6.6", "downloadPageURL": "https://example.com/download", \(mistake)}
            """.utf8)
            let manifest = try UpdateManifest.decode(data)
            XCTAssertEqual(manifest.version, "3.6.6", "for \(mistake)")
            XCTAssertEqual(
                manifest.downloadPageURL.absoluteString,
                "https://example.com/download",
                "for \(mistake)",
            )
            XCTAssertNil(manifest.packageURL, "for \(mistake)")
        }
    }

    func testManifestRoundTrip_carriesThePackageURL() throws {
        // The stored pending manifest is read back through the same validation,
        // so the package a remembered update names has to survive the trip.
        let original = UpdateManifest(
            version: "3.6.6",
            downloadPageURL: URL(string: "https://example.com/download")!,
            packageURL: URL(string: "https://example.com/TaigiKeyboard-3.6.6.pkg")!,
        )
        XCTAssertEqual(try UpdateManifest.decode(original.encoded()), original)
    }

    func testManifestDecode_rejectsNonHTTPSAndMalformedVersion() {
        let insecure = Data("""
        {"version": "3.6.6", "downloadPageURL": "http://example.com/download"}
        """.utf8)
        XCTAssertThrowsError(try UpdateManifest.decode(insecure))

        let unparsable = Data("""
        {"version": "3.6.6-beta", "downloadPageURL": "https://example.com/download"}
        """.utf8)
        XCTAssertThrowsError(try UpdateManifest.decode(unparsable))
    }

    // MARK: - Checker guards

    private func makeChecker(
        installed: String = "3.6.5",
        remote: String = "9.9.9",
        fetchFails: Bool = false,
        fetchDelayNanoseconds: UInt64 = 0,
    ) -> UpdateChecker {
        let manifest = UpdateManifest(
            version: remote,
            downloadPageURL: URL(string: "https://example.com/download")!,
            packageURL: nil,
        )
        return UpdateChecker(
            settings: SettingsStore(userDefaults: userDefaults),
            installedVersionText: installed,
            fetchManifest: {
                if fetchDelayNanoseconds > 0 { try await Task.sleep(nanoseconds: fetchDelayNanoseconds) }
                if fetchFails { throw UpdateManifest.ManifestError.malformed }
                return manifest
            },
            announce: { [weak self] manifest in
                self?.announced.append(manifest)
                return self?.deliverySucceeds ?? false
            },
            withdrawAnnouncement: { [weak self] in self?.withdrawCount += 1 },
            presentManualOutcome: { [weak self] outcome in
                self?.presented.append(outcome)
            },
        )
    }

    private func allowNextCheckNow() {
        userDefaults.set(Date.distantPast, forKey: SettingsStore.Keys.updateNextCheckDate.name)
    }

    func testAutoCheck_newerVersion_announcesOnce_neverRepeatsThatVersion() async throws {
        let checker = makeChecker()

        let task = try XCTUnwrap(checker.checkAutomatically())
        await task.value
        XCTAssertEqual(announced.map(\.version), ["9.9.9"])
        // The automatic path never opens an alert — that is the whole point of
        // the notification: no window, no activation, no committed composition.
        XCTAssertTrue(presented.isEmpty)

        // Same day: the check parked the next attempt a day out.
        XCTAssertNil(checker.checkAutomatically())

        // Next day: the same version is old news. Once per version is what
        // makes a fast release train bearable.
        allowNextCheckNow()
        let repeatTask = try XCTUnwrap(checker.checkAutomatically())
        await repeatTask.value
        XCTAssertEqual(announced.count, 1)
    }

    func testAutoCheck_undeliveredAnnouncement_isNotRecordedAsAnnounced() async throws {
        // trace: a notice the system refused (permission not granted yet) must
        // stay announceable, or granting permission later would arrive at a
        // version already marked as told.
        deliverySucceeds = false
        let checker = makeChecker()
        let task = try XCTUnwrap(checker.checkAutomatically())
        await task.value
        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.updateLastNotifiedVersion.name) ?? "",
            "",
        )

        deliverySucceeds = true
        allowNextCheckNow()
        let retry = try XCTUnwrap(checker.checkAutomatically())
        await retry.value
        XCTAssertEqual(announced.count, 2)
        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.updateLastNotifiedVersion.name),
            "9.9.9",
        )
    }

    func testPendingUpdate_isRecordedForTheSettingsPane_andClearedWhenUpToDate() async throws {
        let checker = makeChecker()
        let found = try XCTUnwrap(checker.checkAutomatically())
        await found.value
        XCTAssertEqual(checker.pendingUpdate?.version, "9.9.9")

        // A later check finding the installed copy current retracts both the
        // stored state and the notice still sitting in Notification Centre.
        allowNextCheckNow()
        let current = makeChecker(remote: "3.6.5")
        let settled = try XCTUnwrap(current.checkAutomatically())
        await settled.value
        XCTAssertNil(current.pendingUpdate)
        XCTAssertEqual(withdrawCount, 1)
        XCTAssertNil(userDefaults.data(forKey: SettingsStore.Keys.updatePendingManifest.name))
    }

    func testPendingUpdate_failedCheckKeepsIt() async throws {
        let checker = makeChecker()
        try await XCTUnwrap(checker.checkAutomatically()).value
        allowNextCheckNow()
        // trace: a failed fetch proves nothing about what is published, so the
        // pane must keep showing what the last successful check found.
        let offline = makeChecker(fetchFails: true)
        try await XCTUnwrap(offline.checkAutomatically()).value
        XCTAssertEqual(offline.pendingUpdate?.version, "9.9.9")
    }

    func testPendingUpdate_isDroppedOnceInstalledCatchesUp_andTheNoticeIsWithdrawn() async throws {
        let checker = makeChecker(remote: "3.6.6")
        try await XCTUnwrap(checker.checkAutomatically()).value
        XCTAssertEqual(checker.pendingUpdate?.version, "3.6.6")
        XCTAssertEqual(withdrawCount, 0)

        // trace: same defaults suite, but this build IS 3.6.6 — a user who
        // installed by other means must not be told to install it again, and
        // waiting for the next successful check would leave both the pane and
        // a delivered notice advertising the copy already running.
        let upgraded = makeChecker(installed: "3.6.6")
        XCTAssertNil(upgraded.pendingUpdate)
        XCTAssertNil(userDefaults.data(forKey: SettingsStore.Keys.updatePendingManifest.name))
        XCTAssertEqual(withdrawCount, 1)
    }

    func testFreshInstall_withNothingStored_withdrawsNothing() {
        // trace: no stored manifest means no notice was ever posted; a blanket
        // withdraw on every launch would be a request sent for nothing.
        _ = makeChecker()
        XCTAssertEqual(withdrawCount, 0)
    }

    func testAutoCheck_equivalentVersionSpelling_isNotAnnouncedTwice() async throws {
        // trace: "4.0" and "4.0.0" are one version to `DottedVersion`, which is
        // what decided it was newer; the once-per-version guard has to agree,
        // or a manifest respelling the same release announces it again.
        userDefaults.set("4.0", forKey: SettingsStore.Keys.updateLastNotifiedVersion.name)
        let task = try XCTUnwrap(makeChecker(remote: "4.0.0").checkAutomatically())
        await task.value
        XCTAssertTrue(announced.isEmpty)
    }

    func testAutoCheck_upToDateAndOlderRemote_staySilent() async throws {
        // trace: remote "3.6.5" == installed → upToDate; remote "1.0.0" is a
        // downgrade the strict `>` refuses to announce → upToDate as well.
        for remote in ["3.6.5", "1.0.0"] {
            allowNextCheckNow()
            let task = try XCTUnwrap(makeChecker(remote: remote).checkAutomatically())
            await task.value
        }
        XCTAssertTrue(announced.isEmpty)
    }

    func testAutoCheck_fetchFailure_silent_andWaitsOutTheDay() async throws {
        let before = Date()
        let task = try XCTUnwrap(makeChecker(fetchFails: true).checkAutomatically())
        await task.value
        XCTAssertTrue(announced.isEmpty)
        let nextCheck = try XCTUnwrap(
            userDefaults.object(forKey: SettingsStore.Keys.updateNextCheckDate.name) as? Date,
        )
        // trace: the stamp is written once, before the fetch, as now + 24h for
        // every outcome — a failure must not retry sooner, or the automatic
        // path stops being bounded to one connection a day.
        XCTAssertGreaterThan(nextCheck, before.addingTimeInterval(23 * 60 * 60))
        XCTAssertLessThan(nextCheck, before.addingTimeInterval(25 * 60 * 60))
    }

    func testManualCheck_ignoresThrottleAndAlreadyAnnounced() async throws {
        // Throttled a day out and this version already announced — the button
        // still asks, still answers, and answers in an alert rather than a
        // notification.
        userDefaults.set(Date().addingTimeInterval(9999), forKey: SettingsStore.Keys.updateNextCheckDate.name)
        userDefaults.set("9.9.9", forKey: SettingsStore.Keys.updateLastNotifiedVersion.name)
        let task = try XCTUnwrap(makeChecker().checkManually())
        await task.value
        XCTAssertEqual(presented.count, 1)
        XCTAssertTrue(announced.isEmpty)
    }

    func testManualCheck_reportsUpToDateAndFailure() async throws {
        let upToDateTask = try XCTUnwrap(makeChecker(remote: "3.6.5").checkManually())
        await upToDateTask.value
        let failedTask = try XCTUnwrap(makeChecker(fetchFails: true).checkManually())
        await failedTask.value
        XCTAssertEqual(presented, [.upToDate, .failed])
    }

    func testSecondCheckWhileOneIsInFlight_doesNotStart() async throws {
        let checker = makeChecker(fetchDelayNanoseconds: 100_000_000)
        let first = try XCTUnwrap(checker.checkManually())
        XCTAssertNil(checker.checkManually())
        await first.value
        XCTAssertEqual(presented.count, 1)
    }

    func testManualPressDuringInFlightAutoCheck_deliversThatCheckAsTheManualAnswer() async throws {
        // trace: remote == installed → upToDate, which the auto path would
        // swallow; the manual press that landed mid-flight surfaces it instead.
        let checker = makeChecker(remote: "3.6.5", fetchDelayNanoseconds: 100_000_000)
        let auto = try XCTUnwrap(checker.checkAutomatically())
        XCTAssertNil(checker.checkManually())
        await auto.value
        XCTAssertEqual(presented, [.upToDate])
    }
}
