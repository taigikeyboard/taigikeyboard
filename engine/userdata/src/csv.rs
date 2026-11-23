//! Reading and writing the hand-editable CSV of the user's own dictionary.
//! Port of the former Swift `UserDataCSV` + `CustomDictionaryCSV`.

use crate::custom_dictionary::CustomDictionaryRow;

/// The CSV dialect the platforms share. A SINGLE-RECORD dialect, not full
/// RFC 4180: quoting inside a line is honoured (doubled-quote escape
/// included), but a record is always one line, because every mirror splits
/// on newlines before parsing. The engine is the only parser; iOS
/// `CSVDocument` only wraps the exported text for the file exporter.
pub struct UserDataCSV;

impl UserDataCSV {
    /// Splits one CSV line into its fields. A doubled quote inside a quoted
    /// field is one literal quote; an unbalanced quote leaves the rest of the
    /// line quoted rather than erroring.
    fn parse_line(line: &str) -> Vec<String> {
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
}

/// The `roman,hanji` CSV the Custom Dictionary page reads and writes.
pub struct CustomDictionaryCSV;

impl CustomDictionaryCSV {
    /// Largest CSV file an import accepts (5 MiB).
    pub const MAX_FILE_SIZE_BYTES: u64 = 5 * 1024 * 1024;

    pub fn encode(rows: &[CustomDictionaryRow]) -> String {
        rows.iter()
            .map(|row| {
                format!(
                    "{},{}\n",
                    UserDataCSV::escape(&row.roman),
                    UserDataCSV::escape(&row.hanji)
                )
            })
            .collect()
    }

    /// Parses `csv` into rows. A row needs a romanization; the Hanji column
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

    /// Parses a file's bytes the platform read for the user — the size cap
    /// first, then UTF-8, then [`Self::decode`].
    pub fn decode_bytes(
        bytes: &[u8],
        entry_limit: usize,
    ) -> Result<Vec<CustomDictionaryRow>, CustomDictionaryCSVError> {
        if bytes.len() as u64 > Self::MAX_FILE_SIZE_BYTES {
            return Err(CustomDictionaryCSVError::FileTooLarge {
                limit_bytes: Self::MAX_FILE_SIZE_BYTES,
            });
        }
        let text = std::str::from_utf8(bytes).map_err(|_| CustomDictionaryCSVError::NotUtf8)?;
        Self::decode(text, entry_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: usize = 30_000;

    #[test]
    fn round_trip_keeps_both_columns_and_survives_quotes() {
        // trace: `UserDataCSV::escape` quotes only on `,` `"` newline; `parse_line`
        // undoubles `""` and keeps an unbalanced quote's rest as one field.
        let rows = [
            CustomDictionaryRow::new("gâu-tsá", "𠢕早"),
            CustomDictionaryRow::new("tsia̍h-pá--buē", "食飽未"),
            CustomDictionaryRow::new("say \"hi\"", "講,好"),
        ];
        let decoded =
            CustomDictionaryCSV::decode(&CustomDictionaryCSV::encode(&rows), LIMIT).unwrap();
        let romans: Vec<&str> = decoded.iter().map(|r| r.roman.as_str()).collect();
        assert_eq!(romans, ["gâu-tsá", "tsia̍h-pá--buē", "say \"hi\""]);
        assert_eq!(decoded[2].hanji, "講,好");
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
        // trace: `CustomDictionaryCSV::decode` — a row needs a roman, Hanji may be
        // empty; all-unusable content → `NoUsableRows`; over the limit → `TooManyRows`.
        let decoded = CustomDictionaryCSV::decode("gua,\n,我\n", LIMIT).unwrap();
        assert_eq!(
            decoded.iter().map(|r| r.roman.as_str()).collect::<Vec<_>>(),
            ["gua"]
        );
        let decoded = CustomDictionaryCSV::decode("gua,我\nnonsense\nli,你\n", LIMIT).unwrap();
        assert_eq!(
            decoded.iter().map(|r| r.hanji.as_str()).collect::<Vec<_>>(),
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
    fn decode_bytes_refuses_non_utf8() {
        assert_eq!(
            CustomDictionaryCSV::decode_bytes(&[0xFF, 0xFE, b',', b'a'], LIMIT),
            Err(CustomDictionaryCSVError::NotUtf8)
        );
        assert_eq!(
            CustomDictionaryCSV::decode_bytes("gua,我\n".as_bytes(), LIMIT)
                .unwrap()
                .len(),
            1
        );
    }
}
