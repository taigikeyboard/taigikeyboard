#!/bin/bash
#
# 建立 App 使用的 SQLite 資料庫
#
# 輸入：output/dictionary.csv
# 輸出：output/dictionary.db

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

VERSION=$(date +%Y%m%d)
DICTIONARY_CSV="$OUTPUT_DIR/dictionary.csv"
DB_FILE="$OUTPUT_DIR/dictionary.db"

echo "=================================================="
echo "02_create_app_db"
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
    hanzi TEXT,
    tl TEXT NOT NULL,
    poj TEXT NOT NULL,
    frequency INTEGER,
    kautian INTEGER DEFAULT 0,
    taigitv INTEGER DEFAULT 0,
    itaigi INTEGER DEFAULT 0,
    sitbut INTEGER DEFAULT 0,
    taihoa INTEGER DEFAULT 0,
    taijit INTEGER DEFAULT 0,
    kungge INTEGER DEFAULT 0,

    UNIQUE(tl, hanzi)
);

CREATE INDEX idx_tl ON dictionary(tl);
CREATE INDEX idx_poj ON dictionary(poj);
CREATE INDEX idx_hanzi ON dictionary(hanzi);
CREATE INDEX idx_frequency ON dictionary(frequency);

EOF

sqlite3 "$DB_FILE" << EOF
.mode csv
.import ${DICTIONARY_CSV} temp_import

INSERT OR IGNORE INTO dictionary (
    hanzi, tl, poj, frequency, kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge
)
SELECT
    CASE WHEN hanzi = '' OR hanzi IS NULL THEN NULL ELSE hanzi END,
    tl, poj, frequency,
    CASE WHEN kautian = 'True' THEN 1 ELSE 0 END,
    CASE WHEN taigitv = 'True' THEN 1 ELSE 0 END,
    CASE WHEN itaigi = 'True' THEN 1 ELSE 0 END,
    CASE WHEN sitbut = 'True' THEN 1 ELSE 0 END,
    CASE WHEN taihoa = 'True' THEN 1 ELSE 0 END,
    CASE WHEN taijit = 'True' THEN 1 ELSE 0 END,
    CASE WHEN kungge = 'True' THEN 1 ELSE 0 END
FROM temp_import
WHERE tl IS NOT NULL AND tl != '';

DROP TABLE temp_import;

ANALYZE;
EOF

COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM dictionary;")

echo "[INFO] Done!"
echo "  - Version: ${VERSION}"
echo "  - Entries: ${COUNT}"
echo "  - Output: $DB_FILE"
