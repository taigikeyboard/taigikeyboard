package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import java.io.File
import java.io.RandomAccessFile
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.MappedByteBuffer
import java.nio.channels.FileChannel

/**
 * Read-only binary dictionary reader (mmap-based)
 *
 * Reads `dictionary.bin` — a compact binary format replacing SQLite
 * for read-only dictionary lookups. Uses memory-mapped I/O.
 *
 * Binary format (little-endian):
 *   Header: "TKDB" (4) + version u32 (4) + count u32 (4) + build_ts u32 (4)
 *   Offset table: count × u32 (absolute byte offset to each record)
 *   Records: bitmask u16 + frequency u32 + hanzi_len u8 + tl_len u8 + hanzi + tl
 *
 * IMPORTANT: All reads use absolute-position methods (getInt(offset), getShort(offset))
 * for thread safety across coroutines on Dispatchers.IO.
 */
class DictionaryBinaryReader private constructor(
    private val buffer: MappedByteBuffer,
    val recordCount: Int,
    val buildTimestamp: Long,
) {
    data class DictionaryRecord(
        val bitmask: Int,
        val frequency: Int,
        val hanzi: String?,
        val tl: String,
    )

    /** Read a record by rowId (1-based) */
    fun record(rowId: Int): DictionaryRecord? {
        if (rowId < 1 || rowId > recordCount) return null

        val offsetIndex = rowId - 1
        val offsetPos = HEADER_SIZE + offsetIndex * 4
        val recordOffset = buffer.getInt(offsetPos).toLong() and 0xFFFFFFFFL

        val recordEnd: Long =
            if (offsetIndex + 1 < recordCount) {
                val nextOffsetPos = HEADER_SIZE + (offsetIndex + 1) * 4
                buffer.getInt(nextOffsetPos).toLong() and 0xFFFFFFFFL
            } else {
                buffer.capacity().toLong()
            }

        if (recordOffset < 0 || recordEnd <= recordOffset || recordEnd > buffer.capacity()) {
            return null
        }

        val recOff = recordOffset.toInt()
        val recEnd = recordEnd.toInt()

        // Record: bitmask(2) + frequency(4) + hanzi_len(1) + tl_len(1) = 8 minimum
        if (recEnd - recOff < MIN_RECORD_SIZE) return null

        var pos = recOff
        val bitmask = buffer.getShort(pos).toInt() and 0xFFFF
        pos += 2
        val frequency = buffer.getInt(pos)
        pos += 4
        val hanziLen = buffer.get(pos).toInt() and 0xFF
        pos += 1
        val tlLen = buffer.get(pos).toInt() and 0xFF
        pos += 1

        if (pos + hanziLen + tlLen > recEnd) return null

        val hanzi: String? =
            if (hanziLen > 0) {
                val bytes = ByteArray(hanziLen)
                for (i in 0 until hanziLen) bytes[i] = buffer.get(pos + i)
                pos += hanziLen
                decodeUtf8Strict(bytes) // null on invalid UTF-8 (matches iOS)
            } else {
                null
            }

        val tlBytes = ByteArray(tlLen)
        for (i in 0 until tlLen) tlBytes[i] = buffer.get(pos + i)
        val tl = decodeUtf8Strict(tlBytes) ?: return null // reject record on invalid tl (matches iOS)

        return DictionaryRecord(
            bitmask = bitmask,
            frequency = frequency,
            hanzi = hanzi,
            tl = tl,
        )
    }

    companion object {
        private const val TAG = "DictBinaryReader"
        private val MAGIC = byteArrayOf(0x54, 0x4B, 0x44, 0x42) // "TKDB"
        private const val HEADER_SIZE = 16
        private const val SUPPORTED_VERSION = 1
        private const val MIN_RECORD_SIZE = 8 // bitmask(2) + frequency(4) + hanzi_len(1) + tl_len(1)
        /** Strict UTF-8 decode: returns null on invalid bytes (matches iOS behavior) */
        private fun decodeUtf8Strict(bytes: ByteArray): String? =
            try {
                Charsets.UTF_8.newDecoder()
                    .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
                    .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
                    .decode(java.nio.ByteBuffer.wrap(bytes))
                    .toString()
            } catch (_: java.nio.charset.CharacterCodingException) {
                null
            }

        // Bitmask bit positions
        private const val KHIIN_BIT = 1 shl 9
        private const val DEV_BIT = 1 shl 10
        private const val VARIANT_BIT = 1 shl 12

        /** Bit-to-source mapping for bitmask → DictionarySource conversion */
        private val BIT_TO_SOURCE =
            listOf(
                0 to DictionarySource.KAUTIAN,
                1 to DictionarySource.TAIGITV,
                2 to DictionarySource.ITAIGI,
                3 to DictionarySource.SITBUT,
                4 to DictionarySource.TAIHOA,
                5 to DictionarySource.TAIJIT,
                6 to DictionarySource.KUNGGE,
                7 to DictionarySource.STTI,
                8 to DictionarySource.KHPOO,
                9 to DictionarySource.KHIIN,
                10 to DictionarySource.DEV,
                11 to DictionarySource.LKK,
            )

        /** Create reader from file path */
        fun open(file: File): DictionaryBinaryReader? {
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

                val countRaw = mapped.getInt(8).toLong() and 0xFFFFFFFFL
                if (countRaw > Int.MAX_VALUE) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "Record count out of range: $countRaw")
                    return null
                }
                val count = countRaw.toInt()
                val buildTs = mapped.getInt(12).toLong() and 0xFFFFFFFFL

                // Validate file covers header + offset table
                val minSize = HEADER_SIZE.toLong() + count.toLong() * 4
                if (file.length() < minSize) {
                    if (BuildConfig.DEBUG) Log.e(TAG, "File truncated: ${file.length()} < $minSize")
                    return null
                }

                if (BuildConfig.DEBUG) {
                    Log.i(TAG, "Loaded: count=$count, buildTs=$buildTs, size=${file.length()}")
                }

                return DictionaryBinaryReader(mapped, count, buildTs)
            } catch (e: Exception) {
                if (BuildConfig.DEBUG) Log.e(TAG, "Failed to open: ${e.message}", e)
                return null
            }
        }

        /** Convert bitmask to list of DictionarySource */
        fun sourcesFromBitmask(bitmask: Int): List<DictionarySource> =
            BIT_TO_SOURCE.mapNotNull { (bit, source) ->
                if ((bitmask and (1 shl bit)) != 0) source else null
            }

        /** Check if a record passes the dictionary filter (3-layer, matches iOS) */
        fun passesFilter(
            recordBitmask: Int,
            enabledDicts: EnabledDictionaries,
        ): Boolean {
            // Layer 1: variant exclusion
            if (!enabledDicts.variant && (recordBitmask and VARIANT_BIT) != 0) return false
            // Layer 2: khiin exclusion
            if (!enabledDicts.khiin && (recordBitmask and KHIIN_BIT) != 0) return false
            // Layer 3: source OR match (dev always included)
            if (enabledDicts.allEnabled()) return true
            val enabledMask = enabledDicts.sourceBitmask()
            return (recordBitmask and enabledMask) != 0 || (recordBitmask and DEV_BIT) != 0
        }
    }
}
