#!/bin/bash
#
# 建立 Trie 建置用的 SQLite 資料庫
#
# 輸入：output/dictionary.csv
# 輸出：output/trie.db

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

VERSION=$(date +%Y%m%d)
DICTIONARY_CSV="$OUTPUT_DIR/dictionary.csv"
DB_FILE="$OUTPUT_DIR/trie.db"

echo "=================================================="
echo "03_create_trie_db"
echo "=================================================="
echo "Input:  $DICTIONARY_CSV"
echo "Output: $DB_FILE"
echo ""

if [ ! -f "$DICTIONARY_CSV" ]; then
    echo "[ERROR] Dictionary CSV file not found: $DICTIONARY_CSV"
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
    id INTEGER PRIMARY KEY AUTOINCREMENT,
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

INSERT INTO dictionary (
    tl_num, tl_notone, tl_abbrev, poj_num, poj_notone, poj_abbrev
)
SELECT
    tl_num,
    CASE WHEN tl_notone = '' OR tl_notone IS NULL THEN NULL ELSE tl_notone END,
    CASE WHEN tl_abbrev = '' OR tl_abbrev IS NULL THEN NULL ELSE tl_abbrev END,
    CASE WHEN poj_num = '' OR poj_num IS NULL THEN NULL ELSE poj_num END,
    CASE WHEN poj_notone = '' OR poj_notone IS NULL THEN NULL ELSE poj_notone END,
    CASE WHEN poj_abbrev = '' OR poj_abbrev IS NULL THEN NULL ELSE poj_abbrev END
FROM temp_import
WHERE tl_num IS NOT NULL AND tl_num != ''
  AND (LENGTH(tl_num) - LENGTH(REPLACE(tl_num, '-', '')) + 1) <= 4;

DROP TABLE temp_import;

ANALYZE;
EOF

COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM dictionary;")

echo "[INFO] Done!"
echo "  - Version: ${VERSION}"
echo "  - Entries: ${COUNT}"
echo "  - Output: $DB_FILE"
