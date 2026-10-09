//! The Learning Records page's SQL: list one kind of learned row, correct a
//! row's count, forget a row. One shape over the two learning tables the
//! pages list (the bigram store has no page); each store describes its own
//! table with a [`Table`] beside its schema and keeps its learning, ranking
//! and eviction SQL.
//!
//! Design: `docs/architecture/learning-records-page-roadmap.md` § Engine.

use protos::engine::{LearningRecord, LearningRecordKind, LearningRecordOrder, LearningRecords};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};

use crate::custom_dictionary::escaped_for_like;
use crate::database::{
    deferred_transaction, immediate_transaction, UserDataDatabase, UserDataDatabaseError,
};
use crate::paging::{last_page_offset, saturated_u32};
use crate::UserDataStores;

/// The largest count a page can set: a bound on what a text field accepts,
/// far past any count that still changes a ranking.
const MAX_COUNT: i64 = 1_000_000;

/// One learning table as the page reads it, declared by the store that owns
/// the table.
pub(crate) struct Table {
    pub(crate) name: &'static str,
    /// The columns that say which word a row is, in [`LearningRecord`]
    /// order: text, TL.
    pub(crate) identity: [&'static str; 2],
    pub(crate) count: &'static str,
    pub(crate) last_used: &'static str,
    /// What else the store deletes with the row whose `id` this is, inside
    /// the caller's transaction.
    pub(crate) delete_dependents: Option<fn(&Connection, i64) -> rusqlite::Result<()>>,
}

impl Table {
    /// The columns [`decode`] reads. The identity columns are `NOT NULL` in
    /// both tables; a NULL count lists as 0, as does a time SQLite cannot
    /// parse.
    fn columns(&self) -> String {
        let [text, tl] = self.identity;
        format!(
            "id, {text}, {tl}, COALESCE({}, 0), COALESCE(CAST(strftime('%s', {}) AS INTEGER) * 1000, 0)",
            self.count, self.last_used,
        )
    }

    /// `WHERE` matching `?1` (an escaped `LIKE` pattern) against every
    /// identity column.
    fn filter_clause(&self) -> String {
        let terms: Vec<String> = self
            .identity
            .iter()
            .map(|column| format!("{column} LIKE ?1 ESCAPE '\\'"))
            .collect();
        format!("WHERE {}", terms.join(" OR "))
    }

    /// Ends in `id`, so two pages never share or skip a row at a tie.
    fn order_clause(&self, order: LearningRecordOrder) -> String {
        let (count, last_used) = (self.count, self.last_used);
        match order {
            LearningRecordOrder::MostUsed => {
                format!("ORDER BY {count} DESC, {last_used} DESC, id ASC")
            }
            LearningRecordOrder::MostRecent => {
                format!("ORDER BY {last_used} DESC, {count} DESC, id ASC")
            }
        }
    }

    /// The identity guard and its values: the row with `record.id`, only
    /// while it still holds the identity the page listed. `learned_phrases`
    /// reuses a deleted id, and a stale dialog must not edit the phrase
    /// that took it.
    fn guard(&self, record: &LearningRecord) -> (String, Vec<Value>) {
        let [text, tl] = self.identity;
        (
            format!("id = ? AND {text} = ? AND {tl} = ?"),
            vec![
                Value::from(record.id),
                Value::from(record.text.clone()),
                Value::from(record.tl.clone()),
            ],
        )
    }
}

fn target(
    stores: &UserDataStores,
    kind: LearningRecordKind,
) -> (&'static Table, &UserDataDatabase) {
    match kind {
        LearningRecordKind::Frequency => (
            &crate::frequency::LEARNING_TABLE,
            &stores.frequency.database,
        ),
        LearningRecordKind::LearnedPhrase => (
            &crate::learned_phrases::LEARNING_TABLE,
            &stores.learned_phrases.database,
        ),
    }
}

fn decode(kind: LearningRecordKind, row: &rusqlite::Row<'_>) -> rusqlite::Result<LearningRecord> {
    let text: String = row.get(1)?;
    Ok(LearningRecord {
        kind: kind as i32,
        id: row.get(0)?,
        can_add_to_custom_dictionary: can_add_to_custom_dictionary(&text),
        text,
        tl: row.get(2)?,
        count: row.get(3)?,
        last_used_ms: row.get(4)?,
    })
}

/// Whether Add to Custom Dictionary takes this row: one whose text holds
/// Hanji; a romanization pick has no Hanji to file. Any syllable count: a
/// one-syllable custom word (是 / sī) takes its toneless key in continuous
/// input as any custom word does, and that is the user's choice to make.
pub(crate) fn can_add_to_custom_dictionary(text: &str) -> bool {
    phonetics::api::is_hanji(text)
}

/// One page of `kind`, with the totals, read from one snapshot. `filter` is
/// matched as a substring; `limit` is at least 1. A page past the end
/// answers the last page that exists.
pub(crate) fn list(
    stores: &UserDataStores,
    kind: LearningRecordKind,
    filter: &str,
    order: LearningRecordOrder,
    limit: u32,
    offset: u32,
) -> Result<LearningRecords, UserDataDatabaseError> {
    let (table, database) = target(stores, kind);
    let filter = filter.trim();
    let pattern = (!filter.is_empty()).then(|| format!("%{}%", escaped_for_like(filter)));
    let filter_clause = pattern
        .as_ref()
        .map(|_| table.filter_clause())
        .unwrap_or_default();
    let (limit, requested) = (limit.max(1) as usize, offset as usize);
    database.perform::<_, UserDataDatabaseError>(move |connection| {
        Ok(deferred_transaction::<_, rusqlite::Error>(connection, |connection| {
            let count = |clause: &str, pattern: Option<&String>| -> rusqlite::Result<u32> {
                connection.query_row(
                    &format!("SELECT COUNT(*) FROM {} {clause};", table.name),
                    params_from_iter(pattern),
                    |row| row.get(0),
                )
            };
            // The matches and the table's rows are counted by the statement
            // that reads the page: one scan, not one per number. The bounds
            // are inlined integers, so `?1` is the pattern when there is one.
            let page = |offset: usize| -> rusqlite::Result<Vec<(LearningRecord, u32, u32)>> {
                let mut statement = connection.prepare(&format!(
                    "SELECT {columns}, COUNT(*) OVER (), (SELECT COUNT(*) FROM {name})\nFROM {name} {filter_clause} {order} LIMIT {limit} OFFSET {offset};",
                    columns = table.columns(),
                    name = table.name,
                    order = table.order_clause(order),
                ))?;
                let rows = statement.query_map(params_from_iter(pattern.as_ref()), |row| {
                    Ok((decode(kind, row)?, row.get(5)?, row.get(6)?))
                })?;
                rows.collect()
            };
            let mut offset = requested;
            let mut rows = page(offset)?;
            if rows.is_empty() && offset > 0 {
                let matching_total = count(&filter_clause, pattern.as_ref())?;
                offset = last_page_offset(matching_total as usize, limit, offset);
                rows = page(offset)?;
            }
            // No row to carry the totals: nothing matches.
            let (matching_total, total) = match rows.first() {
                Some((_, matching_total, total)) => (*matching_total, *total),
                None => (0, count("", None)?),
            };
            Ok(LearningRecords {
                records: rows.into_iter().map(|(record, ..)| record).collect(),
                total,
                matching_total,
                offset: saturated_u32(offset),
            })
        })?)
    })
}

/// Sets `record`'s count, clamped to `1..=MAX_COUNT`, and answers the row
/// as stored — `None` when the guard matches nothing. One statement: the
/// keyboard's own increments land before or after it, and the row's time is
/// not touched.
pub(crate) fn set_count(
    stores: &UserDataStores,
    kind: LearningRecordKind,
    record: &LearningRecord,
    count: i64,
) -> Result<Option<LearningRecord>, UserDataDatabaseError> {
    let (table, database) = target(stores, kind);
    let (guard, guard_values) = table.guard(record);
    let mut values = vec![Value::from(count.clamp(1, MAX_COUNT))];
    values.extend(guard_values);
    database.perform::<_, UserDataDatabaseError>(move |connection| {
        Ok(connection
            .query_row(
                &format!(
                    "UPDATE {} SET {} = ? WHERE {guard} RETURNING {};",
                    table.name,
                    table.count,
                    table.columns()
                ),
                params_from_iter(values),
                |row| decode(kind, row),
            )
            .optional()?)
    })
}

/// Forgets `record` and what its store deletes with it, in one transaction;
/// `false` when the guard matches nothing.
pub(crate) fn delete(
    stores: &UserDataStores,
    kind: LearningRecordKind,
    record: &LearningRecord,
) -> Result<bool, UserDataDatabaseError> {
    let (table, database) = target(stores, kind);
    let (guard, values) = table.guard(record);
    let id = record.id;
    database.perform::<_, UserDataDatabaseError>(move |connection| {
        immediate_transaction::<_, UserDataDatabaseError>(connection, |connection| {
            let removed = connection.execute(
                &format!("DELETE FROM {} WHERE {guard};", table.name),
                params_from_iter(values),
            )? > 0;
            if let (true, Some(delete_dependents)) = (removed, table.delete_dependents) {
                delete_dependents(connection, id)?;
            }
            Ok(removed)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JournalMode, UserDataPaths};
    use phonetics::api::derive_custom_query_key;

    const FREQUENCY: LearningRecordKind = LearningRecordKind::Frequency;
    const PHRASE: LearningRecordKind = LearningRecordKind::LearnedPhrase;
    const MOST_USED: LearningRecordOrder = LearningRecordOrder::MostUsed;
    // trace: 2026-01-01T00:00:00Z = 1767225600 s.
    const NEW_YEAR_2026_MS: i64 = 1_767_225_600_000;

    fn open(directory: &std::path::Path) -> UserDataStores {
        let stores = UserDataStores::at(UserDataPaths::in_directory(directory), JournalMode::Wal);
        stores.open_blocking();
        stores
    }

    fn filtered(
        stores: &UserDataStores,
        kind: LearningRecordKind,
        filter: &str,
    ) -> LearningRecords {
        list(stores, kind, filter, MOST_USED, 100, 0).unwrap()
    }

    fn all(stores: &UserDataStores, kind: LearningRecordKind) -> Vec<LearningRecord> {
        filtered(stores, kind, "").records
    }

    fn texts(records: &[LearningRecord]) -> Vec<&str> {
        records.iter().map(|record| record.text.as_str()).collect()
    }

    fn set(stores: &UserDataStores, record: &LearningRecord, count: i64) -> Option<LearningRecord> {
        set_count(stores, record.kind(), record, count).unwrap()
    }

    fn remove(stores: &UserDataStores, record: &LearningRecord) -> bool {
        delete(stores, record.kind(), record).unwrap()
    }

    fn record_times(stores: &UserDataStores, word: &str, tl: &str, times: usize) {
        for _ in 0..times {
            stores.frequency.record(word, tl);
        }
    }

    /// Raw SQL on one store's writer, behind its queued writes.
    fn sql<T: Send + 'static>(
        database: &UserDataDatabase,
        body: impl FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    ) -> T {
        database
            .perform::<_, UserDataDatabaseError>(move |connection| Ok(body(connection)?))
            .unwrap()
    }

    /// `CURRENT_TIMESTAMP` has one-second resolution; a test that depends
    /// on a row's time dates the rows `condition` selects itself.
    fn date_frequency_rows(stores: &UserDataStores, condition: &'static str) {
        sql(&stores.frequency.database, move |connection| {
            connection.execute(
                &format!(
                    "UPDATE user_frequency SET last_used = '2026-01-01 00:00:00' {condition};"
                ),
                [],
            )
        });
    }

    #[test]
    fn frequency_rows_list_most_used_first_and_filter_by_text_or_tl() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        record_times(&stores, "台灣", "tâi-uân", 3);
        record_times(&stores, "食飯", "tsia̍h-pn̄g", 1);
        record_times(&stores, "重", "tîng", 2);
        record_times(&stores, "重", "tāng", 5);

        let listed = filtered(&stores, FREQUENCY, "");
        assert_eq!((listed.total, listed.matching_total), (4, 4));
        // The pair is the word: 重/tāng and 重/tîng are two rows.
        let pairs: Vec<(&str, &str, i64)> = listed
            .records
            .iter()
            .map(|record| (record.text.as_str(), record.tl.as_str(), record.count))
            .collect();
        assert_eq!(
            pairs,
            [
                ("重", "tāng", 5),
                ("台灣", "tâi-uân", 3),
                ("重", "tîng", 2),
                ("食飯", "tsia̍h-pn̄g", 1)
            ]
        );
        assert!(listed
            .records
            .iter()
            .all(|record| record.last_used_ms > 0 && record.kind() == FREQUENCY));

        let by_hanji = filtered(&stores, FREQUENCY, " 重 ");
        assert_eq!((by_hanji.total, by_hanji.matching_total), (4, 2));
        assert_eq!(
            texts(&filtered(&stores, FREQUENCY, "uân").records),
            ["台灣"]
        );
        let nothing = filtered(&stores, FREQUENCY, "無");
        assert_eq!((nothing.total, nothing.matching_total), (4, 0));
    }

    #[test]
    fn a_filter_character_sqlite_treats_as_a_wildcard_matches_itself_only() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        record_times(&stores, "100%", "", 1);
        record_times(&stores, "百", "pah", 1);

        assert_eq!(texts(&filtered(&stores, FREQUENCY, "%").records), ["100%"]);
    }

    #[test]
    fn pages_do_not_share_a_row_at_a_tie_and_a_page_past_the_end_is_the_last_one() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        // Five rows, every count 1, one time: only the `id` tie-break is
        // left to order them.
        for word in ["一", "二", "三", "四", "五"] {
            record_times(&stores, word, "", 1);
        }
        date_frequency_rows(&stores, "");
        let page = |filter: &str, offset: u32| {
            list(&stores, FREQUENCY, filter, MOST_USED, 2, offset).unwrap()
        };
        let (first, second, third) = (page("", 0), page("", 2), page("", 4));
        let seen: Vec<i64> = [&first, &second, &third]
            .iter()
            .flat_map(|page| page.records.iter().map(|record| record.id))
            .collect();
        let mut ascending = seen.clone();
        ascending.sort_unstable();
        ascending.dedup();
        assert_eq!(ascending.len(), 5, "no row on two pages");
        assert_eq!(seen, ascending, "ties are broken by id");
        assert_eq!(
            (second.total, second.matching_total, second.offset),
            (5, 5, 2)
        );

        let past = page("", 40);
        assert_eq!(past.offset, 4);
        assert_eq!(past.records, third.records);
        let empty = page("無", 40);
        assert_eq!((empty.offset, empty.matching_total, empty.total), (0, 0, 5));
        assert!(empty.records.is_empty());
    }

    #[test]
    fn setting_a_count_clamps_it_keeps_the_time_and_reorders_the_list() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        record_times(&stores, "台灣", "tâi-uân", 3);
        record_times(&stores, "食飯", "tsia̍h-pn̄g", 1);
        // An old time: a set that stamped "now" could not go unnoticed.
        date_frequency_rows(&stores, "WHERE word = '食飯'");
        let rows = all(&stores, FREQUENCY);
        let meal = rows.iter().find(|record| record.text == "食飯").unwrap();
        assert_eq!(meal.last_used_ms, NEW_YEAR_2026_MS);

        let stored = set(&stores, meal, 40).expect("the stored row");
        assert_eq!(stored.count, 40);
        assert_eq!(
            stored.last_used_ms, NEW_YEAR_2026_MS,
            "an edit is not a use"
        );
        assert_eq!(texts(&all(&stores, FREQUENCY)), ["食飯", "台灣"]);

        assert_eq!(set(&stores, meal, 0).unwrap().count, 1);
        assert_eq!(set(&stores, meal, i64::MAX).unwrap().count, MAX_COUNT);
        // The keyboard keeps counting on top of what the page set.
        set(&stores, meal, 7);
        record_times(&stores, "食飯", "tsia̍h-pn̄g", 1);
        assert_eq!(filtered(&stores, FREQUENCY, "食飯").records[0].count, 8);
    }

    #[test]
    fn a_deleted_frequency_row_is_gone_and_is_learned_again_on_the_next_pick() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        record_times(&stores, "台灣", "tâi-uân", 3);
        let row = all(&stores, FREQUENCY).remove(0);

        assert!(remove(&stores, &row));
        assert!(all(&stores, FREQUENCY).is_empty());
        assert!(!remove(&stores, &row), "already gone");
        assert_eq!(set(&stores, &row, 5), None, "nothing to set");

        record_times(&stores, "台灣", "tâi-uân", 1);
        assert_eq!(all(&stores, FREQUENCY)[0].count, 1);
    }

    #[test]
    fn a_record_whose_text_no_longer_matches_its_id_is_left_alone() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        stores.learned_phrases.learn_phrase("食飯", "tsia̍h-pn̄g");
        let old = all(&stores, PHRASE).remove(0);
        assert!(remove(&stores, &old));
        // `learned_phrases.id` is a plain INTEGER PRIMARY KEY: the next
        // phrase takes the id the deleted one had.
        stores.learned_phrases.learn_phrase("啉茶", "lim-tê");
        let new = all(&stores, PHRASE).remove(0);
        assert_eq!(new.id, old.id, "trace: the id was reused");

        assert_eq!(set(&stores, &old, 9), None, "a stale dialog edits nothing");
        assert!(!remove(&stores, &old));
        assert_eq!(all(&stores, PHRASE), [new]);
    }

    #[test]
    fn a_deleted_learned_phrase_takes_its_search_keys_with_it() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        stores.learned_phrases.learn_phrase("食飯", "tsia̍h-pn̄g");
        stores.learned_phrases.learn_phrase("啉茶", "lim-tê");
        let key = derive_custom_query_key("tsiahpng", "tl").expect("a key");
        // `rows_matching` reads on the reader connection: it does not wait
        // for the queued learns as the page's own requests do.
        stores.learned_phrases.database.wait_for_queued_writes();
        assert_eq!(stores.learned_phrases.rows_matching(&key, 5).len(), 1);

        let rows = all(&stores, PHRASE);
        let meal = rows.iter().find(|record| record.text == "食飯").unwrap();
        assert_eq!(meal.tl, "tsia̍h-pn̄g");
        assert_eq!(set(&stores, meal, 12).unwrap().count, 12);
        assert!(remove(&stores, meal));

        assert!(stores.learned_phrases.rows_matching(&key, 5).is_empty());
        let orphaned: i64 = sql(&stores.learned_phrases.database, |connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM learned_search_key WHERE phrase_id NOT IN (SELECT id FROM learned_phrases);",
                [],
                |row| row.get(0),
            )
        });
        assert_eq!(orphaned, 0);
        assert_eq!(texts(&all(&stores, PHRASE)), ["啉茶"]);
    }

    #[test]
    fn a_second_handle_on_the_same_files_sees_and_edits_what_the_first_wrote() {
        let directory = tempfile::tempdir().unwrap();
        let keyboard = open(directory.path());
        let settings = open(directory.path());
        record_times(&keyboard, "台灣", "tâi-uân", 2);
        keyboard.frequency.database.wait_for_queued_writes();

        let row = all(&settings, FREQUENCY).remove(0);
        assert_eq!(set(&settings, &row, 10).unwrap().count, 10);
        record_times(&keyboard, "台灣", "tâi-uân", 1);
        keyboard.frequency.database.wait_for_queued_writes();
        let row = all(&settings, FREQUENCY).remove(0);
        assert_eq!(row.count, 11, "the pick adds to the set");

        assert!(remove(&settings, &row));
        assert!(keyboard.frequency.all_rows().unwrap().is_empty());
    }

    #[test]
    fn most_recent_first_puts_the_last_touched_row_on_top() {
        let directory = tempfile::tempdir().unwrap();
        let stores = open(directory.path());
        record_times(&stores, "舊", "kū", 5);
        record_times(&stores, "新", "sin", 1);
        date_frequency_rows(&stores, "WHERE word = '舊'");

        let recent = list(
            &stores,
            FREQUENCY,
            "",
            LearningRecordOrder::MostRecent,
            10,
            0,
        )
        .unwrap();
        assert_eq!(texts(&all(&stores, FREQUENCY)), ["舊", "新"]);
        assert_eq!(texts(&recent.records), ["新", "舊"]);
    }
}
