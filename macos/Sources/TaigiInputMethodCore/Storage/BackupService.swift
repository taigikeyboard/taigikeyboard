// The `.taigi` file: everything the user's three databases hold, in one
// document they can carry to another machine.

import Foundation

/// Why a `.taigi` file could not be restored.
enum BackupError: Error, CustomStringConvertible {
    /// Written by a newer version than this build understands. Refused rather
    /// than partially read: a format that gained a field is one thing, a
    /// format that changed the meaning of one is another, and this side cannot
    /// tell which from a version number alone.
    case unsupportedVersion(Int)

    var description: String {
        switch self {
        case let .unsupportedVersion(version): "backup format version \(version) is not supported"
        }
    }
}

/// What restoring one category did.
///
/// Per category, and with the failure kept rather than folded into a count,
/// because the three databases cannot be restored in one transaction: if the
/// frequency import throws after the custom dictionary succeeded, the user has
/// to be told which half landed. iOS reports a plain count and treats a failure
/// as zero rows, which reads as "there was nothing to restore".
enum BackupCategoryOutcome: Hashable, Sendable {
    case restored(Int)
    case failed(String)

    var restoredCount: Int? {
        if case let .restored(count) = self {
            count
        } else {
            nil
        }
    }
}

struct BackupImportResult: Hashable, Sendable {
    let customDictionary: BackupCategoryOutcome
    let frequency: BackupCategoryOutcome
    let association: BackupCategoryOutcome

    var hasFailure: Bool {
        [customDictionary, frequency, association].contains { outcome in
            if case .failed = outcome {
                true
            } else {
                false
            }
        }
    }
}

/// Writes and reads the `.taigi` document.
///
/// The schema is version 2 and is shared with iOS and Android
/// (`ios/.../Lexicon/Services/BackupService.swift:34-70`), which is the whole
/// point of the file: it is the only way user data moves between the three
/// platforms, and macOS has no other cross-device path at all.
struct BackupService: Sendable {
    /// CROSS-PLATFORM INVARIANT — the schema version iOS and Android write.
    static let currentVersion = 2
    /// Version 1 lacked the per-reading `tl` columns; its rows restore into the
    /// tolerant empty-reading bucket. Anything above `currentVersion` is a
    /// format this build has not seen.
    static let supportedVersions = 1 ... currentVersion

    private let stores: UserDataStores
    private let appVersion: String

    init(stores: UserDataStores, appVersion: String = BackupService.bundleVersion()) {
        self.stores = stores
        self.appVersion = appVersion
    }

    // MARK: - Export

    func export() async throws -> Data {
        let document = try await BackupDocument(
            version: Self.currentVersion,
            exportedAt: Self.timestampFormatter.string(from: Date()),
            platform: "macos",
            appVersion: appVersion,
            customDictionary: stores.customDictionary.allRows().map {
                BackupDocument.CustomEntry(roman: $0.roman, hanzi: $0.hanzi)
            },
            userFrequency: stores.frequency.allRows().map {
                BackupDocument.FrequencyEntry(
                    word: $0.word,
                    tl: $0.tl,
                    count: $0.count,
                    // Empty, as iOS and Android write it. The field is part of
                    // the format and nothing reads it; a real timestamp here
                    // would be a value the other two would import and discard.
                    lastUsed: "",
                )
            },
            userAssociation: stores.association.rows().map {
                BackupDocument.AssociationEntry(
                    prevWord: $0.pair.previous,
                    prevTl: $0.pair.previousTl,
                    nextWord: $0.pair.next,
                    nextTl: $0.pair.nextTl,
                    count: $0.count,
                    lastUsed: "",
                )
            },
        )

        let encoder = JSONEncoder()
        // Sorted and pretty-printed so the file is readable and diffable by
        // hand. Not byte-stable across exports — `exportedAt` moves, and the
        // row order is each store's own listing order — but two exports taken
        // a moment apart differ only where something actually changed.
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        return try encoder.encode(document)
    }

    // MARK: - Import

    /// Restores every category it can, and reports each one separately.
    ///
    /// A category that throws does not stop the others: the file has already
    /// been read, and refusing to restore the associations because the
    /// frequencies failed would throw away data the user still has a right to.
    func restore(from data: Data) async throws -> BackupImportResult {
        let document = try JSONDecoder().decode(BackupDocument.self, from: data)
        guard Self.supportedVersions.contains(document.version) else {
            throw BackupError.unsupportedVersion(document.version)
        }

        let customDictionary = await outcome {
            let rows = document.customDictionary.map {
                CustomDictionaryRow(roman: $0.roman, hanzi: $0.hanzi)
            }
            return try await stores.customDictionary.batchImport(rows).imported
        }

        let frequency = await outcome {
            try await stores.frequency.batchImportMerge(document.userFrequency.map {
                FrequencyRow(
                    word: $0.word,
                    // A version-1 backup has no reading. Empty is the bucket
                    // the engine already falls back to, not a guess at one.
                    tl: $0.tl ?? "",
                    count: $0.count,
                    // The stored timestamp is the store's own; the file
                    // carries none worth restoring.
                    lastUsedMillis: 0,
                )
            })
        }

        let association = await outcome {
            let needsFold = document.version < 2
            return try await stores.association.batchImportMerge(document.userAssociation.map { entry in
                AssociationRow(
                    pair: AssociationPair(
                        previous: entry.prevWord,
                        previousTl: Self.reading(entry.prevTl ?? "", foldingFromPoj: needsFold),
                        next: entry.nextWord,
                        nextTl: Self.reading(entry.nextTl, foldingFromPoj: needsFold),
                    ),
                    count: entry.count,
                )
            })
        }

        return BackupImportResult(
            customDictionary: customDictionary,
            frequency: frequency,
            association: association,
        )
    }

    private func outcome(_ body: () async throws -> Int) async -> BackupCategoryOutcome {
        do {
            return try await .restored(body())
        } catch {
            return .failed(String(describing: error))
        }
    }

    /// A reading as the identity key spells it.
    ///
    /// Version 2 declares its readings canonical TL already, so they are
    /// stored verbatim. Only a version-1 file — written before the format said
    /// which script it carried — is folded, and a fold the engine cannot do
    /// leaves the reading alone rather than dropping the row.
    ///
    /// NAMED CROSS-PLATFORM DIVERGENCE, classified **deferred**
    /// (`.claude/rules/cross-platform-alignment.md` §3): iOS folds every
    /// restored reading regardless of version
    /// (`ios/.../BackupService.swift:180-190`). That rewrites a TL reading the
    /// two scripts spell differently — POJ `eng` is TL `ing`, so a canonical
    /// TL `tíng` survives but a dialectal TL `eng` does not. Fixing iOS is a
    /// round of its own; macOS does not reproduce it on a v2 file.
    private static func reading(_ stored: String, foldingFromPoj: Bool) -> String {
        guard foldingFromPoj, !stored.isEmpty else { return stored }
        return RustEngineBridge.pojToTl(stored) ?? stored
    }

    private static func bundleVersion() -> String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0"
    }

    /// ISO-8601 UTC, POSIX-fixed, so the stamp reads the same wherever the
    /// file is opened.
    private static let timestampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss'Z'"
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter
    }()
}

/// The `.taigi` document itself.
///
/// Field names AND types are the wire format, shared with iOS
/// (`ios/.../Lexicon/Services/BackupService.swift:33-70`) and Android. This is
/// the only path user data takes between the three platforms, so a field that
/// is optional there must be optional here and a `String` there must be a
/// `String` here — a stricter type on this side would simply fail to decode a
/// file the phones wrote.
struct BackupDocument: Codable, Equatable, Sendable {
    let version: Int
    let exportedAt: String
    let platform: String
    let appVersion: String
    let customDictionary: [CustomEntry]
    let userFrequency: [FrequencyEntry]
    let userAssociation: [AssociationEntry]

    struct CustomEntry: Codable, Equatable, Sendable {
        let roman: String
        let hanzi: String
    }

    /// `tl` is optional because a version-1 backup predates the per-reading
    /// key. `lastUsed` is a REQUIRED string that every platform writes empty:
    /// it is vestigial, kept because iOS and Android declare it non-optional
    /// and would refuse a file that omitted it.
    struct FrequencyEntry: Codable, Equatable, Sendable {
        let word: String
        let tl: String?
        let count: Int
        let lastUsed: String
    }

    /// `prevTl` is optional and `nextTl` is not — the asymmetry is iOS's, and
    /// changing either side of it here would break decoding one direction.
    struct AssociationEntry: Codable, Equatable, Sendable {
        let prevWord: String
        let prevTl: String?
        let nextWord: String
        let nextTl: String
        let count: Int
        let lastUsed: String
    }
}
