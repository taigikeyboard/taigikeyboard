import Foundation

/// Read-only binary association reader (mmap-based)
///
/// Reads `association.bin` — a compact binary format replacing SQLite
/// word_association table for read-only bigram/phrase lookups.
///
/// Binary format (little-endian):
///   Header: "TKWA" (4) + version u32 + key_count u32 + entry_count u32 + build_ts u32
///   Key offset table: key_count × u32 (absolute byte offset to each key entry)
///   Key section (sorted by prev_word UTF-8):
///     Each key: prev_word_len u8 + prev_word + entry_offset u32 + entry_count u16
///   Entry section (sorted by count DESC per group):
///     Each entry: bitmask u16 + count u32 + next_word_len u8 + next_tl_len u8 + next_word + next_tl
final class AssociationBinaryReader: @unchecked Sendable {
    // MARK: - Types

    struct AssociationEntry {
        let nextWord: String
        let nextTl: String
        let count: Int
        let bitmask: UInt16
    }

    // MARK: - Constants

    private static let magic: [UInt8] = [0x54, 0x4B, 0x57, 0x41] // "TKWA"
    private static let headerSize = 20
    private static let supportedVersion: UInt32 = 1

    // MARK: - Properties

    private let data: Data
    private let basePtr: UnsafeRawPointer
    private let keyCount: Int
    let buildTimestamp: UInt32

    // MARK: - Initialization

    init?(bundle: Bundle = ResourceBundleResolver.dictionaryBundle) {
        guard let url = bundle.url(forResource: "association", withExtension: "bin") else {
            return nil
        }

        guard let mappedData = try? Data(contentsOf: url, options: .mappedIfSafe) else {
            return nil
        }

        guard mappedData.count >= Self.headerSize else {
            return nil
        }

        data = mappedData
        basePtr = (data as NSData).bytes

        // Validate magic
        let magicBytes = UnsafeRawBufferPointer(start: basePtr, count: 4)
        guard magicBytes.elementsEqual(Self.magic) else {
            return nil
        }

        let version = basePtr.loadUnaligned(fromByteOffset: 4, as: UInt32.self)
        guard version == Self.supportedVersion else {
            return nil
        }

        let kc = basePtr.loadUnaligned(fromByteOffset: 8, as: UInt32.self)
        keyCount = Int(kc)
        // entry_count at offset 12 (not needed at runtime)
        buildTimestamp = basePtr.loadUnaligned(fromByteOffset: 16, as: UInt32.self)

        let minSize = Self.headerSize + keyCount * 4
        guard data.count >= minSize else {
            return nil
        }
    }

    // MARK: - Lookup

    /// Look up associations for a prev_word using binary search
    func lookup(prevWord: String, limit: Int = 60) -> [AssociationEntry] {
        let targetBytes = Array(prevWord.utf8)
        guard !targetBytes.isEmpty else { return [] }

        // Binary search on key offset table
        var lo = 0
        var hi = keyCount - 1

        while lo <= hi {
            let mid = lo + (hi - lo) / 2
            let cmp = compareKeyAt(index: mid, with: targetBytes)

            if cmp == 0 {
                // Found — read entries
                return readEntries(atKeyIndex: mid, limit: limit)
            } else if cmp < 0 {
                lo = mid + 1
            } else {
                hi = mid - 1
            }
        }

        return [] // Not found
    }

    // MARK: - Private Methods

    /// Compare the key at given index with target bytes
    /// Returns: negative if key < target, 0 if equal, positive if key > target
    private func compareKeyAt(index: Int, with target: [UInt8]) -> Int {
        let keyOffset = keyOffsetAt(index)
        guard keyOffset >= 0, keyOffset < data.count else { return -1 }

        let keyLen = Int(basePtr.load(fromByteOffset: keyOffset, as: UInt8.self))
        let keyStart = keyOffset + 1

        guard keyStart + keyLen <= data.count else { return -1 }

        // Compare byte by byte
        let cmpLen = min(keyLen, target.count)
        for i in 0 ..< cmpLen {
            let a = Int(basePtr.load(fromByteOffset: keyStart + i, as: UInt8.self))
            let b = Int(target[i])
            if a != b { return a - b }
        }

        // Lengths differ
        return keyLen - target.count
    }

    /// Get the absolute file offset of the key entry at given index
    private func keyOffsetAt(_ index: Int) -> Int {
        let pos = Self.headerSize + index * 4
        return Int(basePtr.loadUnaligned(fromByteOffset: pos, as: UInt32.self))
    }

    /// Read entries for the key at given index
    private func readEntries(atKeyIndex index: Int, limit: Int) -> [AssociationEntry] {
        let keyOffset = keyOffsetAt(index)
        guard keyOffset >= 0, keyOffset < data.count else { return [] }

        let keyLen = Int(basePtr.load(fromByteOffset: keyOffset, as: UInt8.self))

        // After prev_word: entry_offset (u32) + entry_count (u16)
        let metaPos = keyOffset + 1 + keyLen
        guard metaPos + 6 <= data.count else { return [] }

        let entryOffset = Int(basePtr.loadUnaligned(fromByteOffset: metaPos, as: UInt32.self))
        let entryCount = Int(basePtr.loadUnaligned(fromByteOffset: metaPos + 4, as: UInt16.self))

        guard entryOffset >= 0, entryOffset <= data.count else { return [] }

        let readCount = min(entryCount, limit)
        var entries: [AssociationEntry] = []
        entries.reserveCapacity(readCount)

        var pos = entryOffset
        for _ in 0 ..< readCount {
            guard pos + 8 <= data.count else { break }

            let bitmask = basePtr.loadUnaligned(fromByteOffset: pos, as: UInt16.self)
            pos += 2
            let count = basePtr.loadUnaligned(fromByteOffset: pos, as: UInt32.self)
            pos += 4
            let nwLen = Int(basePtr.load(fromByteOffset: pos, as: UInt8.self))
            pos += 1
            let ntLen = Int(basePtr.load(fromByteOffset: pos, as: UInt8.self))
            pos += 1

            guard pos + nwLen + ntLen <= data.count else { break }

            let nextWord = String(
                bytes: UnsafeRawBufferPointer(start: basePtr + pos, count: nwLen),
                encoding: .utf8,
            ) ?? ""
            pos += nwLen

            let nextTl = String(
                bytes: UnsafeRawBufferPointer(start: basePtr + pos, count: ntLen),
                encoding: .utf8,
            ) ?? ""
            pos += ntLen

            entries.append(AssociationEntry(
                nextWord: nextWord,
                nextTl: nextTl,
                count: Int(count),
                bitmask: bitmask,
            ))
        }

        return entries
    }

    // MARK: - Association Filter

    /// Check if an association entry passes the dictionary source filter
    /// Association bitmask uses 9 bits (kautian..khpoo), same order as dictionary bits 0-8
    static func passesFilter(entryBitmask: UInt16, enabledMask: UInt16, allEnabled: Bool) -> Bool {
        if allEnabled { return true }
        if enabledMask == 0 { return false }
        return (entryBitmask & enabledMask) != 0
    }
}
