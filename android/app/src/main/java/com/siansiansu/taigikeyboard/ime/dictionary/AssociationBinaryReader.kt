package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import java.io.File
import java.io.RandomAccessFile
import java.nio.ByteOrder
import java.nio.MappedByteBuffer
import java.nio.channels.FileChannel

/**
 * Read-only binary association reader (mmap-based)
 *
 * Reads `association.bin` — a compact binary format replacing SQLite
 * word_association table for read-only bigram/phrase lookups.
 *
 * Binary format (little-endian):
 *   Header: "TKWA" (4) + version u32 + key_count u32 + entry_count u32 + build_ts u32
 *   Key offset table: key_count × u32 (absolute byte offset to each key entry)
 *   Key section (sorted by prev_word UTF-8):
 *     Each key: prev_word_len u8 + prev_word + entry_offset u32 + entry_count u16
 *   Entry section (sorted by count DESC per group):
 *     Each entry: bitmask u16 + count u32 + next_word_len u8 + next_tl_len u8 + next_word + next_tl
 *
 * IMPORTANT: All reads use absolute-position methods for thread safety.
 */
class AssociationBinaryReader private constructor(
    private val buffer: MappedByteBuffer,
    private val keyCount: Int,
    val buildTimestamp: Long,
) {
    data class AssociationEntry(
        val nextWord: String,
        val nextTl: String,
        val count: Int,
        val bitmask: Int,
    )

    /** Look up associations for a prev_word using binary search */
    fun lookup(
        prevWord: String,
        limit: Int = 60,
    ): List<AssociationEntry> {
        val targetBytes = prevWord.toByteArray(Charsets.UTF_8)
        if (targetBytes.isEmpty()) return emptyList()

        // Binary search on key offset table
        var lo = 0
        var hi = keyCount - 1

        while (lo <= hi) {
            val mid = lo + (hi - lo) / 2
            val cmp = compareKeyAt(mid, targetBytes)

            if (cmp == 0) {
                return readEntries(mid, limit)
            } else if (cmp < 0) {
                lo = mid + 1
            } else {
                hi = mid - 1
            }
        }

        return emptyList()
    }

    /** Compare the key at given index with target bytes */
    private fun compareKeyAt(
        index: Int,
        target: ByteArray,
    ): Int {
        val keyOffsetLong = keyOffsetAt(index)
        if (keyOffsetLong >= buffer.capacity()) return -1
        val keyOffset = keyOffsetLong.toInt() // safe after bounds check

        val keyLen = buffer.get(keyOffset).toInt() and 0xFF
        val keyStart = keyOffset + 1

        if (keyStart + keyLen > buffer.capacity()) return -1

        val cmpLen = minOf(keyLen, target.size)
        for (i in 0 until cmpLen) {
            val a = buffer.get(keyStart + i).toInt() and 0xFF
            val b = target[i].toInt() and 0xFF
            if (a != b) return a - b
        }

        return keyLen - target.size
    }

    /** Get the absolute file offset of the key entry at given index (unsigned u32) */
    private fun keyOffsetAt(index: Int): Long {
        val pos = HEADER_SIZE + index * 4
        return buffer.getInt(pos).toLong() and 0xFFFFFFFFL
    }

    /** Read entries for the key at given index */
    private fun readEntries(
        keyIndex: Int,
        limit: Int,
    ): List<AssociationEntry> {
        val keyOffsetLong = keyOffsetAt(keyIndex)
        if (keyOffsetLong >= buffer.capacity()) return emptyList()
        val keyOffset = keyOffsetLong.toInt() // safe after bounds check

        val keyLen = buffer.get(keyOffset).toInt() and 0xFF

        // After prev_word: entry_offset (u32) + entry_count (u16)
        val metaPos = keyOffset + 1 + keyLen
        if (metaPos + 6 > buffer.capacity()) return emptyList()

        val entryOffsetLong = buffer.getInt(metaPos).toLong() and 0xFFFFFFFFL
        val entryCount = buffer.getShort(metaPos + 4).toInt() and 0xFFFF

        if (entryOffsetLong > buffer.capacity()) return emptyList()
        val entryOffset = entryOffsetLong.toInt() // safe after bounds check

        val readCount = minOf(entryCount, limit)
        val entries = ArrayList<AssociationEntry>(readCount)

        var pos = entryOffset
        for (i in 0 until readCount) {
            if (pos + 8 > buffer.capacity()) break

            val bitmask = buffer.getShort(pos).toInt() and 0xFFFF
            pos += 2
            val count = buffer.getInt(pos)
            pos += 4
            val nwLen = buffer.get(pos).toInt() and 0xFF
            pos += 1
            val ntLen = buffer.get(pos).toInt() and 0xFF
            pos += 1

            if (pos + nwLen + ntLen > buffer.capacity()) break

            val nwBytes = ByteArray(nwLen)
            for (j in 0 until nwLen) nwBytes[j] = buffer.get(pos + j)
            pos += nwLen

            // Strict UTF-8 decoding: skip entry on invalid bytes (matches iOS)
            val nextWord = decodeUtf8Strict(nwBytes)
            if (nextWord == null) {
                pos += ntLen // advance past next_tl to keep pos synchronized
                continue
            }

            val ntBytes = ByteArray(ntLen)
            for (j in 0 until ntLen) ntBytes[j] = buffer.get(pos + j)
            pos += ntLen

            val nextTl = decodeUtf8Strict(ntBytes) ?: continue

            entries.add(
                AssociationEntry(
                    nextWord = nextWord,
                    nextTl = nextTl,
                    count = count,
                    bitmask = bitmask,
                ),
            )
        }

        return entries
    }

    companion object {
        private const val TAG = "AssocBinaryReader"
        private val MAGIC = byteArrayOf(0x54, 0x4B, 0x57, 0x41) // "TKWA"
        private const val HEADER_SIZE = 20
        private const val SUPPORTED_VERSION = 1

        /** Strict UTF-8 decode: returns null on invalid bytes (matches iOS behavior) */
        private fun decodeUtf8Strict(bytes: ByteArray): String? =
            try {
                Charsets.UTF_8
                    .newDecoder()
                    .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
                    .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
                    .decode(java.nio.ByteBuffer.wrap(bytes))
                    .toString()
            } catch (_: java.nio.charset.CharacterCodingException) {
                null
            }

        /** Create reader from file path */
        fun open(file: File): AssociationBinaryReader? {
            if (!file.exists() || file.length() < HEADER_SIZE) {
                if (BuildConfig.DEBUG) Log.e(TAG, "File not found or too small: ${file.absolutePath}")
                return null
            }

            try {
                val raf = RandomAccessFile(file, "r")
                val channel = raf.channel
                val mapped = channel.map(FileChannel.MapMode.READ_ONLY, 0, file.length())
                mapped.order(ByteOrder.LITTLE_ENDIAN)
                channel.close()
                raf.close()

                // Validate magic
                for (i in MAGIC.indices) {
                    if (mapped.get(i) != MAGIC[i]) {
                        if (BuildConfig.DEBUG) Log.e(TAG, "Magic mismatch at byte $i")
                        return null
                    }
                }

                val version = mapped.getInt(4)
                if (version != SUPPORTED_VERSION) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "Unsupported version: $version")
                    return null
                }

                val kcRaw = mapped.getInt(8).toLong() and 0xFFFFFFFFL
                if (kcRaw > Int.MAX_VALUE) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "Key count out of range: $kcRaw")
                    return null
                }
                val kc = kcRaw.toInt()
                // entry_count at offset 12 (not needed at runtime)
                val buildTs = mapped.getInt(16).toLong() and 0xFFFFFFFFL

                val minSize = HEADER_SIZE.toLong() + kc.toLong() * 4
                if (file.length() < minSize) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "File truncated: ${file.length()} < $minSize")
                    return null
                }

                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "Loaded: keyCount=$kc, buildTs=$buildTs, size=${file.length()}")
                }

                return AssociationBinaryReader(mapped, kc, buildTs)
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "Failed to open: ${e.message}", e)
                return null
            }
        }

        /**
         * Check if an association entry passes the dictionary source filter.
         *
         * CROSS-PLATFORM INVARIANT — must stay in sync with
         * iOS `AssociationBinaryReader.passesFilter()`. Deliberately different
         * from `DictionaryBinaryReader.passesFilter` (no variant/khiin/dev
         * layers — those bits do not exist in association entries). See
         * `docs/engine/binary-format.md` §4.3.
         */
        fun passesFilter(
            entryBitmask: Int,
            enabledDicts: EnabledDictionaries,
        ): Boolean {
            if (enabledDicts.allAssociationSourcesEnabled()) return true
            val enabledMask = enabledDicts.associationBitmask()
            if (enabledMask == 0) return false
            return (entryBitmask and enabledMask) != 0
        }
    }
}
