import AppKit
import CoreText
import os
@testable import TaigiInputMethodCore
import XCTest

/// Drives the full macOS custom-font pipeline in isolated child `xctest` processes
/// without touching `~/Library/Input Methods`, `~/Library/Application Support`, or
/// the parent suite's `TestDesktopCore`:
///
/// 1. `import` phase: sanitised read-only `.ttf` copy through `FontLibraryStorage`
///    → `DesktopCoreBridge` → `taigi-macos-ffi` → `taigi-desktop-storage`, Core Text
///    `.process` registration, `SettingsStore.candidateFontSelection`, horizontal &
///    vertical candidate panels plus `TaigiInputController` + `CandidatePanel.shared`,
///    collision `-2` naming, live S30 removal with panel release, and re-import under
///    a new filename.
/// 2. `cold-launch` phase: fresh process starting with zero registered custom faces,
///    confirming read-only `candidateFontSelection` fallback before activation, cold
///    `CustomFontLibrary.activate(fileName:)` before `DesktopCoreRuntime.configure`,
///    directory rediscovery via `installedFonts()`, and immediate fallback after an
///    external deletion on disk.
@MainActor
final class CustomFontEndToEndTests: XCTestCase {
    private static let phaseEnvKey = "TAIGI_CUSTOM_FONT_E2E_PHASE"
    private static let rootEnvKey = "TAIGI_CUSTOM_FONT_E2E_ROOT"
    private static let suiteEnvKey = "TAIGI_CUSTOM_FONT_E2E_SUITE"
    private static let idEnvKey = "TAIGI_CUSTOM_FONT_E2E_ID"

    private static let reimportedFileName = "Reimported Face.ttf"

    func testEndToEnd_customFontImportRenderRemoveAndColdLaunch() throws {
        let env = ProcessInfo.processInfo.environment
        guard let phase = env[Self.phaseEnvKey] else {
            try runInIsolatedChildProcesses()
            return
        }
        let rootPath = try XCTUnwrap(env[Self.rootEnvKey], "missing \(Self.rootEnvKey)")
        let suiteName = try XCTUnwrap(env[Self.suiteEnvKey], "missing \(Self.suiteEnvKey)")
        let runID = try XCTUnwrap(env[Self.idEnvKey], "missing \(Self.idEnvKey)")
        let tempRoot = URL(fileURLWithPath: rootPath, isDirectory: true)
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))

        switch phase {
        case "import":
            try runImportRenderRemovePhase(tempRoot: tempRoot, userDefaults: userDefaults, runID: runID)
        case "cold-launch":
            try runColdLaunchPhase(tempRoot: tempRoot, userDefaults: userDefaults, runID: runID)
        default:
            XCTFail("unexpected E2E phase: \(phase)")
        }
    }

    private func runInIsolatedChildProcesses() throws {
        let bundlePath = Bundle(for: Self.self).bundlePath
        XCTAssertTrue(
            bundlePath.hasSuffix(".xctest"),
            "expected .xctest bundle path, got \(bundlePath)",
        )

        let tempRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("TaigiCustomFontE2E.\(UUID().uuidString)", isDirectory: true)
        let dataDir = tempRoot.appendingPathComponent("data", isDirectory: true)
        let fontsDir = tempRoot.appendingPathComponent("fonts", isDirectory: true)
        let scratchDir = tempRoot.appendingPathComponent("scratch", isDirectory: true)
        let secondScratchDir = scratchDir.appendingPathComponent("second", isDirectory: true)
        for dir in [dataDir, fontsDir, scratchDir, secondScratchDir] {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }

        let suiteName = "CustomFontEndToEndTests.\(UUID().uuidString)"
        let runID = String(UUID().uuidString.prefix(8))
        defer {
            UserDefaults(suiteName: suiteName)?.removePersistentDomain(forName: suiteName)
            try? FileManager.default.removeItem(at: tempRoot)
        }

        try runChildProcess(
            phase: "import",
            tempRoot: tempRoot,
            suiteName: suiteName,
            runID: runID,
            bundlePath: bundlePath,
        )
        try runChildProcess(
            phase: "cold-launch",
            tempRoot: tempRoot,
            suiteName: suiteName,
            runID: runID,
            bundlePath: bundlePath,
        )
    }

    private func runChildProcess(
        phase: String,
        tempRoot: URL,
        suiteName: String,
        runID: String,
        bundlePath: String,
    ) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/xcrun")
        process.arguments = [
            "xctest",
            "-XCTest",
            "TaigiInputMethodCoreTests.CustomFontEndToEndTests/testEndToEnd_customFontImportRenderRemoveAndColdLaunch",
            bundlePath,
        ]
        var environment = ProcessInfo.processInfo.environment
        environment[Self.phaseEnvKey] = phase
        environment[Self.rootEnvKey] = tempRoot.path(percentEncoded: false)
        environment[Self.suiteEnvKey] = suiteName
        environment[Self.idEnvKey] = runID
        process.environment = environment

        let outputPipe = Pipe()
        process.standardOutput = outputPipe
        process.standardError = outputPipe

        let timedOut = OSAllocatedUnfairLock(initialState: false)
        let deadline = DispatchWorkItem {
            timedOut.withLock { $0 = true }
            if process.isRunning {
                process.terminate()
            }
        }
        DispatchQueue.global().asyncAfter(deadline: .now() + .seconds(30), execute: deadline)
        try process.run()
        let outputData = outputPipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        deadline.cancel()

        let output = String(data: outputData, encoding: .utf8) ?? ""
        XCTAssertFalse(
            timedOut.withLock { $0 },
            "Isolated child xctest (\(phase)) timed out after 30s\n--- output ---\n\(output)",
        )
        XCTAssertEqual(
            process.terminationStatus,
            0,
            """
            Isolated child xctest (\(phase)) failed with status \(process.terminationStatus)
            --- output ---
            \(output)
            """,
        )
        XCTAssertTrue(
            output.contains("Executed 1 test, with 0 failures"),
            "expected child xctest (\(phase)) output to confirm execution\noutput:\n\(output)",
        )
    }

    // MARK: - Phase 1: Import, Render, Remove, Re-import

    private func runImportRenderRemovePhase(
        tempRoot: URL,
        userDefaults: UserDefaults,
        runID: String,
    ) throws {
        let dataDir = tempRoot.appendingPathComponent("data", isDirectory: true)
        let fontsDir = tempRoot.appendingPathComponent("fonts", isDirectory: true)
        let scratchDir = tempRoot.appendingPathComponent("scratch", isDirectory: true)
        let secondScratchDir = scratchDir.appendingPathComponent("second", isDirectory: true)

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
            "child process must configure DesktopCoreRuntime once with isolated dataDirectory",
        )
        let prepared = try XCTUnwrap(runtime.prepare())
        XCTAssertTrue(prepared.hasLexicon)
        XCTAssertGreaterThan(prepared.lexicon.dictionaryRecordCount, 0)

        let library = CustomFontLibrary(directory: fontsDir)
        let settings = SettingsStore(userDefaults: userDefaults, customFontLibrary: library)

        let family1 = "TaigiE2E \(runID)"
        let ps1 = "TaigiE2E-\(runID)"
        let font1Bytes = MinimalTrueTypeFontBuilder.build(family: family1, postScript: ps1)

        let family2 = "TaigiE2EAlt \(runID)"
        let ps2 = "TaigiE2EAlt-\(runID)"
        let font2Bytes = MinimalTrueTypeFontBuilder.build(family: family2, postScript: ps2)

        XCTAssertFalse(RegisteredFace.isRegistered(named: ps1))
        XCTAssertFalse(RegisteredFace.isRegistered(named: ps2))

        let source1 = scratchDir.appendingPathComponent("My Custom Font:2026.ttf")
        try font1Bytes.write(to: source1)
        try FileManager.default.setAttributes([.posixPermissions: 0o444], ofItemAtPath: source1.path)
        defer {
            try? FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: source1.path)
        }

        let source2 = secondScratchDir.appendingPathComponent("My Custom Font:2026.ttf")
        try font2Bytes.write(to: source2)

        // 1. Import read-only font through shared Rust storage + Core Text registration.
        let font1 = try library.addFont(from: source1)
        XCTAssertEqual(font1.fileName, "My Custom Font-2026.ttf")
        XCTAssertEqual(font1.displayName, "My Custom Font-2026")
        XCTAssertEqual(font1.postScriptName, ps1)

        let copy1URL = try library.directory().appendingPathComponent(font1.fileName).standardizedFileURL
        let attrs1 = try FileManager.default.attributesOfItem(atPath: copy1URL.path)
        let mode1 = try XCTUnwrap(attrs1[.posixPermissions] as? Int)
        XCTAssertNotEqual(mode1 & 0o200, 0, "stored library copy must be owner-writable")
        XCTAssertEqual(FontLibraryStorage.listStored(in: fontsDir), [font1.fileName])
        XCTAssertEqual(library.installedFonts(), [font1])
        XCTAssertEqual(library.activatedFont(fileName: font1.fileName), font1)
        XCTAssertEqual(library.registeredFamilies(), [family1])
        XCTAssertFalse(
            RegisteredFace.installedFamilies(excluding: library.registeredFamilies()).contains(family1),
        )

        // Duplicate import while active must fail cleanly without leaving another copy on disk.
        XCTAssertThrowsError(try library.addFont(from: source1)) { error in
            guard case CustomFontLibrary.ImportFailure.nameAlreadyResolves(ps1) = error else {
                return XCTFail("unexpected duplicate-face error: \(error)")
            }
        }
        XCTAssertEqual(FontLibraryStorage.listStored(in: fontsDir), [font1.fileName])

        // 2. Select in SettingsStore and render candidate panels + live controller path.
        persistSelection(.customFile(font1.fileName), to: userDefaults)
        XCTAssertEqual(settings.candidateFontSelection, .custom(font1))
        XCTAssertEqual(settings.candidateMetrics.fontSelection, .custom(font1))
        XCTAssertEqual(settings.candidateMetrics.candidateFont.fontName, ps1)
        XCTAssertEqual(try resolvedFileURL(of: settings.candidateMetrics.candidateFont), copy1URL)

        try renderAndVerifyPanels(
            settings: settings,
            font: font1,
            expectedURL: copy1URL,
            userDefaults: userDefaults,
        )

        // 3. Import a distinct face with colliding filename -> numeric "-2" suffix without clobbering copy 1.
        let font2 = try library.addFont(from: source2)
        XCTAssertEqual(font2.fileName, "My Custom Font-2026-2.ttf")
        XCTAssertEqual(font2.postScriptName, ps2)
        XCTAssertEqual(try Data(contentsOf: copy1URL), font1Bytes)
        XCTAssertEqual(
            FontLibraryStorage.listStored(in: fontsDir),
            [font2.fileName, font1.fileName],
        )

        // 4. Switch selection away, release cached panels, and remove font1 cleanly (S30).
        persistSelection(.builtIn(.system), to: userDefaults)
        CandidatePanel.shared.releaseCachedPanels(drawing: font1)
        CandidatePanel.shared.settings = SettingsStore()
        RunLoop.current.run(until: Date().addingTimeInterval(0.02))

        try library.remove(font1)
        XCTAssertFalse(RegisteredFace.isRegistered(named: ps1))
        XCTAssertNil(library.activatedFont(fileName: font1.fileName))
        XCTAssertEqual(FontLibraryStorage.listStored(in: fontsDir), [font2.fileName])
        XCTAssertEqual(library.registeredFamilies(), [family2])
        XCTAssertEqual(
            CandidateFontSelection.custom(font1).font(ofSize: 16).fontName,
            NSFont.systemFont(ofSize: 16).fontName,
        )

        // 5. Re-import the removed face under a new filename and confirm Core Text draws from the new copy.
        let reimportSource = scratchDir.appendingPathComponent(Self.reimportedFileName)
        try font1Bytes.write(to: reimportSource)
        let reimported = try library.addFont(from: reimportSource)
        XCTAssertEqual(reimported.fileName, Self.reimportedFileName)
        XCTAssertEqual(reimported.postScriptName, ps1)
        XCTAssertEqual(library.activatedFont(fileName: reimported.fileName), reimported)

        let reimportedCopyURL = try library.directory()
            .appendingPathComponent(Self.reimportedFileName)
            .standardizedFileURL
        XCTAssertEqual(
            try resolvedFileURL(of: CandidateFontSelection.custom(reimported).font(ofSize: 16)),
            reimportedCopyURL,
            "re-imported face must draw from its new library path rather than a cached old path",
        )

        persistSelection(.customFile(reimported.fileName), to: userDefaults)
        XCTAssertTrue(userDefaults.synchronize())
    }

    private func renderAndVerifyPanels(
        settings: SettingsStore,
        font: CustomFont,
        expectedURL: URL,
        userDefaults: UserDefaults,
    ) throws {
        let sampleCells = [
            CandidateCellContent(text: "台語", annotation: "Tâi-gí"),
            CandidateCellContent(text: "台灣", annotation: "Tâi-uân"),
        ]

        // Build standalone horizontal & vertical panels in a tightly scoped block so their
        // `NSTextField` views release before `library.remove(font)` is called.
        do {
            let horizontalMetrics = settings.candidateMetrics.arranged(.inline)
            let horizontalPanel = HorizontalCandidatePanel(style: .sequoia, metrics: horizontalMetrics)
            _ = horizontalPanel.layout(sampleCells, forCaret: .zero)
            try assertPanelCells(
                in: horizontalPanel,
                metrics: horizontalMetrics,
                postScriptName: font.postScriptName,
                expectedURL: expectedURL,
            )
            horizontalPanel.clear()

            let verticalMetrics = settings.candidateMetrics.arranged(.inline)
            let verticalPanel = VerticalCandidatePanel(style: .sequoia, metrics: verticalMetrics)
            _ = verticalPanel.layout(sampleCells, forCaret: .zero)
            try assertPanelCells(
                in: verticalPanel,
                metrics: verticalMetrics,
                postScriptName: font.postScriptName,
                expectedURL: expectedURL,
            )
            verticalPanel.clear()
        }

        // Exercise the live composing -> CandidatePanel.shared show path.
        let textClient = RecordingTextInputClient()
        textClient.documentTextForReads = ""
        textClient.selectedRangeToReturn = NSRange(location: 0, length: 0)
        for index in 0 ... 20 {
            textClient.caretRects[index] = CGRect(x: 120, y: 400, width: 1, height: 18)
        }
        userDefaults.set(CandidateLayout.horizontal.rawValue, forKey: SettingsStore.Keys.candidateLayout.name)
        CandidatePanel.shared.settings = settings

        let controller = try XCTUnwrap(TaigiInputController(server: nil, delegate: nil, client: nil))
        controller.candidatePresenter = CandidatePanel.shared
        controller.settings = settings
        controller.activateServer(textClient)
        defer {
            CandidatePanel.shared.hideForHandover()
            controller.deactivateServer(textClient)
        }

        for scalar in "taigi" {
            XCTAssertTrue(
                try controller.handle(TestFixtures.keyDownEvent(characters: String(scalar)), client: textClient),
            )
        }

        if let presentedPanel = NSApplication.shared.windows
            .compactMap({ $0 as? CandidateBasePanel })
            .first(where: { $0.isVisible && $0.configuredMetrics.fontSelection == .custom(font) })
        {
            try assertPanelCells(
                in: presentedPanel,
                metrics: presentedPanel.metrics,
                postScriptName: font.postScriptName,
                expectedURL: expectedURL,
            )
        }
    }

    // MARK: - Phase 2: Cold Launch Discovery & Activation

    private func runColdLaunchPhase(
        tempRoot: URL,
        userDefaults: UserDefaults,
        runID: String,
    ) throws {
        let fontsDir = tempRoot.appendingPathComponent("fonts", isDirectory: true)
        let ps1 = "TaigiE2E-\(runID)"
        let ps2 = "TaigiE2EAlt-\(runID)"

        XCTAssertFalse(RegisteredFace.isRegistered(named: ps1))
        XCTAssertFalse(RegisteredFace.isRegistered(named: ps2))

        let library = CustomFontLibrary(directory: fontsDir)
        let settings = SettingsStore(userDefaults: userDefaults, customFontLibrary: library)

        XCTAssertEqual(settings.storedFontSelection, .customFile(Self.reimportedFileName))
        XCTAssertEqual(
            settings.candidateFontSelection,
            .default,
            "reading settings before activation must fall back to system without registering",
        )

        // Mirror `AppDelegate.applicationDidFinishLaunching` activating the stored custom font
        // before `DesktopCoreRuntime.configure`.
        let activated = try XCTUnwrap(
            library.activate(fileName: settings.storedFontSelection.customFontFile),
        )
        let expectedFont = CustomFont(fileName: Self.reimportedFileName, postScriptName: ps1)
        XCTAssertEqual(activated, expectedFont)
        XCTAssertEqual(settings.candidateFontSelection, .custom(expectedFont))

        let reimportedCopyURL = try library.directory()
            .appendingPathComponent(Self.reimportedFileName)
            .standardizedFileURL
        do {
            let metrics = settings.candidateMetrics
            let cell = CandidateItemView(style: .sequoia, metrics: metrics)
            cell.configure(CandidateCellContent(text: "台語", annotation: "Tâi-gí"))
            try assertCellLabels(
                in: cell,
                metrics: metrics,
                postScriptName: ps1,
                expectedURL: reimportedCopyURL,
            )
        }

        XCTAssertEqual(
            library.installedFonts().map(\.fileName),
            ["My Custom Font-2026-2.ttf", Self.reimportedFileName],
        )

        // Removing the file on disk outside the app must immediately fail `stat` lookup.
        try FileManager.default.removeItem(at: reimportedCopyURL)
        XCTAssertNil(library.activatedFont(fileName: Self.reimportedFileName))
        XCTAssertEqual(settings.candidateFontSelection, .default)
    }

    // MARK: - Helpers

    private func persistSelection(_ selection: StoredFontSelection, to userDefaults: UserDefaults) {
        userDefaults.set(selection.fontType, forKey: SettingsStore.Keys.fontType.name)
        userDefaults.set(selection.customFontFile, forKey: SettingsStore.Keys.customFontFile.name)
        userDefaults.set(selection.installedFontFamily, forKey: SettingsStore.Keys.installedFontFamily.name)
        userDefaults.set(selection.installedFontFace, forKey: SettingsStore.Keys.installedFontFace.name)
    }

    private func resolvedFileURL(of font: NSFont) throws -> URL {
        try XCTUnwrap(CTFontCopyAttribute(font as CTFont, kCTFontURLAttribute) as? URL)
            .standardizedFileURL
    }

    private func assertPanelCells(
        in panel: CandidateBasePanel,
        metrics: CandidateMetrics,
        postScriptName: String,
        expectedURL: URL,
    ) throws {
        let cells = TestFixtures.candidateCells(in: panel)
        XCTAssertFalse(cells.isEmpty, "\(type(of: panel)) laid out no visible candidate cells")
        for cell in cells {
            try assertCellLabels(
                in: cell,
                metrics: metrics,
                postScriptName: postScriptName,
                expectedURL: expectedURL,
            )
        }
    }

    private func assertCellLabels(
        in cell: CandidateItemView,
        metrics: CandidateMetrics,
        postScriptName: String,
        expectedURL: URL,
    ) throws {
        let labels = cell.subviews.compactMap { $0 as? NSTextField }
        XCTAssertEqual(labels.count, 3)
        XCTAssertEqual(labels[0].font?.fontName, metrics.indexFont.fontName)

        let candidateFont = try XCTUnwrap(labels[1].font)
        XCTAssertEqual(candidateFont.fontName, postScriptName)
        XCTAssertEqual(try resolvedFileURL(of: candidateFont), expectedURL)

        let annotationFont = try XCTUnwrap(labels[2].font)
        XCTAssertEqual(annotationFont.fontName, postScriptName)
        XCTAssertEqual(try resolvedFileURL(of: annotationFont), expectedURL)
    }
}

/// Synthesizes a valid minimal TrueType font (`.ttf`) with unique `name` records so Core Text
/// accepts `.process` registration deterministically on any Mac without skipping.
private enum MinimalTrueTypeFontBuilder {
    static func build(family: String, postScript: String) -> Data {
        let head = makeHeadTable()
        let hhea = makeHheaTable()
        let maxp = makeMaxpTable()
        let os2 = makeOS2Table()
        let hmtx: [UInt8] = be16(500) + be16(0) + be16(500) + be16(0)
        let loca: [UInt8] = [0, 0, 0, 0, 0, 0]
        let glyf: [UInt8] = [0, 0, 0, 0]
        let cmap = makeCmapTable()
        var post = [UInt8](repeating: 0, count: 32)
        post[0 ... 3] = [0, 3, 0, 0][...]
        let nameTable = makeNameTable(family: family, postScript: postScript)

        var tables: [(String, [UInt8])] = [
            ("OS/2", os2),
            ("cmap", cmap),
            ("glyf", glyf),
            ("head", head),
            ("hhea", hhea),
            ("hmtx", hmtx),
            ("loca", loca),
            ("maxp", maxp),
            ("name", nameTable),
            ("post", post),
        ]
        tables.sort { $0.0 < $1.0 }

        let numTables = UInt16(tables.count)
        var entrySelector: UInt16 = 0
        var maxPow2: UInt16 = 1
        while maxPow2 * 2 <= numTables {
            maxPow2 *= 2
            entrySelector += 1
        }
        let searchRange = maxPow2 * 16
        let rangeShift = numTables * 16 - searchRange

        var sfnt: [UInt8] = [0, 1, 0, 0]
            + be16(numTables)
            + be16(searchRange)
            + be16(entrySelector)
            + be16(rangeShift)
        var offset = UInt32(12 + tables.count * 16)
        var headOffset = 0
        var body: [UInt8] = []
        for (tag, bytes) in tables {
            let padded = pad4(bytes)
            if tag == "head" {
                headOffset = Int(offset)
            }
            sfnt += Array(tag.utf8)
            sfnt += be32(checksum(bytes))
            sfnt += be32(offset)
            sfnt += be32(UInt32(bytes.count))
            body += padded
            offset += UInt32(padded.count)
        }
        sfnt += body
        let adjustment = UInt32(0xB1B0_AFBA) &- checksum(sfnt)
        sfnt[headOffset + 8 ... headOffset + 11] = be32(adjustment)[...]
        return Data(sfnt)
    }

    private static func makeHeadTable() -> [UInt8] {
        var head = [UInt8](repeating: 0, count: 54)
        head[0 ... 3] = [0, 1, 0, 0][...]
        head[4 ... 7] = [0, 1, 0, 0][...]
        head[12 ... 15] = [0x5F, 0x0F, 0x3C, 0xF5][...]
        head[16 ... 17] = [0, 0x0B][...]
        head[18 ... 19] = be16(1000)[...]
        head[42 ... 43] = be16(500)[...]
        head[44 ... 45] = be16(800)[...]
        head[48 ... 49] = be16(8)[...]
        head[50 ... 51] = [0, 2][...]
        return head
    }

    private static func makeHheaTable() -> [UInt8] {
        var hhea = [UInt8](repeating: 0, count: 36)
        hhea[0 ... 3] = [0, 1, 0, 0][...]
        hhea[4 ... 5] = be16(800)[...]
        hhea[6 ... 7] = be16(UInt16(bitPattern: -200))[...]
        hhea[10 ... 11] = be16(500)[...]
        hhea[34 ... 35] = be16(2)[...]
        return hhea
    }

    private static func makeMaxpTable() -> [UInt8] {
        var maxp = [UInt8](repeating: 0, count: 32)
        maxp[0 ... 3] = [0, 1, 0, 0][...]
        maxp[4 ... 5] = be16(2)[...]
        return maxp
    }

    private static func makeOS2Table() -> [UInt8] {
        var os2 = [UInt8](repeating: 0, count: 78)
        os2[0 ... 1] = be16(1)[...]
        os2[2 ... 3] = be16(500)[...]
        os2[4 ... 5] = be16(400)[...]
        os2[6 ... 7] = be16(5)[...]
        os2[62 ... 63] = be16(0x0040)[...]
        os2[64 ... 65] = be16(0x0020)[...]
        os2[66 ... 67] = be16(0x0020)[...]
        os2[68 ... 69] = be16(800)[...]
        os2[70 ... 71] = be16(UInt16(bitPattern: -200))[...]
        os2[74 ... 75] = be16(800)[...]
        os2[76 ... 77] = be16(200)[...]
        return os2
    }

    private static func makeCmapTable() -> [UInt8] {
        var format4: [UInt8] = []
        format4 += be16(4)
        format4 += be16(32)
        format4 += be16(0)
        format4 += be16(4)
        format4 += be16(4)
        format4 += be16(1)
        format4 += be16(0)
        format4 += be16(0x0020) + be16(0xFFFF)
        format4 += be16(0)
        format4 += be16(0x0020) + be16(0xFFFF)
        format4 += be16(UInt16(bitPattern: Int16(1 - 0x0020))) + be16(1)
        format4 += be16(0) + be16(0)

        var cmap: [UInt8] = be16(0) + be16(1)
        cmap += be16(3) + be16(1) + be32(12)
        cmap += format4
        return cmap
    }

    private static func makeNameTable(family: String, postScript: String) -> [UInt8] {
        let records: [(UInt16, String)] = [
            (1, family),
            (2, "Regular"),
            (3, postScript),
            (4, "\(family) Regular"),
            (5, "Version 1.0"),
            (6, postScript),
        ]
        var entries: [(UInt16, UInt16, UInt16, UInt16, [UInt8])] = []
        for (nameID, value) in records {
            entries.append((1, 0, 0, nameID, Array(value.utf8)))
        }
        for (nameID, value) in records {
            var utf16: [UInt8] = []
            for codeUnit in value.utf16 {
                utf16 += be16(codeUnit)
            }
            entries.append((3, 1, 0x0409, nameID, utf16))
        }

        let stringOffset = UInt16(6 + entries.count * 12)
        var table: [UInt8] = be16(0) + be16(UInt16(entries.count)) + be16(stringOffset)
        var storage: [UInt8] = []
        for (platform, encoding, language, nameID, bytes) in entries {
            table += be16(platform)
                + be16(encoding)
                + be16(language)
                + be16(nameID)
                + be16(UInt16(bytes.count))
                + be16(UInt16(storage.count))
            storage += bytes
        }
        table += storage
        return table
    }

    private static func be16(_ value: UInt16) -> [UInt8] {
        [UInt8(value >> 8), UInt8(value & 0xFF)]
    }

    private static func be32(_ value: UInt32) -> [UInt8] {
        [
            UInt8((value >> 24) & 0xFF),
            UInt8((value >> 16) & 0xFF),
            UInt8((value >> 8) & 0xFF),
            UInt8(value & 0xFF),
        ]
    }

    private static func pad4(_ bytes: [UInt8]) -> [UInt8] {
        var padded = bytes
        while padded.count % 4 != 0 {
            padded.append(0)
        }
        return padded
    }

    private static func checksum(_ bytes: [UInt8]) -> UInt32 {
        let padded = pad4(bytes)
        var sum: UInt32 = 0
        for index in stride(from: 0, to: padded.count, by: 4) {
            let word = (UInt32(padded[index]) << 24)
                | (UInt32(padded[index + 1]) << 16)
                | (UInt32(padded[index + 2]) << 8)
                | UInt32(padded[index + 3])
            sum = sum &+ word
        }
        return sum
    }
}
