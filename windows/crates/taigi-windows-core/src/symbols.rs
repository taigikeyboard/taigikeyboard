//! The symbol picker's table: three categories of insertable strings, compiled
//! in from the shared desktop JSON. Port of macOS `SymbolTable.swift`.

use crate::strings::StringKey;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;

/// `symbols/desktop-symbols.json` at the repository root — the one source
/// both desktops read (macOS bundles the file; this crate compiles it in),
/// so the two pickers cannot drift.
const BUNDLED_JSON: &str = include_str!("../../../../symbols/desktop-symbols.json");

/// One level-1 cell of the symbol picker, and the key its label is looked up
/// under. A closed roster rather than free-form ids: the label has to exist
/// in every display language, and a category the JSON invents would have no
/// row to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolCategoryId {
    Punctuation,
    Brackets,
    SpecialSymbols,
}

impl SymbolCategoryId {
    pub fn label_key(self) -> StringKey {
        match self {
            Self::Punctuation => StringKey::SymbolPunctuation,
            Self::Brackets => StringKey::SymbolBrackets,
            Self::SpecialSymbols => StringKey::SymbolSpecialSymbols,
        }
    }
}

/// One category and, in menu order, the exact string each of its cells
/// inserts — a bracket pair is one entry (`「」`), which is what lets one
/// pick write both halves (USER 2026-09-09).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SymbolCategory {
    pub id: SymbolCategoryId,
    pub symbols: Vec<String>,
}

/// Why a table was refused, naming the offending value.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SymbolTableError {
    #[error("the symbol table does not parse: {0}")]
    Json(String),
    #[error("the symbol table has no categories")]
    NoCategories,
    #[error("category {0:?} appears twice")]
    DuplicateCategory(SymbolCategoryId),
    #[error("category {0:?} has no symbols")]
    EmptyCategory(SymbolCategoryId),
    #[error("symbol {symbol} appears twice in {category:?}")]
    DuplicateSymbol {
        symbol: String,
        category: SymbolCategoryId,
    },
    #[error("category {0:?} has an empty symbol")]
    EmptySymbol(SymbolCategoryId),
}

/// The whole table. Validated on construction, so no table can exist that
/// the picker cannot show: a duplicate would draw two cells for one pick, an
/// empty symbol would consume a key and write nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolTable {
    categories: Vec<SymbolCategory>,
}

impl SymbolTable {
    pub fn new(categories: Vec<SymbolCategory>) -> Result<Self, SymbolTableError> {
        if categories.is_empty() {
            return Err(SymbolTableError::NoCategories);
        }
        let mut seen_ids = HashSet::new();
        for category in &categories {
            if !seen_ids.insert(category.id) {
                return Err(SymbolTableError::DuplicateCategory(category.id));
            }
            if category.symbols.is_empty() {
                return Err(SymbolTableError::EmptyCategory(category.id));
            }
            let mut seen_symbols = HashSet::new();
            for symbol in &category.symbols {
                if symbol.is_empty() {
                    return Err(SymbolTableError::EmptySymbol(category.id));
                }
                if !seen_symbols.insert(symbol.as_str()) {
                    return Err(SymbolTableError::DuplicateSymbol {
                        symbol: symbol.clone(),
                        category: category.id,
                    });
                }
            }
        }
        Ok(Self { categories })
    }

    /// Decodes and validates the JSON the file holds. Only `categories` is
    /// read; the file's `comment` is for the reader of the file.
    pub fn parse(json: &str) -> Result<Self, SymbolTableError> {
        #[derive(Deserialize)]
        struct File {
            categories: Vec<SymbolCategory>,
        }
        let file: File = serde_json::from_str(json)
            .map_err(|error| SymbolTableError::Json(error.to_string()))?;
        Self::new(file.categories)
    }

    /// The compiled-in table, or `None` when the file does not validate —
    /// logged once, at the first ask, and the picker chord then does nothing
    /// rather than open an empty window. `the_bundled_table_is_valid_and_in_menu_order`
    /// pins that it does.
    pub fn bundled() -> Option<&'static SymbolTable> {
        static TABLE: OnceLock<Option<SymbolTable>> = OnceLock::new();
        TABLE
            .get_or_init(|| match Self::parse(BUNDLED_JSON) {
                Ok(table) => Some(table),
                Err(error) => {
                    log::error!("symbols.bundled_table_invalid error={error}");
                    None
                }
            })
            .as_ref()
    }

    pub fn categories(&self) -> &[SymbolCategory] {
        &self.categories
    }

    /// The category `id` names, or `None` when the table has none.
    pub fn category(&self, id: SymbolCategoryId) -> Option<&SymbolCategory> {
        self.categories.iter().find(|category| category.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> &'static SymbolTable {
        SymbolTable::bundled().expect("the bundled symbol table validates")
    }

    #[test]
    fn the_bundled_table_is_valid_and_in_menu_order() {
        // trace: SymbolTableTests.swift — the same file, the same three
        // categories in the same order.
        let ids: Vec<_> = bundled().categories().iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            [
                SymbolCategoryId::Punctuation,
                SymbolCategoryId::Brackets,
                SymbolCategoryId::SpecialSymbols
            ]
        );
    }

    #[test]
    fn every_bracket_is_a_pair_and_every_mark_one_character() {
        for pair in &bundled()
            .category(SymbolCategoryId::Brackets)
            .unwrap()
            .symbols
        {
            assert_eq!(pair.chars().count(), 2, "{pair} is not a pair");
        }
        for id in [
            SymbolCategoryId::Punctuation,
            SymbolCategoryId::SpecialSymbols,
        ] {
            for symbol in &bundled().category(id).unwrap().symbols {
                assert_eq!(symbol.chars().count(), 1, "{symbol} in {id:?}");
            }
        }
    }

    #[test]
    fn the_punctuation_category_carries_every_full_width_mapped_mark() {
        // A user in romanization mode — where the full-width map is off —
        // still has a way to write every mark the map would have typed.
        let table = bundled();
        let punctuation = &table
            .category(SymbolCategoryId::Punctuation)
            .unwrap()
            .symbols;
        let brackets: String = table
            .category(SymbolCategoryId::Brackets)
            .unwrap()
            .symbols
            .concat();
        for key in ",.?!;:()[]{}<>'@#$%^&*_+".chars() {
            let Some(mapped) = crate::policies::full_width_mapped(&key.to_string()) else {
                continue;
            };
            assert!(
                punctuation.contains(&mapped) || brackets.contains(&mapped),
                "{mapped} is typed by the full-width map but not offered by the picker"
            );
        }
    }

    #[test]
    fn validation_refuses_what_the_picker_cannot_show() {
        let category = |id, symbols: &[&str]| SymbolCategory {
            id,
            symbols: symbols.iter().map(|s| (*s).to_owned()).collect(),
        };
        assert_eq!(
            SymbolTable::new(vec![]).unwrap_err(),
            SymbolTableError::NoCategories
        );
        assert_eq!(
            SymbolTable::new(vec![category(SymbolCategoryId::Punctuation, &["，", "，"])])
                .unwrap_err(),
            SymbolTableError::DuplicateSymbol {
                symbol: "，".into(),
                category: SymbolCategoryId::Punctuation
            }
        );
        assert_eq!(
            SymbolTable::new(vec![category(SymbolCategoryId::Brackets, &["「」", ""])])
                .unwrap_err(),
            SymbolTableError::EmptySymbol(SymbolCategoryId::Brackets)
        );
        assert_eq!(
            SymbolTable::new(vec![category(SymbolCategoryId::Brackets, &[])]).unwrap_err(),
            SymbolTableError::EmptyCategory(SymbolCategoryId::Brackets)
        );
        let brackets = category(SymbolCategoryId::Brackets, &["「」"]);
        assert_eq!(
            SymbolTable::new(vec![brackets.clone(), brackets]).unwrap_err(),
            SymbolTableError::DuplicateCategory(SymbolCategoryId::Brackets)
        );
        assert!(matches!(
            SymbolTable::parse(r#"{"categories":[{"id":"emoji","symbols":["😀"]}]}"#),
            Err(SymbolTableError::Json(_))
        ));
    }
}
