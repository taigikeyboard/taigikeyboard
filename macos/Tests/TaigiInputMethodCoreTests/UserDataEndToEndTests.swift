import AppKit
import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives `EngineUserDataClient`, `UserDataRules`, `CustomDictionaryPageModel`,
/// `LearningRecordsPageModel`, `SettingsStore`, and `TaigiInputController`
/// (`SymbolSwap` + `insertSymbol`) against real SQLite stores in a dedicated
/// child `xctest` process so the parent suite's `TestDesktopCore` (configured
/// with `dataDirectory: nil`) stays untouched.
@MainActor
final class UserDataEndToEndTests: XCTestCase {
    private static let childEnvKey = "TAIGI_USER_DATA_E2E_CHILD"

    func testEndToEnd_realSQLiteAndDesktopCoreSeam() async throws {
        if ProcessInfo.processInfo.environment[Self.childEnvKey] != "1" {
            try runInIsolatedChildProcess()
            return
        }
        try await runRealSQLiteAndDesktopCoreSuite()
    }

    private func runInIsolatedChildProcess() throws {
        let bundlePath = Bundle(for: Self.self).bundlePath
        XCTAssertTrue(
            bundlePath.hasSuffix(".xctest"),
            "expected .xctest bundle path, got \(bundlePath)",
        )

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/xcrun")
        process.arguments = [
            "xctest",
            "-XCTest",
            "TaigiInputMethodCoreTests.UserDataEndToEndTests/testEndToEnd_realSQLiteAndDesktopCoreSeam",
            bundlePath,
        ]
        var environment = ProcessInfo.processInfo.environment
        environment[Self.childEnvKey] = "1"
        process.environment = environment

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        try process.run()
        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()

        let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
        let stderr = String(data: stderrData, encoding: .utf8) ?? ""
        XCTAssertEqual(
            process.terminationStatus,
            0,
            """
            Isolated child xctest failed with status \(process.terminationStatus)
            --- stdout ---
            \(stdout)
            --- stderr ---
            \(stderr)
            """,
        )
        XCTAssertTrue(
            stdout.contains("passed") || stderr.contains("passed"),
            "expected child xctest output to confirm execution\nstdout:\n\(stdout)\nstderr:\n\(stderr)",
        )
    }

    private func runRealSQLiteAndDesktopCoreSuite() async throws {
        let tempRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("TaigiUserDataE2E.\(UUID().uuidString)", isDirectory: true)
        let dataDir = tempRoot.appendingPathComponent("userdata", isDirectory: true)
        let scratchDir = tempRoot.appendingPathComponent("scratch", isDirectory: true)
        try FileManager.default.createDirectory(at: dataDir, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: scratchDir, withIntermediateDirectories: true)
        let suiteName = "UserDataEndToEndTests.\(UUID().uuidString)"
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer {
            userDefaults.removePersistentDomain(forName: suiteName)
            try? FileManager.default.removeItem(at: tempRoot)
        }

        // Configure and Prepare once in this child process with real SQLite + lexicon.
        let runtime = try XCTUnwrap(
            DesktopCoreRuntime.configure(
                DesktopCoreRuntime.Configuration(
                    dataDirectory: dataDir,
                    dictionariesDirectory: TestFixtures.dictionaryDirectory,
                    dictionaryStamp: 1,
                    systemLocale: "zh-Hant-TW",
                ),
                userDefaults: userDefaults,
            ),
            "child process must configure DesktopCoreRuntime once with real dataDirectory",
        )
        let prepared = try XCTUnwrap(runtime.prepare())
        XCTAssertTrue(prepared.hasLexicon)
        XCTAssertGreaterThan(prepared.lexicon.dictionaryRecordCount, 0)

        let settings = SettingsStore(userDefaults: userDefaults)
        let client = EngineUserDataClient()
        let window = NSWindow()

        // 1. CustomDictionaryPageModel -> real EngineUserDataClient -> real SQLite.
        let customPanels = FakeFilePanels()
        let customModel = CustomDictionaryPageModel(client: client, panels: customPanels)

        await customModel.save(CustomDictionaryRow(roman: "khór", hanji: "苦"))
        XCTAssertNil(customModel.message)
        XCTAssertEqual(customModel.list.rows.count, 1)
        let inserted = try XCTUnwrap(customModel.list.rows.first)
        XCTAssertFalse(inserted.id.isEmpty, "engine must mint UUID for new empty-id row")
        XCTAssertEqual(inserted.hanji, "苦")

        var edited = inserted
        edited.hanji = "苦瓜"
        await customModel.save(edited)
        XCTAssertEqual(customModel.list.rows.first?.id, inserted.id)
        XCTAssertEqual(customModel.list.rows.first?.hanji, "苦瓜")

        customModel.inputMode = .tps
        await customModel.load()
        XCTAssertEqual(customModel.list.rows.first?.displayRoman, "ㄎㄜˋ")

        await customModel.exportCSV(in: window)
        let exportedCSV = try XCTUnwrap(customPanels.written)
        XCTAssertNotNil(
            exportedCSV.suggestedName.wholeMatch(of: /taigi_custom_dictionary_\d{4}-\d{2}-\d{2}\.csv/),
            exportedCSV.suggestedName,
        )
        XCTAssertTrue(String(data: exportedCSV.data, encoding: .utf8)?.contains("khór,苦瓜") == true)

        await customModel.deleteAll()
        XCTAssertEqual(customModel.list.totalCount, 0)

        let csvFile = scratchDir.appendingPathComponent("import.csv")
        try exportedCSV.data.write(to: csvFile)
        customPanels.fileToOpen = csvFile
        await customModel.importCSV(in: window)
        XCTAssertEqual(customModel.list.totalCount, 1)
        XCTAssertEqual(customModel.list.rows.first?.hanji, "苦瓜")

        // 2. Real TaigiInputController composing -> Frequency SQLite row + SymbolSwap E2E.
        let textClient = RecordingTextInputClient()
        textClient.documentTextForReads = ""
        textClient.selectedRangeToReturn = NSRange(location: 0, length: 0)
        for index in 0 ... 20 {
            textClient.caretRects[index] = CGRect(x: 120, y: 400, width: 1, height: 18)
        }

        let controller = try XCTUnwrap(TaigiInputController(server: nil, delegate: nil, client: nil))
        let candidatePresenter = RecordingCandidatePresenter()
        let pickerPresenter = RecordingCandidatePresenter()
        controller.candidatePresenter = candidatePresenter
        controller.symbolPickerPresenter = pickerPresenter
        controller.settings = settings
        controller.activateServer(textClient)
        defer { controller.deactivateServer(textClient) }

        // Type "taigi" with Hanji-first so Return commits a real Frequency row to SQLite.
        userDefaults.set(true, forKey: SettingsStore.Keys.isHanjiFirst.name)
        for scalar in "taigi" {
            XCTAssertTrue(
                try controller.handle(TestFixtures.keyDownEvent(characters: String(scalar)), client: textClient),
            )
        }
        XCTAssertTrue(
            try controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: textClient),
        )
        XCTAssertEqual(textClient.documentTextForReads, "台語")

        // Switch to Roman-first with Auto-Space ON -> commit arms trailing auto-space.
        settings.isAutoSpaceEnabled = true
        userDefaults.set(false, forKey: SettingsStore.Keys.isHanjiFirst.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)

        for scalar in "taigi" {
            XCTAssertTrue(
                try controller.handle(TestFixtures.keyDownEvent(characters: String(scalar)), client: textClient),
            )
        }
        XCTAssertTrue(
            try controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: textClient),
        )
        XCTAssertEqual(textClient.documentTextForReads, "台語taigi ")

        // Pick attaching "，" via symbol picker -> SymbolSwap says yes, swaps trailing space.
        settings.noteRecentSymbol("，")
        controller.openSymbolPicker(client: textClient)
        XCTAssertTrue(
            try controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: textClient),
        )
        XCTAssertEqual(textClient.documentTextForReads, "台語taigi，")

        // Commit again -> arm trailing space; pick non-attaching "（）" -> space is kept.
        for scalar in "taigi" {
            XCTAssertTrue(
                try controller.handle(TestFixtures.keyDownEvent(characters: String(scalar)), client: textClient),
            )
        }
        XCTAssertTrue(
            try controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: textClient),
        )
        XCTAssertEqual(textClient.documentTextForReads, "台語taigi，taigi ")

        settings.noteRecentSymbol("（）")
        controller.openSymbolPicker(client: textClient)
        XCTAssertTrue(
            try controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: textClient),
        )
        XCTAssertEqual(textClient.documentTextForReads, "台語taigi，taigi （）")

        // 3. LearningRecordsPageModel -> real EngineUserDataClient -> real SQLite + .taigi backup.
        let learningPanels = FakeFilePanels()
        let learningModel = LearningRecordsPageModel(client: client, settings: settings, panels: learningPanels)
        learningModel.kind = .frequency
        learningModel.order = .mostUsed
        await learningModel.load()
        XCTAssertNil(learningModel.message)
        XCTAssertGreaterThan(learningModel.list.totalCount, 0)
        let freqRow = try XCTUnwrap(learningModel.list.rows.first { $0.text == "台語" })

        await learningModel.setCount(of: freqRow, to: 42)
        let updatedFreq = try XCTUnwrap(learningModel.list.rows.first { $0.text == "台語" })
        XCTAssertEqual(updatedFreq.count, 42)

        await learningModel.exportBackup(in: window)
        let exportedBackup = try XCTUnwrap(learningPanels.written)
        XCTAssertNotNil(
            exportedBackup.suggestedName.wholeMatch(of: /taigi_backup_\d{4}-\d{2}-\d{2}\.taigi/),
            exportedBackup.suggestedName,
        )

        await customModel.deleteAll()
        await learningModel.clearAll()
        XCTAssertEqual(learningModel.list.totalCount, 0)
        await customModel.load()
        XCTAssertEqual(customModel.list.totalCount, 0)

        let backupFile = scratchDir.appendingPathComponent("restore.taigi")
        try exportedBackup.data.write(to: backupFile)
        learningPanels.fileToOpen = backupFile
        await learningModel.importBackup(in: window)
        XCTAssertEqual(learningModel.list.rows.first { $0.text == "台語" }?.count, 42)

        await customModel.load()
        XCTAssertEqual(customModel.list.totalCount, 1)
        XCTAssertEqual(customModel.list.rows.first?.hanji, "苦瓜")

        // 4. SettingsStore persistence uses UserDataRules spellings cleanly.
        settings.learningRecordsKind = .learnedPhrase
        settings.learningRecordsOrder = .mostRecent
        XCTAssertEqual(userDefaults.string(forKey: "learningRecordsKind"), "learnedPhrase")
        XCTAssertEqual(userDefaults.string(forKey: "learningRecordsOrder"), "mostRecent")
    }
}
