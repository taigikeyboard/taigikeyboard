// Proves a downloaded package is this project's own installer before anything opens it.

import Foundation
import Security

/// What a downloaded package has to prove about itself.
///
/// Derived from the running copy rather than written down as constants: the
/// team that signed the copy asking for an update is the only team whose
/// update it should accept, and the bundle the package installs is the one
/// already running. A constant would be a second home for the signing identity,
/// and it would go stale in silence on the day the certificate is replaced.
struct UpdatePackageIdentity: Equatable, Sendable {
    /// The Apple Developer team, as it appears in the signing certificate's
    /// organizational unit — `257NDUUBD3` for this project's releases.
    let teamIdentifier: String
    /// The bundle the package lays down, which is this input method itself.
    let bundleIdentifier: String
    /// `CFBundleShortVersionString` — the dotted `3.6.6`, not the monotonic
    /// `CFBundleVersion` the Installer compares between releases.
    let shortVersion: String

    /// The identity an update for the running copy would have to carry, or nil
    /// when the running copy cannot name one.
    ///
    /// Nil is the honest answer for a development build: it is signed ad hoc,
    /// so it has no team, and there is nothing for a downloaded package to be
    /// pinned against. In-app installation is unavailable there for the same
    /// reason it is unavailable against a manifest that names no package —
    /// the capability is absent, and the download page still is not.
    static func expected(
        forVersion version: String,
        bundle: Bundle = .main,
        teamIdentifier: String? = runningTeamIdentifier(),
    ) -> UpdatePackageIdentity? {
        guard let teamIdentifier, let bundleIdentifier = bundle.bundleIdentifier else { return nil }
        return UpdatePackageIdentity(
            teamIdentifier: teamIdentifier,
            bundleIdentifier: bundleIdentifier,
            shortVersion: version,
        )
    }

    /// The team the running copy is signed with. Absent on an ad-hoc or
    /// unsigned build.
    ///
    /// Read once for the life of the process, which is as often as the answer
    /// can change. It has to be: validating this bundle's seal takes about 60 ms
    /// (measured against the installed copy, which carries 39 MB of fonts and
    /// the dictionaries), and `UpdateInstallation.offer(for:)` asks for it from
    /// inside a SwiftUI `body` — so paying it per render would put a full code
    /// signature check behind every settings redraw, including the ones a
    /// keystroke causes by writing `inputMode`.
    static func runningTeamIdentifier() -> String? {
        cachedTeamIdentifier
    }

    private static let cachedTeamIdentifier: String? = readRunningTeamIdentifier()

    /// The signature is checked before it is read: `SecCodeCopySigningInformation`
    /// reports what a signature says without deciding whether it is valid, and
    /// a team taken from an unverified signature is not an identity anything
    /// should be pinned against.
    private static func readRunningTeamIdentifier() -> String? {
        var code: SecCode?
        guard SecCodeCopySelf(SecCSFlags(), &code) == errSecSuccess, let code else { return nil }
        var staticCode: SecStaticCode?
        guard SecCodeCopyStaticCode(code, SecCSFlags(), &staticCode) == errSecSuccess,
              let staticCode,
              SecStaticCodeCheckValidity(staticCode, SecCSFlags(), nil) == errSecSuccess
        else { return nil }
        var information: CFDictionary?
        guard SecCodeCopySigningInformation(
            staticCode,
            SecCSFlags(rawValue: kSecCSSigningInformation),
            &information,
        ) == errSecSuccess,
            let signingInformation = information as? [String: Any]
        else { return nil }
        return signingInformation[kSecCodeInfoTeamIdentifier as String] as? String
    }
}

/// Decides whether a downloaded `.pkg` may be handed to Installer.app.
///
/// The manifest that named the package is served unsigned over HTTPS, which was
/// safe for as long as nothing it pointed at was ever executed. Downloading an
/// installer ends that: whoever can rewrite the manifest can name any package,
/// and a package installed here lands in `~/Library/Input Methods`, inside every
/// application the user types in. So the manifest is not what is trusted — the
/// package's own signature is.
///
/// Gatekeeper cannot make that call alone. It accepts any correctly notarized
/// package, including one an attacker notarized under their own Developer ID.
/// Pinning the team is the check it cannot do for us, and it is the reason this
/// type exists.
///
/// `SecAssessment`, the in-process form of `spctl --assess`, is SPI and does not
/// compile in Swift, so Apple's own tools are driven instead. Their exit status
/// is what is read, never their prose: `pkgutil` prints for people, and the
/// wording is not a contract. The identity comes from the archive itself —
/// certificates out of the table of contents, the installed bundle out of the
/// Distribution — both XML, both machine-readable.
enum UpdatePackageVerifier {
    /// Why a package was refused. The caller shows one message for all of them;
    /// the cases exist so a test can say which check fired, and so a debug log
    /// names the failure rather than describing it.
    enum Rejection: Error, Equatable {
        /// Apple's own validation refused it: unsigned, broken, or not notarized.
        case untrusted
        /// Signed and trusted, but not by us.
        case wrongSigner(teamIdentifier: String?)
        /// Signed by us, but not the package the manifest announced.
        case wrongPackage(bundleIdentifier: String?, shortVersion: String?)
        /// The archive would not give up what it declares about itself.
        case unreadable
    }

    /// A `Developer ID Installer` certificate's common name begins with this.
    /// Checked alongside the team so this verification stands on its own rather
    /// than inferring the certificate's type from another tool's exit status.
    private static let installerCommonNamePrefix = "Developer ID Installer:"

    /// Throws unless `package` is a trusted installer, signed by
    /// `expected.teamIdentifier`, declaring exactly the bundle and version
    /// `expected` names.
    ///
    /// Synchronous and blocking — it waits on subprocesses and parses XML, so
    /// it belongs off the main actor.
    static func verify(_ package: URL, is expected: UpdatePackageIdentity) throws {
        // Apple's validation first: it covers the archive's signature and its
        // certificate chain, and — through the checksums that signature covers
        // — that the download arrived intact, which is why no separate digest
        // is carried in the manifest. Notarization is not claimed here; the
        // package is quarantined, so Installer.app assesses that itself when it
        // opens, and that assessment is the system's to make.
        guard let signatureCheck = run("/usr/sbin/pkgutil", ["--check-signature", package.path]),
              signatureCheck.status == 0
        else {
            throw Rejection.untrusted
        }

        // Every signing certificate in the archive, not just the first one
        // found. `pkgutil` reports that *a* signature validated without saying
        // which, so requiring all of them to be ours is what binds its verdict
        // to this identity: a package carrying an attacker's working signature
        // cannot be made to read as ours by adding a block that quotes our
        // certificate, because that block's neighbour would still be theirs.
        for signer in try signingCertificates(of: package) {
            let team = certificateField(signer, kSecOIDOrganizationalUnitName)
            guard let commonName = certificateField(signer, kSecOIDCommonName),
                  commonName.hasPrefix(installerCommonNamePrefix),
                  team == expected.teamIdentifier
            else {
                throw Rejection.wrongSigner(teamIdentifier: team)
            }
        }

        let declared = try declaredBundle(of: package)
        guard declared.bundleIdentifier == expected.bundleIdentifier,
              declared.shortVersion == expected.shortVersion
        else {
            throw Rejection.wrongPackage(
                bundleIdentifier: declared.bundleIdentifier,
                shortVersion: declared.shortVersion,
            )
        }
    }

    // MARK: - The signing certificate

    /// The leaf certificate of every signature the archive carries.
    private static func signingCertificates(of package: URL) throws -> [SecCertificate] {
        guard let toc = run("/usr/bin/xar", ["--dump-toc=-", "-f", package.path]),
              toc.status == 0
        else {
            throw Rejection.unreadable
        }
        let leaves = signingLeafCertificates(inTableOfContents: toc.output)
        let certificates = leaves.compactMap { leaf in
            Data(base64Encoded: leaf, options: .ignoreUnknownCharacters)
                .flatMap { SecCertificateCreateWithData(nil, $0 as CFData) }
        }
        // An archive `pkgutil` accepted has a signature, so finding none means
        // this did not read the archive `pkgutil` read.
        guard !certificates.isEmpty, certificates.count == leaves.count else {
            throw Rejection.unreadable
        }
        return certificates
    }

    /// The leaf certificate of each signature block in a xar table of contents.
    ///
    /// Only certificates inside a `signature`'s own `X509Data` count, and only
    /// the first of each — the leaf, which is what signed the archive. A
    /// `X509Certificate` element anywhere else in the document is not part of
    /// any signature and is ignored; reading the document's first certificate
    /// wherever it sat would let a package carry a decoy and be read as signed
    /// by whoever the decoy names.
    static func signingLeafCertificates(inTableOfContents data: Data) -> [String] {
        let collector = TableOfContentsCertificates()
        let parser = XMLParser(data: data)
        parser.delegate = collector
        guard parser.parse() else { return [] }
        return collector.leaves
    }

    /// One distinguished-name field of a certificate's subject, by OID.
    private static func certificateField(_ certificate: SecCertificate, _ oid: CFString) -> String? {
        guard let values = SecCertificateCopyValues(
            certificate,
            [kSecOIDX509V1SubjectName] as CFArray,
            nil,
        ) as? [String: Any],
            let subject = values[kSecOIDX509V1SubjectName as String] as? [String: Any],
            let fields = subject[kSecPropertyKeyValue as String] as? [[String: Any]]
        else { return nil }
        let field = fields.first { $0[kSecPropertyKeyLabel as String] as? String == oid as String }
        return field?[kSecPropertyKeyValue as String] as? String
    }

    // MARK: - What the package says it installs

    /// The bundle identity a package's `Distribution` declares.
    struct DeclaredBundle: Equatable {
        let bundleIdentifier: String
        let shortVersion: String
    }

    /// Reads the one bundle the package installs.
    ///
    /// Exactly one, because this project ships exactly one: a package that
    /// declares none, or more than one, is not the package this update flow was
    /// built for, and refusing it is how that stays true rather than becoming a
    /// silent guess about which entry to believe.
    private static func declaredBundle(of package: URL) throws -> DeclaredBundle {
        let scratch = FileManager.default.temporaryDirectory
            .appendingPathComponent("update-distribution-\(UUID().uuidString)")
        guard (try? FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)) != nil
        else { throw Rejection.unreadable }
        defer { try? FileManager.default.removeItem(at: scratch) }

        guard let extraction = run(
            "/usr/bin/xar",
            ["-x", "-C", scratch.path, "-f", package.path, "Distribution"],
        ), extraction.status == 0,
        let distribution = try? Data(contentsOf: scratch.appendingPathComponent("Distribution"))
        else { throw Rejection.unreadable }

        guard let only = declaredBundle(inDistribution: distribution) else {
            throw Rejection.unreadable
        }
        return only
    }

    /// The one bundle a `Distribution` says the package installs, or nil when
    /// it names none or more than one.
    static func declaredBundle(inDistribution data: Data) -> DeclaredBundle? {
        let collector = DistributionBundles()
        let parser = XMLParser(data: data)
        parser.delegate = collector
        guard parser.parse(), !collector.sawIncompleteBundle, collector.bundles.count == 1
        else { return nil }
        return collector.bundles.first
    }

    // MARK: - Subprocesses

    /// How long either tool gets. Both answer in milliseconds on a real
    /// package; the bound exists because the input is a file that arrived over
    /// the network, and a malformed archive must not be able to park this work
    /// forever — which would leave the pane stuck on 咧下載… with no way back.
    private static let toolTimeout: TimeInterval = 60

    /// Runs a tool and collects what it wrote, or nil if it could not be
    /// started. Absolute paths and an argument array — never a shell — so
    /// nothing in a path or a URL can be read as syntax.
    private static func run(_ executable: String, _ arguments: [String]) -> (status: Int32, output: Data)? {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        let output = Pipe()
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
        } catch {
            return nil
        }
        // Armed before the read, because the read is the blocking part:
        // terminating the child closes its end of the pipe, which is what ends
        // that read. A timeout checked afterwards would never be reached.
        //
        // A timer source rather than a work item: cancelling a source releases
        // its handler, while a cancelled `DispatchWorkItem` holds what it
        // captured until its deadline passes anyway — which would keep this
        // process and its open pipe alive for a further minute after the work
        // was done, three times over per verification.
        let watchdog = DispatchSource.makeTimerSource(queue: .global())
        watchdog.schedule(deadline: .now() + toolTimeout)
        watchdog.setEventHandler {
            if process.isRunning {
                process.terminate()
            }
        }
        watchdog.resume()
        // Drained before the wait: a table of contents is far larger than the
        // pipe buffer, and a child blocked on a full pipe nobody is reading
        // would never reach the exit this would then wait for forever.
        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        watchdog.cancel()
        return (process.terminationStatus, data)
    }
}

/// Collects the leaf certificate of each signature in a xar table of contents.
///
/// Ancestry-aware on purpose: a certificate only counts as a signer when it
/// sits inside a signature element's `X509Data`, and only the first one there
/// is the leaf. Anything else in the document — including a certificate element
/// planted at the top to be found first — is not what signed the archive.
private final class TableOfContentsCertificates: NSObject, XMLParserDelegate {
    /// One entry per signature block: the certificate that signed it.
    private(set) var leaves: [String] = []

    /// `signature` and `x-signature` — a package carries the classic RSA
    /// signature and the CMS one, and both are signatures whose signer matters.
    private static let signatureElements: Set<String> = ["signature", "x-signature"]

    private var signatureDepth = 0
    private var isInsideCertificateData = false
    private var hasLeafForThisData = false
    private var current: String?

    func parser(
        _: XMLParser,
        didStartElement element: String,
        namespaceURI _: String?,
        qualifiedName _: String?,
        attributes _: [String: String],
    ) {
        if Self.signatureElements.contains(element) {
            signatureDepth += 1
            return
        }
        guard signatureDepth > 0 else { return }
        switch element {
        case "X509Data":
            isInsideCertificateData = true
            hasLeafForThisData = false
        case "X509Certificate" where isInsideCertificateData && !hasLeafForThisData:
            current = ""
        default:
            break
        }
    }

    func parser(_: XMLParser, foundCharacters characters: String) {
        current? += characters
    }

    func parser(
        _: XMLParser,
        didEndElement element: String,
        namespaceURI _: String?,
        qualifiedName _: String?,
    ) {
        if Self.signatureElements.contains(element) {
            signatureDepth = max(0, signatureDepth - 1)
            return
        }
        switch element {
        case "X509Data":
            isInsideCertificateData = false
        case "X509Certificate":
            guard let finished = current else { return }
            leaves.append(finished)
            hasLeafForThisData = true
            current = nil
        default:
            break
        }
    }
}

/// Collects the `<bundle>` entries of a package's Distribution — what it says
/// it installs, and at which version.
private final class DistributionBundles: NSObject, XMLParserDelegate {
    private(set) var bundles: [UpdatePackageVerifier.DeclaredBundle] = []

    /// Whether any `<bundle>` failed to name both an identifier and a short
    /// version. Counted rather than skipped: skipping one would let a second,
    /// half-declared bundle slip past the "exactly one" rule, which is the
    /// whole point of counting.
    private(set) var sawIncompleteBundle = false

    func parser(
        _: XMLParser,
        didStartElement element: String,
        namespaceURI _: String?,
        qualifiedName _: String?,
        attributes: [String: String],
    ) {
        guard element == "bundle" else { return }
        guard let identifier = attributes["id"],
              let shortVersion = attributes["CFBundleShortVersionString"]
        else {
            sawIncompleteBundle = true
            return
        }
        bundles.append(
            UpdatePackageVerifier.DeclaredBundle(
                bundleIdentifier: identifier,
                shortVersion: shortVersion,
            ),
        )
    }
}
