// Verifies the v3.6.1 R3 custom-dictionary cross-mode SQL JOIN structure.
// Android's SQLiteDatabase is unavailable in JVM unit tests, so this exercises
// the EXACT production DDL + query strings (`CustomDictionaryService` companion
// constants) against an in-memory JDBC SQLite. The native derivation parity
// (DeriveCustomSearchKeys / DeriveCustomQueryKey producing the family/form/key
// rows) is covered by the Rust `engine/phonetics/src/custom_search.rs` tests +
// device dogfood — JVM can't load the `.so` (same constraint as the NextWord
// S11 round).

package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Test
import java.sql.Connection
import java.sql.DriverManager

class CustomDictionaryServiceCrossModeTest {
    private fun openSchema(): Connection {
        val conn = DriverManager.getConnection("jdbc:sqlite::memory:")
        conn.createStatement().use { stmt ->
            stmt.executeUpdate(
                """
                CREATE TABLE custom_dictionary (
                    id TEXT PRIMARY KEY,
                    roman TEXT NOT NULL,
                    hanzi TEXT NOT NULL,
                    notone TEXT DEFAULT '',
                    abbrev TEXT DEFAULT '',
                    roman_num TEXT DEFAULT '',
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                """.trimIndent(),
            )
            stmt.executeUpdate(CustomDictionaryService.CREATE_SEARCH_KEY_TABLE_SQL)
            stmt.executeUpdate(CustomDictionaryService.CREATE_SEARCH_KEY_LOOKUP_INDEX_SQL)
            stmt.executeUpdate(CustomDictionaryService.CREATE_SEARCH_KEY_ENTRY_INDEX_SQL)
        }
        return conn
    }

    private fun insertEntry(
        conn: Connection,
        id: String,
        roman: String,
        hanzi: String,
    ) {
        conn.prepareStatement(
            "INSERT INTO custom_dictionary (id, roman, hanzi) VALUES (?, ?, ?)",
        ).use { ps ->
            ps.setString(1, id)
            ps.setString(2, roman)
            ps.setString(3, hanzi)
            ps.executeUpdate()
        }
    }

    private fun insertSearchKey(
        conn: Connection,
        entryId: String,
        family: String,
        form: String,
        key: String,
    ) {
        conn.prepareStatement(
            "INSERT INTO custom_search_key (entry_id, family, form, key) VALUES (?, ?, ?, ?)",
        ).use { ps ->
            ps.setString(1, entryId)
            ps.setString(2, family)
            ps.setString(3, form)
            ps.setString(4, key)
            ps.executeUpdate()
        }
    }

    /** Run the production `SEARCH_SQL` and return the matched entry ids in order. */
    private fun search(
        conn: Connection,
        family: String,
        form: String,
        key: String,
        limit: Int = 50,
    ): List<String> {
        val ids = mutableListOf<String>()
        conn.prepareStatement(CustomDictionaryService.SEARCH_SQL).use { ps ->
            ps.setString(1, family)
            ps.setString(2, form)
            ps.setString(3, key)
            ps.setInt(4, limit)
            ps.executeQuery().use { rs ->
                while (rs.next()) {
                    ids.add(rs.getString(1))
                }
            }
        }
        return ids
    }

    /**
     * The 食 entry is stored once but indexed under all three families; a query
     * in ANY family (tl / poj / tps) joins back to the same entry. Pre-R3 a
     * POJ-stored entry hard-missed a TL/TPS query — this is the fix.
     */
    @Test
    fun INVARIANT_CUSTOM_DICT_CROSS_MODE_anyFamilyKeyJoinsToEntry() {
        openSchema().use { conn ->
            insertEntry(conn, "e1", "chiah", "食")
            insertSearchKey(conn, "e1", "tl", "notone", "tsiah")
            insertSearchKey(conn, "e1", "poj", "notone", "chiah")
            insertSearchKey(conn, "e1", "tps", "notone", "ㄐㄧㄚㆷ")

            // TL-family query finds the entry via the tl/notone key.
            assertEquals(listOf("e1"), search(conn, family = "tl", form = "notone", key = "tsiah"))
            // POJ-family query finds the same entry via the poj/notone key.
            assertEquals(listOf("e1"), search(conn, family = "poj", form = "notone", key = "chiah"))
            // TPS-family query finds the same entry via the tps/notone key.
            assertEquals(listOf("e1"), search(conn, family = "tps", form = "notone", key = "ㄐㄧㄚㆷ"))
            // Wrong family → no match (the key only exists under its own family).
            assertEquals(emptyList<String>(), search(conn, family = "tl", form = "notone", key = "chiah"))
        }
    }

    /**
     * `form IN (?, 'abbrev')` — an abbrev-form row is matched by a query whose
     * primary form is `notone`. A two-syllable entry's `gs` abbrev surfaces it.
     */
    @Test
    fun INVARIANT_CUSTOM_DICT_CROSS_MODE_abbrevFormAlwaysMatched() {
        openSchema().use { conn ->
            insertEntry(conn, "e2", "gâu-tsá", "𠢕早")
            insertSearchKey(conn, "e2", "tl", "notone", "gautsa")
            insertSearchKey(conn, "e2", "tl", "abbrev", "gs")

            // Primary-form (notone) query matches the abbrev row.
            assertEquals(listOf("e2"), search(conn, family = "tl", form = "notone", key = "gs"))
            // Primary-form query also still matches the notone row.
            assertEquals(listOf("e2"), search(conn, family = "tl", form = "notone", key = "gau"))
        }
    }

    /** `LIKE ? || '%'` is a prefix match; `DISTINCT` collapses multi-row joins. */
    @Test
    fun INVARIANT_CUSTOM_DICT_CROSS_MODE_prefixMatchDistinctEntry() {
        openSchema().use { conn ->
            insertEntry(conn, "e3", "tâi-gí", "台語")
            insertSearchKey(conn, "e3", "tl", "notone", "taigi")
            insertSearchKey(conn, "e3", "tl", "abbrev", "tg")

            // Prefix "tai" matches the notone key; DISTINCT keeps it a single row
            // even though both side rows belong to the same entry.
            assertEquals(listOf("e3"), search(conn, family = "tl", form = "notone", key = "tai"))
            // Non-matching prefix → empty.
            assertEquals(emptyList<String>(), search(conn, family = "tl", form = "notone", key = "xyz"))
        }
    }
}
