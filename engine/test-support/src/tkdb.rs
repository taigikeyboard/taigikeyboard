//! Byte serializers for the `dictionary.bin` (TKDB) and `association.bin`
//! (TKWA) fixtures the lexicon readers open.

const TKWA_HEADER_SIZE: usize = 20;

/// The corpus total the walker priced `DictionaryRecord.frequency` against
/// before E1 P3 (`Σ frequency` of the 2026-09-26 `dictionary.csv`); the
/// offline simulator freezes the same value (`dictionary/tools/walker_gold.py`
/// `PRE_P3_CORPUS_TOTAL_FREQ`).
const PRE_E1_CORPUS_TOTAL_FREQ: f64 = 13_056_588.0;

/// A hermetic fixture's `walker_cost` derived from its `frequency` with the
/// pre-E1 walker formula, `round(ln(PRE_E1_CORPUS_TOTAL_FREQ / (1 + freq)) ×
/// 1000)`, so fixtures written against frequencies keep the segmentation
/// they were written for. Tests that pin the cost itself set `walker_cost`
/// explicitly. Production costs come from `dictionary/build/walker_lm.py`.
pub fn walker_cost_from_fixture_frequency(frequency: u32) -> u16 {
    let nats = (PRE_E1_CORPUS_TOTAL_FREQ / (1.0 + f64::from(frequency))).ln();
    (nats * 1000.0).round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// One TKDB record. The optional fields drive the on-disk record layout,
/// independently of the header `version` (tests forge mismatches on purpose):
/// - `syllable_count = None` → v1 layout (no syllable_count byte);
///   `Some(n)` → v2+ layout with that count.
/// - `kautian_subtag = None` → v1/v2 layout (no subtag bytes);
///   `Some(s)` → v3 layout with the 2-byte subtag after syllable_count.
/// - `walker_cost = None` → v1–v3 layout (no walker-cost bytes);
///   `Some(c)` → v4 layout with the 2-byte cost after the subtag.
pub struct TkdbRow<'a> {
    pub bitmask: u16,
    pub frequency: u32,
    pub syllable_count: Option<u8>,
    pub kautian_subtag: Option<u16>,
    pub walker_cost: Option<u16>,
    pub hanji: &'a str,
    pub tl: &'a str,
}

/// Build a TKDB byte sequence with the given `magic`, header `version` and
/// rows (offset table, then the record payload).
pub fn build_tkdb(magic: &[u8; 4], version: u32, rows: &[TkdbRow<'_>]) -> Vec<u8> {
    build_tkdb_with_model(magic, version, rows, TkdbModel::default())
}

/// Hermetic model metadata; independent of the production dictionary build.
#[derive(Clone, Copy)]
pub struct TkdbModel {
    pub tokens: u64,
    pub vocabulary: u64,
    pub alpha: f64,
}

impl Default for TkdbModel {
    fn default() -> Self {
        Self {
            tokens: 3_627_713,
            vocabulary: 167_922,
            alpha: 10.0,
        }
    }
}

/// The v5 fixture header may carry a different model for each loaded dictionary.
pub fn build_tkdb_with_model(
    magic: &[u8; 4],
    version: u32,
    rows: &[TkdbRow<'_>],
    model: TkdbModel,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(magic);
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

    if version >= 5 {
        out.extend_from_slice(&model.tokens.to_le_bytes());
        out.extend_from_slice(&model.vocabulary.to_le_bytes());
        out.extend_from_slice(&model.alpha.to_le_bytes());
    }
    let header_size = out.len();
    let offset_table_size = rows.len() * 4;
    let mut offsets = Vec::<u32>::with_capacity(rows.len());
    let mut payload = Vec::<u8>::new();
    for row in rows {
        offsets.push((header_size + offset_table_size + payload.len()) as u32);
        payload.extend_from_slice(&row.bitmask.to_le_bytes());
        payload.extend_from_slice(&row.frequency.to_le_bytes());
        payload.push(row.hanji.len() as u8);
        payload.push(row.tl.len() as u8);
        if let Some(syll) = row.syllable_count {
            payload.push(syll);
        }
        if let Some(subtag) = row.kautian_subtag {
            payload.extend_from_slice(&subtag.to_le_bytes());
        }
        if let Some(cost) = row.walker_cost {
            payload.extend_from_slice(&cost.to_le_bytes());
        }
        payload.extend_from_slice(row.hanji.as_bytes());
        payload.extend_from_slice(row.tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

/// One TKWA entry: `(bitmask, count, next_word, next_tl)`.
pub type TkwaEntry<'a> = (u16, u32, &'a str, &'a str);

/// Build a TKWA byte sequence: `version`, then `keys` in the order given
/// (the caller sorts them by raw UTF-8 bytes — the reader binary-searches).
pub fn build_tkwa(version: u32, keys: &[(&str, &[TkwaEntry<'_>])]) -> Vec<u8> {
    let key_section_start = TKWA_HEADER_SIZE + keys.len() * 4;
    let key_sizes: Vec<usize> = keys.iter().map(|(key, _)| 1 + key.len() + 4 + 2).collect();
    let entry_section_start = key_section_start + key_sizes.iter().sum::<usize>();
    let entry_size = |(_, _, nw, nt): &TkwaEntry<'_>| 8 + nw.len() + nt.len();

    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(keys.len() as u32).to_le_bytes());
    let entry_count: usize = keys.iter().map(|(_, entries)| entries.len()).sum();
    out.extend_from_slice(&(entry_count as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

    let mut key_offset = key_section_start;
    for size in &key_sizes {
        out.extend_from_slice(&(key_offset as u32).to_le_bytes());
        key_offset += size;
    }
    let mut entry_offset = entry_section_start;
    for (key, entries) in keys {
        assert!(key.len() <= u8::MAX as usize, "key too long");
        out.push(key.len() as u8);
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&(entry_offset as u32).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        entry_offset += entries.iter().map(entry_size).sum::<usize>();
    }
    for (_, entries) in keys {
        for (bitmask, count, next_word, next_tl) in *entries {
            out.extend_from_slice(&bitmask.to_le_bytes());
            out.extend_from_slice(&count.to_le_bytes());
            out.push(next_word.len() as u8);
            out.push(next_tl.len() as u8);
            out.extend_from_slice(next_word.as_bytes());
            out.extend_from_slice(next_tl.as_bytes());
        }
    }
    out
}
