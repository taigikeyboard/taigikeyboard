#!/bin/bash

set -e

get_app_version() {
    local version_file="VERSION"

    if [ -f "$version_file" ]; then
        local version=$(cat "$version_file" | tr -d '\n\r' | tr -d ' ')
        if [ ! -z "$version" ]; then
            echo "$version"
            return
        fi
    fi

    echo "1.0.0"
}

APP_VERSION=$(get_app_version)

echo "[INFO] Using app version: $APP_VERSION"

DB_FILE="./dictionary.db"

echo "[INFO] Creating database with FTS5..."

if [ -f "$DB_FILE" ]; then
    echo "[WARN] Existing database found. Deleting: $DB_FILE"
    rm "$DB_FILE"
fi

sqlite3 "$DB_FILE" << 'EOF'

PRAGMA journal_mode = DELETE;
PRAGMA synchronous = NORMAL;

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT
);

INSERT INTO metadata (key, value) VALUES
    ('app_version', '$APP_VERSION'),
    ('schema_version', '1.0'),
    ('build_date', datetime('now')),
    ('source_format', 'kautian.ods');
CREATE TABLE dictionary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tl TEXT NOT NULL,
    poj TEXT NOT NULL,
    hanzi TEXT,
    tl_no_tone TEXT NOT NULL,
    poj_no_tone TEXT NOT NULL,
    syllable_count INTEGER,
    is_variant INTEGER DEFAULT 0,

    UNIQUE(tl, poj, hanzi, tl_no_tone, poj_no_tone)
);

CREATE VIRTUAL TABLE dictionary_fts USING fts5(
    tl,
    poj,
    hanzi,
    tl_no_tone,
    poj_no_tone
);

CREATE INDEX idx_poj_no_tone ON dictionary(poj_no_tone);
CREATE INDEX idx_tl_no_tone ON dictionary(tl_no_tone);
CREATE INDEX idx_hanzi ON dictionary(hanzi) WHERE hanzi IS NOT NULL;
CREATE INDEX idx_syllable ON dictionary(syllable_count);

CREATE VIEW poj_view AS
SELECT id, poj as roman, hanzi, poj_no_tone as base_form
FROM dictionary
ORDER BY syllable_count;

CREATE VIEW tl_view AS
SELECT id, tl as roman, hanzi, tl_no_tone as base_form
FROM dictionary
ORDER BY syllable_count;

EOF

echo "[INFO] Database structure created"

echo "[INFO] Database version information:"
sqlite3 "$DB_FILE" "SELECT key, value FROM metadata;"

DICTIONARY_CSV="./csv/dictionary.csv"

if [ ! -f "$DICTIONARY_CSV" ]; then
    echo "[ERROR] Dictionary CSV file not found: $DICTIONARY_CSV"
    echo "[INFO] Please ensure dictionary.csv exists with required columns"
    exit 1
fi

if [ -f "dictionary_version.txt" ]; then
    echo "[INFO] Reading version information..."
    while IFS='=' read -r key value; do
        if [ ! -z "$key" ] && [ ! -z "$value" ]; then
            sqlite3 "$DB_FILE" "UPDATE metadata SET value='$value' WHERE key='$key';"
        fi
    done < dictionary_version.txt
fi

echo "[INFO] Importing dictionary data..."
sqlite3 "$DB_FILE" << EOF
.mode csv
.import $DICTIONARY_CSV temp_import

INSERT OR IGNORE INTO dictionary (
    tl, poj, hanzi,
    tl_no_tone, poj_no_tone,
    syllable_count, is_variant
)
SELECT
    tl, poj,
    CASE WHEN hanzi = '' OR hanzi IS NULL THEN NULL ELSE hanzi END,
    tl_no_tone, poj_no_tone,
    syllable_count,
    CASE WHEN is_variant = 'True' THEN 1 ELSE 0 END
FROM temp_import
WHERE tl IS NOT NULL AND tl != '';

DROP TABLE temp_import;

INSERT INTO dictionary_fts(
    rowid, tl, poj, hanzi,
    tl_no_tone, poj_no_tone
)
SELECT
    id, tl, poj, hanzi,
    tl_no_tone, poj_no_tone
FROM dictionary;

INSERT INTO dictionary_fts(dictionary_fts) VALUES('optimize');

ANALYZE;
EOF

DICT_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM dictionary;")
FTS_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM dictionary_fts;")

echo "[INFO] Import complete!"
echo "  - Dictionary entries: $DICT_COUNT"
echo "  - FTS5 entries: $FTS_COUNT"

BUILD_DATE=$(sqlite3 "$DB_FILE" "SELECT value FROM metadata WHERE key='build_date';")
echo "[INFO] App version: $APP_VERSION (built on $BUILD_DATE)"
echo "[INFO] Database ready for use"