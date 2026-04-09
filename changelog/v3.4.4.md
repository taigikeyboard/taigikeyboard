## v3.4.4 (rel-v3.4.2)

### Dictionary

#### Bug Fixes
- **Trie/dictionary ID mismatch**: Fixed `03_create_trie_db.sh` — trie.db was built independently from dictionary.db with no deduplication (`INSERT` vs `INSERT OR IGNORE`), causing 1,847 extra entries and divergent AUTOINCREMENT IDs. MARISA trie stored trie.db rowids, but runtime lookups used dictionary.db, returning wrong entries for any ID past the first duplicate. Fixed by `ATTACH`ing dictionary.db and `JOIN`ing to use correct IDs. Affects both iOS and Android.

### Shared
- Updated `dictionary.trie` on both platforms with corrected row IDs
