# Learning Records page — roadmap

> **Status**: P0–P6 complete — P0 merged `5bab5b9b`; P1 merged #366 `1716751a`; P2 merged #369 `21f1270a`; P3 + P4 merged #370 `7aacb3e2`; P5 + P6 merged #371 `102c6aa8` (2026-10-03). P7 merged #415 `2988236a` (2026-10-06); P8 (Add to Custom Dictionary for word-frequency rows) merged #422 `88d5d157` (2026-10-06). Device dogfood pending. Requested by the maintainer 2026-10-03: "a page where users can view and edit learning records and ranking scores, for mobile and desktop".

A settings page, on all five platforms, that lists what the keyboard has learned from the user, lets them correct one row's count and delete one row. Today the only control is "Delete Learning Records", which empties every learning store at once.

## Today (grounded in code)

| Fact | Source |
|---|---|
| Three learning stores, engine-owned: word frequency `(word, tl, count, last_used)` cap 20 000; next-word association `(prev_word, prev_tl, next_word, next_tl, count, last_used)` cap 50 000; learned phrases `(hanzi, roman, learn_count, updated_at)` cap 2 000 | `engine/userdata/src/frequency.rs:13-50`, `association.rs:13-64`, `learned_phrases.rs:14-48` |
| A frequency row feeds ranking as `(boost − 1) × decay`: `boost = min(1 + count × 0.1, 5.0)` (saturates at count 40), decay over `last_used` with τ = 30 days. An internal weight, not a number any UI shows | `engine/ranking/src/score.rs:39-47`, `:196`, `:218`, `:258-269` |
| An association row's count has no such ceiling and its decay has a floor, so a raised count on an old row still ranks | `engine/nextword/src/scorer.rs:25-32`, `:63-71` |
| A learned phrase's `learn_count` decides which five rows a whole-buffer match returns and which row is evicted first; it is not handed to candidate ranking | `learned_phrases.rs:156`, `:232-247`, `engine/dispatch/src/user_data/with_stores.rs:274-278` |
| Frequency and association eviction also order by count, then time | `engine/userdata/src/capacity.rs:81` |
| No request lists, edits or deletes ONE learning row: the stores expose `all_rows` (tests / backup) and `delete_all` only; `UserDataRequest` has 11 methods, the page ones all custom-dictionary | `engine/protos/proto/user_data.proto:17-29`, `engine/userdata/src/requests.rs:44-86` |
| "Delete Learning Records" sends `ResetUserData {frequency, association, learned_phrases}`; before P9 it was a row on each Custom Dictionary page, since P9 it is on this page (Design) | desktop `settings/learning_records.rs` `clear_all_job`, macOS `LearningRecordsPage.swift` `clearAll`, iOS `LearningRecordsMenuView.swift`, Android `LearningRecordsMenuScreen.kt` |
| Custom Dictionary page shape per platform — the style this page copies: iOS / Android load every row and show at most 100, bottom search bar, swipe / icon delete, alert / dialog edit; macOS `Form` + paged `Table` (10 rows) + `UserDataPageChrome`; Windows / Linux share `desktop-core` `settings::custom_dictionary::Listing` (paged, settled filter, one job slot) | iOS `CustomDictionaryView.swift:6-240`, Android `CustomDictionaryScreen.kt:86-441`, macOS `CustomDictionaryPage.swift:21-442` + `UserDataPageChrome.swift`, `desktop/crates/taigi-desktop-core/src/settings/custom_dictionary.rs:18-209`, Windows `pages/custom_dictionary.rs`, Linux `pages/custom_dictionary.rs` |
| The desktop has no next-word prediction; its association store is write-only | `association.rs:46-48` |

## Design

### What the user gets

One page, **Learning Records**, with a kind switch:

| Kind | Row shows | Editable | Platforms |
|---|---|---|---|
| Word frequency (the ranking score) | Hanji / word, TL reading, count, last used | count; delete row; Add to Custom Dictionary (copies it, P8) | all five |
| Learned phrases | Hanji, TL reading, count | count; delete row; Add to Custom Dictionary (moves it, P7) | all five |

- **The count is what the user edits.** The engine's boost / decay maths stays where it is. For word frequency only, the desktop edit dialog says that counts above 40 rank the same; learned phrases make no such promise, and mobile shows no such note.
- **Search** filters by Hanji or romanization substring, in SQL, like the Custom Dictionary filter.
- **Order**: most used first (default) or most recent first — "the word I just picked by mistake" is the row a user comes to delete.
- **No next-word association.** Planned for mobile, dropped at dogfood (maintainer 2026-10-03: "word-association records are not needed"); the engine kind stays, no page lists it.
- **No add by hand.** A word the user wants is a custom word; this page corrects what was learned.
- **Add to Custom Dictionary** files one row's word in the custom dictionary. A learned phrase **moves** (P7 — Discord discussion "add a learned phrase to the custom dictionary", 2026-10-05; maintainer 2026-10-06: "word frequency does not move, learned phrases move"). A word-frequency row is **copied** (P8 — maintainer 2026-10-06: word frequency should also be addable to the custom dictionary one row at a time, like phrases, on mobile and desktop): the row stays, because it keeps weighting its word (`user_weight` leads the candidate sort, `engine/ranking/src/sort_key.rs:64`) and deleting it would demote the word it counts.
- **Which rows can be added**: text with Hanji and a TL of two syllables or more (`learning_records::can_add_to_custom_dictionary`; the engine sets `LearningRecord.can_add_to_custom_dictionary` on every row it answers and every page shows the verb from it). A custom word overrides the walker edge of its toneless key (`engine/composing/src/continuous.rs` custom edge override), so a one-tap add of a one-syllable word (是 / sī) would take every toneless `si`; a romanization pick (`RecordUsage` with no Hanji) has no Hanji to file. A one-syllable custom word stays a deliberate entry on the Custom Dictionary page.
- **"Delete Learning Records" lives on this page** (P9 — maintainer 2026-10-06: move it here, one button for every kind, desktop and mobile). One button, not one per kind: it empties every learning store, the next-word association no page lists included, so a per-kind wipe would leave the association orphaned, and a desktop button beside the kind picker would read as "delete this kind". Desktop: a destructive row under the list; mobile: a destructive row under the two kind rows on the Learning Records subpage, the kind rows off while it runs. It asks first, title plus a line saying the custom words stay (`clearLearningRecordsMessage`). The Custom Dictionary page keeps only Delete All Custom Words: user-authored words are not re-learned, so the two wipes are never merged.

### Engine (one implementation, every platform)

New `UserDataRequest` methods (tags 12–14), answered by `engine/userdata`:

```proto
enum LearningRecordKind { FREQUENCY = 0; LEARNED_PHRASE = 1; ASSOCIATION = 2; }
enum LearningRecordOrder { MOST_USED = 0; MOST_RECENT = 1; }

message LearningRecord {
  LearningRecordKind kind = 1;
  int64 id = 2;              // the store's row id — a page handle, not identity
  string text = 3;           // word / phrase Hanji / next word
  string tl = 4;             // canonical TL of `text`
  string previous_text = 5;  // association only
  string previous_tl = 6;    // association only
  int64 count = 7;
  int64 last_used_ms = 8;
}
message ListLearningRecords { kind, filter, limit, offset, order }   → LearningRecords { records, total, matching_total, offset }
message SetLearningRecordCount { record, count }                     → LearningRecordSaved { optional record }
message DeleteLearningRecord { record }                              → LearningRecordDeleted { removed }
```

- Paging contract = `ListCustomEntries`: the offset is pulled back to the last page that exists (`engine/userdata/src/paging.rs`, shared). Unlike it, `total`, `matching_total`, the offset clamp and the rows are read in ONE read transaction (the page statement counts both itself: one scan per page), `limit` 0 is refused (50 000 rows never travel in one answer), and every order ends in `id` so two pages never share or skip a row at a tie.
- **Identity guard.** A mutation carries the whole `LearningRecord` the page listed; the SQL matches `id` AND the row's identity columns (`(word, tl)`; `(hanzi, roman)`; both association pairs). `learned_phrases.id` is a plain `INTEGER PRIMARY KEY`, so a deleted id can be reused — a stale dialog must not edit the phrase that took it. No match (evicted, deleted, reused) answers "no record" / `removed = false`, which the page reports as "this record is gone" and reloads.
- `SetLearningRecordCount` clamps to `1..=1 000 000` (the learned-phrase ceiling, `learned_phrases.rs:43`) and keeps `last_used` — an edit is not a use. One `UPDATE … SET count = ? … RETURNING` statement.
- **Concurrent keyboard writes**: each process has its own writer queue (`database.rs:358-385`), so the order is the database's commit order — a set overwrites the increments before it, later picks add to it, and a deleted row is learned again on the next pick. No cross-process flush protocol.
- Deleting a learned phrase deletes its search keys in the same transaction. No `VACUUM` per row.
- The filter escapes `%` / `_` / `\` as the custom dictionary's does (`custom_dictionary.rs:520-524`); a NULL association TL lists as `''` and the guard compares `COALESCE(tl, '')`.
- Word identity stays the `(Hanji, canonical TL)` pair (Core Principle #6): the row id only addresses a row the page already listed; no lookup, dedup or merge keys on it.
- Unknown `kind` / `order` values are refused (`FAIL_INVARIANT`, as every refused user-data request).
- Backup format unchanged.
- **Add (P7, tag 15; renamed and widened to frequency rows in P8)** — `AddLearningRecordToCustomDictionary { record }` → `LearningRecordAddedToCustomDictionary { refusal, detail }`. A row `can_add_to_custom_dictionary` refuses is `FAIL_INVARIANT` (decided again from `text` / `tl`, never from the request's flag). `CustomDictionaryStore::add_unless_stored` adds `(roman = record.tl, hanji = record.text)` unless an entry with that Hanji already reads the same once both romans fold to canonical TL (`canonical_tl_form(_, Tl)` — a stored POJ `góa / 我` is `guá / 我`; Core Principle #6); check, cap and write share one transaction, and a stored word is no refusal even at the cap. Then a learned phrase is deleted as `DeleteLearningRecord` deletes it (a row already gone is not a failure); a frequency row is left as it is — count, last used and id unchanged. Two files, no shared transaction: a refused or failed add keeps the phrase; a delete that fails after the add is a store error, and the retry finds the word stored and deletes the phrase. After a phrase's move the custom word offers the whole phrase, as the learned phrase did (`lexicon/src/continuous/candidate.rs:275`).

### Platforms

| Platform | Entry | Page |
|---|---|---|
| iOS | Dictionary tab → "Keyboard Data": one Learning Records row after Custom Dictionary, opening a subpage with two rows, Word Frequency and Phrases (maintainer dogfood 2026-10-03: one item per kind, no kind picker; 2026-10-05: group them under Learning Records, matching desktop) | `LearningRecordsMenuView` → `LearningRecordsView(kind:)` + view model over `UserDataClient`; order menu row, list, bottom `SearchBar`, swipe delete, alert edit — the Custom Dictionary idiom, but engine-paged (100 rows, load more at the end; re-reads in ≤100-row chunks) with the filter sent to the engine |
| Android | Dictionary settings → "Keyboard Data": the same Learning Records row and subpage | `LearningRecordsMenuActivity` / `Screen` → `LearningRecordsActivity` (kind as intent extra) / `Screen` / `ViewModel`; order card, `FilterSearchBar`, dialog edit; same engine paging as iOS |
| macOS | sidebar pane after Custom Dictionary (`SettingsSplitView.swift:13`) | `LearningRecordsPage` on `UserDataPageChrome` (`UserDataFilterField`, `Table`, `UserDataListPager`) |
| Windows / Linux | sidebar pane after Custom Dictionary (`settings/choices.rs:316`) | shared model `desktop-core/src/settings/learning_records.rs` (reusing the `Listing` paging / settle logic) + one page file per toolkit |

Every page: a change of kind, order or filter invalidates a load still in flight; "record is gone", "could not read" and "nothing learned yet" are three different states.

### Relation to macOS-over-desktop-core

That track completed 2026-10-03 (P15 merged #365); P1 was rebased onto it. This work leaves the macOS key path, `taigi-macos-ffi` and `CoreComposingBackend` alone — the macOS page uses the Swift user-data client the Custom Dictionary page already uses (the user-data slice is the one macOS still sends itself).

### Desktop shared model (P2)

- `desktop-core/src/settings/listing.rs` — the paged, filtered list (`Listing<Row: ListedRow>`, `LoadRequest`, `JobOutcome`, page size 10, filter settle, overlay delay), lifted out of `custom_dictionary.rs` so Custom Dictionary and Learning Records share it; `custom_dictionary::Listing` is now `Listing<CustomDictionaryEntry>`.
- `desktop-core/src/settings/learning_records.rs` — kinds (frequency, phrases), orders, labels, the count note, the jobs (a row already gone is a notice, not a failure), the last-used day label.
- `SettingsPane::LearningRecords` (`learningRecords`) exists from P2; it joins `SettingsPane::SIDEBAR` with the Windows page in P3 (the Windows window asserts its pane table equals that roster). Linux lists it from P2 (its own `SIDEBAR`).
- Desktop page: kind + order pickers over the Custom Dictionary table shape (filter, 4 columns — reading, word, count, last used — ✎ / − verbs, pager). Edit = count dialog; delete = immediate, as one custom word's delete is.

### Strings

`learningRecords*` + `learningRecordGone` keys in `i18n/dictionary.json`; `learningRecordsAssociation` and `learningRecordsInfo` were deleted at dogfood (#371, #370). Taigi TL / POJ verified with `taigi-converter` (phonetics-specialist, 2026-10-03). Reused: `searchPlaceholder`, `romanLabel`, `hanziLabel`, `noResults`, `common.delete` / `save` / `cancel`, `desktop.entriesSection`.

## Phases

| Phase | Scope | Status |
|---|---|---|
| P0 | this roadmap | Done |
| P1 | engine: proto + store methods + `learning_records.rs` + routing; store tests (id reuse, two connections, phrase keys, paging, NULL TL) + dispatch tests; regenerated Android Java / iOS Swift / macOS Swift protos in the same PR | Merged #366 `1716751a` |
| P2 | i18n keys with every generated output (incl. `ios/Localizable.xcstrings`) + `desktop-core` page model (+ shared `Listing`) + Linux page; gate = every platform in the keys' scope | Merged #369 `21f1270a` |
| P3 | Windows page + `SettingsPane::SIDEBAR` entry | Merged #370 `7aacb3e2` |
| P4 | macOS page | Merged #370 `7aacb3e2` |
| P5 | iOS page | Merged #371 `102c6aa8` |
| P6 | Android page | Merged #371 `102c6aa8` |
| P7 | Add to Custom Dictionary for learned phrases: engine op + store method, protos, two i18n keys, desktop-core job, a verb on the Phrases list of all five pages (macOS context menu + button beside `−`, iOS leading swipe, Android row icon, Windows / Linux third verb) | Merged #415 `2988236a` |
| P9 | Delete Learning Records moves from Custom Dictionary to this page on all five platforms; confirm body `clearLearningRecordsMessage`; desktop-core `Confirm` enum → `presentation::Confirmation` + `custom_dictionary::DELETE_ALL` / `learning_records::CLEAR_ALL`, `clear_learning_records_job` → `learning_records::clear_all_job` (reloads the list) | In progress |
| P8 | Add to Custom Dictionary for word-frequency rows (copy): tag 15 renamed `Move…` → `AddLearningRecordToCustomDictionary` / `LearningRecordAddedToCustomDictionary` (wire unchanged), `LearningRecord.can_add_to_custom_dictionary` (field 9), `is_hanji` moved `lexicon` → `phonetics` so `userdata` reads it, i18n keys `learningRecordsMove…` / `…Moved…` → `…Add…` / `…Added…` (values unchanged); every page shows the verb on both kinds from the row flag (desktop: the verb is greyed out for a row that cannot be added) | Merged #422 `88d5d157` |

## Best practices alignment

Per phase: P1 — `docs/contributing/rust-migration-policy.md` §6 (user data is engine-owned), AGENTS.md "Shared logic lives in the engine", Core Principle #6; P2–P6 — each platform's guide in `docs/contributing/`, `docs/contributing/i18n.md`, `docs/contributing/cross-platform-alignment.md`.

| Mainstream practice | Source | This plan |
|---|---|---|
| Auto-learned data is kept apart from the user's own words | `moe_taigi_apk` UserVoc / LearnedVoc (`docs/references/mainstream-ime-comparison.md:89`, `:305`) | own page beside Custom Dictionary, never mixed into it (all phases) |
| A learned entry carries a count and a last-used time; rank = count with time decay | `references/librime/src/rime/dict/user_dictionary.cc` `c= d= t=` (`mainstream-ime-comparison.md:80`, `:87`) | the page shows and edits exactly those two stored facts; the formula is not duplicated in any UI (P1) |
| Bounded learning store, fewest-selections-then-least-recent eviction | ChiaKey `Manjusri/Headers/LanguageModel.h` (`mainstream-ime-comparison.md:90`) | a count edit keeps `last_used`, so eviction order only moves through the count the user set (P1) |

**Deliberately not adopted**

- A suppression / blocklist dictionary (mozc `user_dictionary.cc`, `mainstream-ime-comparison.md:364`): deleting the learned row is enough to undo a mistaken pick; a "never show" list is a different feature.
- Showing the computed boost: it changes with the clock (30-day decay); a number that drifts while the page is open reads as a bug. Count + last used are the stable inputs.
- Adding rows by hand: that is the custom dictionary.
- Deleting a word-frequency row once its word is added (a move, as for phrases): see What the user gets — the frequency row is the word's ranking weight.
- A second op for frequency rows beside the phrase move: one user verb, one op; what happens to the row after the add is the engine's per-kind decision.
- A generic "user-data table browser" abstraction over all four stores: three row shapes, one flat message — no trait.
