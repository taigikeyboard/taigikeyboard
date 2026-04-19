import Foundation

/// All-in-one user data backup and restore service
///
/// Exports and imports all user data (custom dictionary, frequency, associations)
/// as a single `.taigi` JSON file for device migration.
final class BackupService: @unchecked Sendable {
    // MARK: - Dependencies

    private let customDictionaryService: CustomDictionaryService
    private let userFrequencyRepository: UserFrequencyRepository
    private let nextWordService: NextWordService
    private let logger = DebugLogger(category: "BackupService")

    // MARK: - Initialization

    init(
        customDictionaryService: CustomDictionaryService = CompositionRoot.customDictionaryService,
        userFrequencyRepository: UserFrequencyRepository = CompositionRoot.userFrequencyRepository,
        nextWordService: NextWordService = CompositionRoot.nextWordService,
    ) {
        self.customDictionaryService = customDictionaryService
        self.userFrequencyRepository = userFrequencyRepository
        self.nextWordService = nextWordService
    }

    // MARK: - Models

    struct BackupData: Codable {
        let version: Int
        let exportedAt: String
        let platform: String
        let appVersion: String
        let customDictionary: [CustomDictEntry]
        let userFrequency: [FrequencyEntry]
        let userAssociation: [AssociationBackupEntry]
    }

    struct CustomDictEntry: Codable {
        let roman: String
        let hanzi: String
    }

    struct FrequencyEntry: Codable {
        let word: String
        let count: Int
        let lastUsed: String
    }

    struct AssociationBackupEntry: Codable {
        let prevWord: String
        let prevTl: String?
        let nextWord: String
        let nextTl: String
        let count: Int
        let lastUsed: String
    }

    struct ImportResult {
        let customDict: Int
        let frequency: Int
        let association: Int
    }

    // MARK: - Export

    func exportAll() async throws -> Data {
        let customEntries = try await customDictionaryService.fetchAll()
        let frequencyData = await userFrequencyRepository.topWordsAsync(limit: Int.max)
        let associationData = await nextWordService.allAssociations()

        let appVersion = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleShortVersionString",
        ) as? String ?? "1.0"

        let backup = BackupData(
            version: 1,
            exportedAt: ISO8601DateFormatter().string(from: Date()),
            platform: "ios",
            appVersion: appVersion,
            customDictionary: customEntries.map {
                CustomDictEntry(roman: $0.roman, hanzi: $0.hanzi)
            },
            userFrequency: frequencyData.map {
                FrequencyEntry(word: $0.word, count: $0.count, lastUsed: "")
            },
            userAssociation: associationData.map {
                AssociationBackupEntry(
                    prevWord: $0.prevWord,
                    prevTl: $0.prevTl,
                    nextWord: $0.nextWord,
                    nextTl: $0.nextTl,
                    count: $0.count,
                    lastUsed: "",
                )
            },
        )

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(backup)
    }

    // MARK: - Import

    func importAll(from data: Data) async throws -> ImportResult {
        let decoder = JSONDecoder()
        let backup = try decoder.decode(BackupData.self, from: data)

        guard backup.version >= 1 else {
            throw BackupError.unsupportedVersion(backup.version)
        }

        // Import custom dictionary (skip duplicates by roman+hanzi)
        let customCount = try await importCustomDictionary(backup.customDictionary)

        // Import frequency data (merge: higher count wins)
        let freqCount = await importFrequency(backup.userFrequency)

        // Import association data (merge: higher count wins)
        let assocCount = await importAssociations(backup.userAssociation)

        logger.info("[IMPORT] custom=\(customCount), freq=\(freqCount), assoc=\(assocCount)")
        return ImportResult(customDict: customCount, frequency: freqCount, association: assocCount)
    }

    // MARK: - Private Import Helpers

    private func importCustomDictionary(_ entries: [CustomDictEntry]) async throws -> Int {
        let existing = try await customDictionaryService.fetchAll()
        let existingPairs = Set(existing.map { "\($0.roman)\t\($0.hanzi)" })

        var imported = 0
        for entry in entries {
            let key = "\(entry.roman)\t\(entry.hanzi)"
            guard !existingPairs.contains(key) else { continue }

            let newEntry = CustomDictionaryEntry(roman: entry.roman, hanzi: entry.hanzi)
            try await customDictionaryService.save(newEntry)
            imported += 1
        }
        return imported
    }

    private func importFrequency(_ entries: [FrequencyEntry]) async -> Int {
        guard !entries.isEmpty else { return 0 }
        do {
            try await userFrequencyRepository.ensureInitialized()
            return try await userFrequencyRepository.batchImportMerge(entries: entries.map {
                (word: $0.word, count: $0.count)
            })
        } catch {
            logger.error("[IMPORT] Frequency import failed: \(error.localizedDescription)")
            return 0
        }
    }

    private func importAssociations(_ entries: [AssociationBackupEntry]) async -> Int {
        guard !entries.isEmpty else { return 0 }
        do {
            // Normalize prevTl/nextTl to TL format (old backups or cross-platform may contain POJ)
            return try await nextWordService.batchImportAssociations(entries: entries.map {
                (prevWord: $0.prevWord,
                 prevTl: RomanizationConverter.pojToTL($0.prevTl ?? ""),
                 nextWord: $0.nextWord,
                 nextTl: RomanizationConverter.pojToTL($0.nextTl),
                 count: $0.count)
            })
        } catch {
            logger.error("[IMPORT] Association import failed: \(error.localizedDescription)")
            return 0
        }
    }
}

// MARK: - Errors

enum BackupError: LocalizedError {
    case unsupportedVersion(Int)

    var errorDescription: String? {
        switch self {
        case let .unsupportedVersion(version):
            "Unsupported backup version: \(version)"
        }
    }
}
