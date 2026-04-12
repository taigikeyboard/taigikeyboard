import Foundation

/// Read-only binary dictionary reader (mmap-based)
///
/// Reads `dictionary.bin` — a compact binary format replacing SQLite
/// for read-only dictionary lookups. Uses memory-mapped I/O.
///
/// Binary format (little-endian):
///   Header: "TKDB" (4) + version u32 (4) + count u32 (4) + build_ts u32 (4)
///   Offset table: count × u32 (absolute byte offset to each record)
///   Records: bitmask u16 + frequency u32 + hanzi_len u8 + tl_len u8 + hanzi + tl
final class DictionaryBinaryReader: @unchecked Sendable {
    // MARK: - Types

    struct DictionaryRecord {
        let bitmask: UInt16
        let frequency: UInt32
        let hanzi: String?
        let tl: String
    }

    // MARK: - Constants

    private static let magic: [UInt8] = [0x54, 0x4B, 0x44, 0x42] // "TKDB"
    private static let headerSize = 16
    private static let supportedVersion: UInt32 = 1

    /// Bitmask bit positions — must match build script
    private static let bitToSource: [(bit: Int, source: DictionarySource)] = [
        (0, .kautian), (1, .taigitv), (2, .itaigi), (3, .sitbut),
        (4, .taihoa), (5, .taijit), (6, .kungge), (7, .stti),
        (8, .khpoo), (9, .khiin), (10, .dev), (11, .lkk),
    ]

    /// Named bitmask constants for filter logic
    private static let khiinBit: UInt16 = 1 << 9
    private static let devBit: UInt16 = 1 << 10
    private static let variantBit: UInt16 = 1 << 12

    // MARK: - Properties

    /// Strongly retained mmap'd data (must live as long as reads happen)
    private let data: Data
    private let basePtr: UnsafeRawPointer
    let recordCount: Int
    let buildTimestamp: UInt32

    // MARK: - Initialization

    init?(bundle: Bundle = ResourceBundleResolver.dictionaryBundle) {
        guard let url = bundle.url(forResource: "dictionary", withExtension: "bin") else {
            return nil
        }

        guard let mappedData = try? Data(contentsOf: url, options: .mappedIfSafe) else {
            return nil
        }

        guard mappedData.count >= Self.headerSize else {
            return nil
        }

        data = mappedData

        // Pin base pointer from retained Data
        basePtr = (data as NSData).bytes

        // Validate magic
        let magicBytes = UnsafeRawBufferPointer(start: basePtr, count: 4)
        guard magicBytes.elementsEqual(Self.magic) else {
            return nil
        }

        // Parse header
        let version = basePtr.loadUnaligned(fromByteOffset: 4, as: UInt32.self)
        guard version == Self.supportedVersion else {
            return nil
        }

        let count = basePtr.loadUnaligned(fromByteOffset: 8, as: UInt32.self)
        recordCount = Int(count)
        buildTimestamp = basePtr.loadUnaligned(fromByteOffset: 12, as: UInt32.self)

        // Validate file size covers header + offset table
        let minSize = Self.headerSize + recordCount * 4
        guard data.count >= minSize else {
            return nil
        }
    }

    // MARK: - Record Access

    /// Read a record by rowId (1-based)
    func record(at rowId: Int) -> DictionaryRecord? {
        guard rowId >= 1, rowId <= recordCount else {
            return nil
        }

        let offsetIndex = rowId - 1
        let offsetPos = Self.headerSize + offsetIndex * 4
        let recordOffset = Int(basePtr.loadUnaligned(fromByteOffset: offsetPos, as: UInt32.self))

        // Determine record end
        let recordEnd: Int
        if offsetIndex + 1 < recordCount {
            let nextOffsetPos = Self.headerSize + (offsetIndex + 1) * 4
            recordEnd = Int(basePtr.loadUnaligned(fromByteOffset: nextOffsetPos, as: UInt32.self))
        } else {
            recordEnd = data.count
        }

        guard recordOffset >= 0, recordEnd > recordOffset, recordEnd <= data.count else {
            return nil
        }

        // Parse record: bitmask(2) + frequency(4) + hanzi_len(1) + tl_len(1) + hanzi + tl
        let minRecordSize = 8 // 2 + 4 + 1 + 1
        guard recordEnd - recordOffset >= minRecordSize else {
            return nil
        }

        var pos = recordOffset
        let bitmask = basePtr.loadUnaligned(fromByteOffset: pos, as: UInt16.self)
        pos += 2
        let frequency = basePtr.loadUnaligned(fromByteOffset: pos, as: UInt32.self)
        pos += 4
        let hanziLen = Int(basePtr.load(fromByteOffset: pos, as: UInt8.self))
        pos += 1
        let tlLen = Int(basePtr.load(fromByteOffset: pos, as: UInt8.self))
        pos += 1

        guard pos + hanziLen + tlLen <= recordEnd else {
            return nil
        }

        let hanzi: String?
        if hanziLen > 0 {
            hanzi = String(
                bytes: UnsafeRawBufferPointer(start: basePtr + pos, count: hanziLen),
                encoding: .utf8,
            )
            pos += hanziLen
        } else {
            hanzi = nil
        }

        guard let tl = String(
            bytes: UnsafeRawBufferPointer(start: basePtr + pos, count: tlLen),
            encoding: .utf8,
        ) else {
            return nil
        }

        return DictionaryRecord(
            bitmask: bitmask,
            frequency: frequency,
            hanzi: hanzi,
            tl: tl,
        )
    }

    // MARK: - Bitmask Utilities

    /// Convert a bitmask to an array of DictionarySource
    static func sourcesFromBitmask(_ bitmask: UInt16) -> [DictionarySource] {
        bitToSource.compactMap { (bitmask & (1 << $0.bit)) != 0 ? $0.source : nil }
    }

    /// Check if a record passes the dictionary filter
    static func passesFilter(
        recordBitmask: UInt16,
        enabledDicts: EnabledDictionaries,
    ) -> Bool {
        // Layer 1: variant exclusion
        if !enabledDicts.variant, (recordBitmask & variantBit) != 0 {
            return false
        }
        // Layer 2: khiin exclusion
        if !enabledDicts.khiin, (recordBitmask & khiinBit) != 0 {
            return false
        }
        // Layer 3: source OR match (dev always included)
        if enabledDicts.allEnabled { return true }

        let enabledMask = enabledDicts.sourceBitmask()
        return (recordBitmask & enabledMask) != 0 || (recordBitmask & devBit) != 0
    }
}
