// 中文: 全使用者資料的備份 / 還原服務 — 將自訂詞庫、頻率、聯想資料一次匯入 / 匯出
// 中文: 為單一 .taigi JSON 檔,給跨機備援用。

import Foundation

/// All-in-one user data backup and restore service
///
/// Exports and imports all user data (custom dictionary, frequency, associations)
/// as a single `.taigi` JSON file for device migration.
// 中文: 整體備份服務 — 一個檔案匯出 / 匯入所有 user data。
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

    // 中文: 備份檔的根 Codable — 版本 / 時戳 / 平台 / 三類 user data。
    struct BackupData: Codable {
        let version: Int
        let exportedAt: String
        let platform: String
        let appVersion: String
        let customDictionary: [CustomDictEntry]
        let userFrequency: [FrequencyEntry]
        let userAssociation: [AssociationBackupEntry]
    }

    // 中文: 備份檔內自訂詞庫項目 — 只保留 roman + hanzi,不存內部 id 與時戳。
    struct CustomDictEntry: Codable {
        let roman: String
        let hanzi: String
    }

    // 中文: 備份檔內頻率項目 — 詞、count 與最後使用時間字串。
    struct FrequencyEntry: Codable {
        let word: String
        let count: Int
        let lastUsed: String
    }

    // 中文: 備份檔內聯想項目 — prev / next 詞 + 對應 TL + count。
    struct AssociationBackupEntry: Codable {
        let prevWord: String
        let prevTl: String?
        let nextWord: String
        let nextTl: String
        let count: Int
        let lastUsed: String
    }

    // 中文: 匯入結果 — 三類資料各別實際匯入的筆數。
    struct ImportResult {
        let customDict: Int
        let frequency: Int
        let association: Int
    }

    // MARK: - Export

    // 中文: 匯出全部 user data 為 JSON Data。
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

    // 中文: 從備份 Data 匯入全部資料 — 三類各自走不同 merge 策略。
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

    // 中文: 匯入自訂詞庫 — 以 roman+hanzi 為去重鍵,只新增,不覆寫既有資料。
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

    // 中文: 匯入頻率資料 — merge-by-max,還原舊備份不會把使用者較高的 count 蓋掉。
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

    // 中文: 匯入聯想資料 — POJ 形式的舊 / 跨平台備份會被先 normalize 成 TL 再寫入。
    private func importAssociations(_ entries: [AssociationBackupEntry]) async -> Int {
        guard !entries.isEmpty else { return 0 }
        do {
            // Normalize prevTl/nextTl to TL format (old backups or cross-platform may contain POJ)
            return try await nextWordService.batchImportAssociations(entries: entries.map {
                (prevWord: $0.prevWord,
                 prevTl: RustEngineBridge.pojToTl($0.prevTl ?? ""),
                 nextWord: $0.nextWord,
                 nextTl: RustEngineBridge.pojToTl($0.nextTl),
                 count: $0.count)
            })
        } catch {
            logger.error("[IMPORT] Association import failed: \(error.localizedDescription)")
            return 0
        }
    }
}

// MARK: - Errors

// 中文: 備份模組的錯誤型別 — 目前只有不支援的 schema 版本一種情境。
enum BackupError: LocalizedError {
    case unsupportedVersion(Int)

    var errorDescription: String? {
        switch self {
        case let .unsupportedVersion(version):
            "Unsupported backup version: \(version)"
        }
    }
}
