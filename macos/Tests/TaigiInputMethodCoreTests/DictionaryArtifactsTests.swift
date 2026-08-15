// Locating the dictionary data, and refusing to pretend a partial set is fine.

import XCTest

@testable import TaigiInputMethodCore

final class DictionaryArtifactsTests: XCTestCase {
    func testInit_repositoryDictionaries_resolvesAllFourPaths() throws {
        let artifacts = try DictionaryArtifacts(baseURL: TestFixtures.dictionaryDirectory)

        XCTAssertTrue(artifacts.triePath.hasSuffix("/dictionary.fst"))
        XCTAssertTrue(artifacts.dictionaryBinPath.hasSuffix("/dictionary.bin"))
        XCTAssertTrue(artifacts.associationBinPath.hasSuffix("/association.bin"))
        XCTAssertTrue(artifacts.syllableInventoryPath.hasSuffix("/syllables.fst"))
    }

    func testInit_missingSyllableInventory_throwsRatherThanDegrading() throws {
        // The engine accepts an empty inventory path by loading no inventory, so
        // this case would otherwise install "successfully" and then return no
        // continuous candidates for every input.
        let directory = try makeDirectory(containing: ["dictionary.fst", "dictionary.bin", "association.bin"])

        XCTAssertThrowsError(try DictionaryArtifacts(baseURL: directory)) { error in
            XCTAssertEqual(
                error as? DictionaryArtifactsError,
                .unusableFile(fileName: "syllables.fst", directory: directory.path),
            )
        }
    }

    func testInit_emptyFile_throws() throws {
        let directory = try makeDirectory(
            containing: ["dictionary.fst", "dictionary.bin", "association.bin", "syllables.fst"],
            emptyFiles: ["dictionary.bin"],
        )

        XCTAssertThrowsError(try DictionaryArtifacts(baseURL: directory)) { error in
            XCTAssertEqual(
                error as? DictionaryArtifactsError,
                .unusableFile(fileName: "dictionary.bin", directory: directory.path),
            )
        }
    }

    private func makeDirectory(
        containing fileNames: [String],
        emptyFiles: [String] = [],
    ) throws -> URL {
        let directory = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("DictionaryArtifactsTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }

        for fileName in fileNames {
            let contents = emptyFiles.contains(fileName) ? Data() : Data([0x01])
            try contents.write(to: directory.appendingPathComponent(fileName))
        }
        return directory
    }
}
