## v3.3.10 (develop-kk10)

### Android

#### Performance Optimization
- **Candidate display refactor**: Migrated from LinearLayout to RecyclerView + DiffUtil + ListAdapter
- **Debounce + Cancel mechanism**: Added debounce and cancellation to candidate updates to prevent redundant calculations
- **UI update performance**: Reduced from ~145ms to 4-6ms

#### Bug Fixes
- **isTranslateSwapped toggle failure**: After RecyclerView refactor, DiffUtil could not detect state changes; added `notifyDataSetChanged()`
- **Dark Mode icon visibility**: Fixed fillColor for `ic_translate`, `ic_keyboard_arrow_up`, `ic_keyboard_arrow_down`, `ic_backspace`
- **Symbol keyboard duplicate number row**: Removed redundant `number_row` extension in SYMBOLS mode
- **English spellcheck timeout**: Added 2-second timeout to prevent coroutine blocking

#### Feature Changes
- **Search syllable limit**: Increased from 3 syllables to 4
- **Removed Recent Emoji**: Deleted `EmojiHistory.kt`, `EmojiHistoryManager.kt`, and `RECENTLY_USED` category

#### New Files
- `CandidateAdapter.kt`: RecyclerView candidate adapter
- `item_candidate.xml`: Candidate item layout

### Shared

#### Dictionary Scripts
- `02_create_app_db.sh`: Syllable limit changed from `<= 3` to `<= 4`
- `03_create_trie_db.sh`: Syllable limit changed from `<= 3` to `<= 4`
