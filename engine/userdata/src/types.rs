//! The rows the stores hand out and take in. Moved from
//! `taigi-desktop-core` (`engine/{composing,nextword,phonetics}.rs`), which
//! re-exports them at their old paths until the desktop switch.

/// One learned frequency row, as the store hands it over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrequencyRow {
    pub word: String,
    pub tl: String,
    pub count: i64,
    pub last_used_ms: i64,
}

/// One custom-dictionary row, the raw stored columns (`CustomDictEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomEntry {
    pub roman: String,
    /// Empty = romanization-only entry; mapped to an ABSENT wire field.
    pub hanzi: String,
}

/// One auto-learned phrase (§50): the `(Hanji, canonical-TL)` pair the user
/// composed segment by segment. Ranked as `composing::UserRows.learned`, a
/// competitor of the dictionary rows — never the override `CustomEntry` is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedPhrase {
    pub hanzi: String,
    pub canonical_tl: String,
}

/// One learned bigram. `previous_tl` / `next_tl` are canonical TL — the
/// identity axis of Core Principle #7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationPair {
    pub previous: String,
    pub previous_tl: String,
    pub next: String,
    pub next_tl: String,
}

/// One way a custom-dictionary entry can be found. `family` is the
/// romanization system (`tl` / `poj` / `tps`), `form` how much of the reading
/// it carries (`num` / `notone` / `abbrev`), `key` the fused string both
/// sides match on. The engine owns all three vocabularies — the store keeps
/// what it is given and asks for the query key the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomSearchKey {
    pub family: String,
    pub form: String,
    pub key: String,
}

impl From<phonetics::api::CustomSearchKey> for CustomSearchKey {
    fn from(key: phonetics::api::CustomSearchKey) -> Self {
        Self {
            family: key.family.to_owned(),
            form: key.form.to_owned(),
            key: key.key,
        }
    }
}
