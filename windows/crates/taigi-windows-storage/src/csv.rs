//! Reading and writing the hand-editable CSV of the user's own dictionary.
//! Port of `Storage/UserDataCSV.swift` + `CustomDictionaryCSV.swift`.

// 中文: 自訂詞庫 CSV — 三平台共用的引號規則(單行記錄方言),roman,hanzi 兩欄。

use crate::custom_dictionary::CustomDictionaryRow;
use std::path::Path;

/// The CSV dialect the platforms share. A SINGLE-RECORD dialect, not full
/// RFC 4180: quoting inside a line is honoured (doubled-quote escape
/// included), but a record is always one line, because every mirror splits
/// on newlines before parsing. CROSS-PLATFORM INVARIANT — mirrors iOS
/// `CSVDocument.swift` (`parseLine` / `escape`) and Android
/// `DictionaryCsvCodec.kt`.
pub struct UserDataCSV;

impl UserDataCSV {
    /// Splits one CSV line into its fields. A doubled quote inside a quoted
    /// field is one literal quote; an unbalanced quote leaves the rest of the
    /// line quoted rather than erroring.
    pub fn parse_line(line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut is_inside_quotes = false;
        let characters: Vec<char> = line.chars().collect();
        let mut index = 0;
        while index < characters.len() {
            let character = characters[index];
            if character == '"'
                && is_inside_quotes
                && index + 1 < characters.len()
                && characters[index + 1] == '"'
            {
                current.push('"');
                index += 1;
            } else if character == '"' {
                is_inside_quotes = !is_inside_quotes;
            } else if character == ',' && !is_inside_quotes {
                fields.push(std::mem::take(&mut current));
            } else {
                current.push(character);
            }
            index += 1;
        }
        fields.push(current);
        fields
    }

    /// Quotes a field only when it would otherwise change the parse — the
    /// trigger set is exactly `,` `"` and newline.
    pub fn escape(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_owned()
        }
    }
}

/// Why a custom-dictionary CSV could not be imported.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CustomDictionaryCSVError {
    /// Bigger than `MAX_FILE_SIZE_BYTES`. Checked before the file is read.
    #[error("file is larger than {} MB", .limit_bytes / (1024 * 1024))]
    FileTooLarge { limit_bytes: u64 },
    #[error("file is not UTF-8 text")]
    NotUtf8,
    /// Content but not one usable row — a wrong-format file, worth saying so.
    #[error("no usable rows in the file")]
    NoUsableRows,
    #[error("file holds more than {limit} entries")]
    TooManyRows { limit: usize },
    #[error("could not read the file: {0}")]
    Read(String),
}

/// The `roman,hanzi` CSV the 自訂詞庫 page reads and writes.
pub struct CustomDictionaryCSV;

impl CustomDictionaryCSV {
    /// CROSS-PLATFORM INVARIANT — mirrors iOS `CustomDictionaryService.swift:90`.
    pub const MAX_FILE_SIZE_BYTES: u64 = 5 * 1024 * 1024;

    pub fn encode(rows: &[CustomDictionaryRow]) -> String {
        rows.iter()
            .map(|row| {
                format!(
                    "{},{}\n",
                    UserDataCSV::escape(&row.roman),
                    UserDataCSV::escape(&row.hanzi)
                )
            })
            .collect()
    }

    /// Parses `csv` into rows. A row needs a romanization; the 漢字 column
    /// may be empty. Unusable rows are dropped, but a file that is entirely
    /// unusable is reported.
    pub fn decode(
        csv: &str,
        entry_limit: usize,
    ) -> Result<Vec<CustomDictionaryRow>, CustomDictionaryCSVError> {
        let mut content_lines = 0;
        let mut rows = Vec::new();
        for line in csv.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            content_lines += 1;
            let columns: Vec<String> = UserDataCSV::parse_line(trimmed)
                .into_iter()
                .map(|column| column.trim().to_owned())
                .collect();
            if columns.len() < 2 || columns[0].is_empty() {
                continue;
            }
            rows.push(CustomDictionaryRow::new(&columns[0], &columns[1]));
        }
        if content_lines > 0 && rows.is_empty() {
            return Err(CustomDictionaryCSVError::NoUsableRows);
        }
        if rows.len() > entry_limit {
            return Err(CustomDictionaryCSVError::TooManyRows { limit: entry_limit });
        }
        Ok(rows)
    }

    /// Reads a file the user picked, refusing it before the read when it is
    /// too big to be a word list.
    pub fn decode_file(
        path: &Path,
        entry_limit: usize,
    ) -> Result<Vec<CustomDictionaryRow>, CustomDictionaryCSVError> {
        let size = std::fs::metadata(path)
            .map_err(|error| CustomDictionaryCSVError::Read(error.to_string()))?
            .len();
        if size > Self::MAX_FILE_SIZE_BYTES {
            return Err(CustomDictionaryCSVError::FileTooLarge {
                limit_bytes: Self::MAX_FILE_SIZE_BYTES,
            });
        }
        let bytes = std::fs::read(path)
            .map_err(|error| CustomDictionaryCSVError::Read(error.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|_| CustomDictionaryCSVError::NotUtf8)?;
        Self::decode(&text, entry_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: usize = 30_000;

    #[test]
    fn round_trip_keeps_both_columns_and_survives_quotes() {
        // trace: CustomDictionaryCSVTests.swift:9-39.
        let rows = [
            CustomDictionaryRow::new("gâu-tsá", "𠢕早"),
            CustomDictionaryRow::new("tsia̍h-pá--buē", "食飽未"),
            CustomDictionaryRow::new("say \"hi\"", "講,好"),
        ];
        let decoded =
            CustomDictionaryCSV::decode(&CustomDictionaryCSV::encode(&rows), LIMIT).unwrap();
        let romans: Vec<&str> = decoded.iter().map(|r| r.roman.as_str()).collect();
        assert_eq!(romans, ["gâu-tsá", "tsia̍h-pá--buē", "say \"hi\""]);
        assert_eq!(decoded[2].hanzi, "講,好");
        assert_eq!(
            UserDataCSV::escape(" lead"),
            " lead",
            "a leading space is left alone"
        );
        assert_eq!(
            UserDataCSV::parse_line("a,\"b,c\",\"d\"\"e\""),
            ["a", "b,c", "d\"e"]
        );
        assert_eq!(
            UserDataCSV::parse_line("\"unbalanced,rest"),
            ["unbalanced,rest"]
        );
    }

    #[test]
    fn decode_keeps_romanization_only_rows_skips_bad_lines_and_refuses_nothing_usable() {
        // trace: CustomDictionaryCSVTests.swift:41-72.
        let decoded = CustomDictionaryCSV::decode("gua,\n,我\n", LIMIT).unwrap();
        assert_eq!(
            decoded.iter().map(|r| r.roman.as_str()).collect::<Vec<_>>(),
            ["gua"]
        );
        let decoded = CustomDictionaryCSV::decode("gua,我\nnonsense\nli,你\n", LIMIT).unwrap();
        assert_eq!(
            decoded.iter().map(|r| r.hanzi.as_str()).collect::<Vec<_>>(),
            ["我", "你"]
        );
        assert_eq!(
            CustomDictionaryCSV::decode("nonsense\n???\n", LIMIT),
            Err(CustomDictionaryCSVError::NoUsableRows)
        );
        assert_eq!(
            CustomDictionaryCSV::decode("", LIMIT).unwrap().len(),
            0,
            "an empty file is empty, not wrong"
        );
        assert_eq!(
            CustomDictionaryCSV::decode("a,1\nb,2\nc,3\n", 2),
            Err(CustomDictionaryCSVError::TooManyRows { limit: 2 })
        );
    }

    #[test]
    fn decode_file_refuses_oversize_and_non_utf8() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("dict.csv");
        std::fs::write(&path, vec![0xFF, 0xFE, b',', b'a']).unwrap();
        assert_eq!(
            CustomDictionaryCSV::decode_file(&path, LIMIT),
            Err(CustomDictionaryCSVError::NotUtf8)
        );
        std::fs::write(&path, "gua,我\n").unwrap();
        assert_eq!(
            CustomDictionaryCSV::decode_file(&path, LIMIT)
                .unwrap()
                .len(),
            1
        );
    }
}
