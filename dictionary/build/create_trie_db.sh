#!/bin/bash
#
# 建立 fst 前綴索引建置用的 SQLite 資料庫
#
# 輸入：output/dictionary.csv, output/dictionary.db
# 輸出：output/trie.db (取名沿用；create_fst.py 用此檔產生 dictionary.fst)
#
# 重要：trie.db 的 id 必須與 dictionary.db 的 id 完全一致，
# 因為 fst 前綴索引儲存的 rowid 會用來查詢 dictionary.db。
# 透過 ATTACH dictionary.db 並 JOIN 取得正確的 id。

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

VERSION=$(date +%Y%m%d)
DICTIONARY_CSV="$OUTPUT_DIR/dictionary.csv"
DICT_DB_FILE="$OUTPUT_DIR/dictionary.db"
DB_FILE="$OUTPUT_DIR/trie.db"

echo "=================================================="
echo "create_trie_db"
echo "=================================================="
echo "Input:  $DICTIONARY_CSV"
echo "        $DICT_DB_FILE"
echo "Output: $DB_FILE"
echo ""

if [ ! -f "$DICTIONARY_CSV" ]; then
    echo "[ERROR] Dictionary CSV file not found: $DICTIONARY_CSV"
    exit 1
fi

if [ ! -f "$DICT_DB_FILE" ]; then
    echo "[ERROR] dictionary.db not found: $DICT_DB_FILE"
    echo "  Please run create_app_db.sh first."
    exit 1
fi

if [ -f "$DB_FILE" ]; then
    rm "$DB_FILE"
fi

sqlite3 "$DB_FILE" << EOF
PRAGMA journal_mode = DELETE;
PRAGMA synchronous = NORMAL;

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT
);

INSERT INTO metadata (key, value) VALUES
    ('version', '${VERSION}'),
    ('build_date', datetime('now'));

CREATE TABLE dictionary (
    id INTEGER PRIMARY KEY,
    tl_num TEXT NOT NULL,
    tl_notone TEXT,
    tl_abbrev TEXT,
    poj_num TEXT,
    poj_notone TEXT,
    poj_abbrev TEXT
);

EOF

sqlite3 "$DB_FILE" << EOF
.mode csv
.import ${DICTIONARY_CSV} temp_import

ATTACH DATABASE '${DICT_DB_FILE}' AS dict;

INSERT INTO dictionary (
    id, tl_num, tl_notone, tl_abbrev, poj_num, poj_notone, poj_abbrev
)
SELECT
    d.id,
    t.tl_num,
    CASE WHEN t.tl_notone = '' OR t.tl_notone IS NULL THEN NULL ELSE t.tl_notone END,
    CASE WHEN t.tl_abbrev = '' OR t.tl_abbrev IS NULL THEN NULL ELSE t.tl_abbrev END,
    CASE WHEN t.poj_num = '' OR t.poj_num IS NULL THEN NULL ELSE t.poj_num END,
    CASE WHEN t.poj_notone = '' OR t.poj_notone IS NULL THEN NULL ELSE t.poj_notone END,
    CASE WHEN t.poj_abbrev = '' OR t.poj_abbrev IS NULL THEN NULL ELSE t.poj_abbrev END
FROM temp_import t
JOIN dict.dictionary d
    ON d.tl = t.tl
    AND (d.hanzi = t.hanzi OR (d.hanzi IS NULL AND (t.hanzi = '' OR t.hanzi IS NULL)))
WHERE t.tl_num IS NOT NULL AND t.tl_num != ''
  AND (LENGTH(t.tl_num) - LENGTH(REPLACE(t.tl_num, '-', '')) + 1) <= 4
GROUP BY d.id;

DETACH DATABASE dict;

DROP TABLE temp_import;

ANALYZE;
EOF

# 驗證 trie.db 與 dictionary.db 筆數一致
DICT_COUNT=$(sqlite3 "$DICT_DB_FILE" "SELECT COUNT(*) FROM dictionary;")
TRIE_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM dictionary;")

echo "[INFO] Done!"
echo "  - Version: ${VERSION}"
echo "  - Entries: ${TRIE_COUNT} (dictionary.db: ${DICT_COUNT})"
echo "  - Output: $DB_FILE"

if [ "$DICT_COUNT" != "$TRIE_COUNT" ]; then
    echo ""
    echo "[WARNING] Row count mismatch! dictionary.db=${DICT_COUNT} trie.db=${TRIE_COUNT}"
    echo "  Some dictionary entries may not be searchable via trie."
fi
