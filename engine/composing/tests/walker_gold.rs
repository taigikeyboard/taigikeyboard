//! E1 walker gold-set harness: types every resolved gold item through the
//! production engine and reports slot-0, word-boundary and top-k metrics per
//! split × category × input variant, and records which homophone the engine
//! picks for every multi-homophone key
//! (`docs/architecture/unified-word-frequency-roadmap.md` §5).
//!
//! `#[ignore]`d; it needs the production artifacts and the files
//! `dictionary/tools/walker_gold.py resolve` writes from the corpus submodule
//! (the committed gold file holds pointers only):
//!
//! ```sh
//! (cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold resolve)
//! cargo test -p composing --test prod walker_gold -- --ignored --nocapture
//! ```
//!
//! Env for `walker_gold_metrics`: `GOLD_SPLITS` (default `dev,calib`; `final`
//! is read once, at the end of P4, and then prints aggregates only) ·
//! `GOLD_FAILURES=1` lists every miss outside `final` · `GOLD_OUT=<path>` writes
//! the aggregate table as TSV · `GOLD_SLOT0_OUT=<path>` writes each non-`final`
//! item's slot 0 per variant (`tools.walker_gold simulate --engine-slot0`).
//!
//! Word boundaries are read from slot 0's displayed roman (one word per
//! space-separated run): what the user sees. The §22 promotion shows one
//! dictionary word where the walker took several edges with the same hanji and
//! reading — counted as that one word.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use protos::engine::CandidateMessage;

use crate::common::{config, default_sources_bitmask, fetch_at_pos_response, Fetch};

const DEFAULT_SPLITS: &str = "dev,calib";
const HELD_OUT_SPLIT: &str = "final";
/// A rank at or past this counts as "not in the first page".
const TOP_K: usize = 5;
/// Joins the words of one gold item in the resolved files (`WORD_SEPARATOR`
/// in `walker_gold.py`); a TL word may itself contain a space.
const WORD_SEPARATOR: char = '+';
/// The variant whose slot-0 roman carries the user's typed separators, so its
/// word spacing says nothing about the walker's boundaries.
const TYPED_SEPARATOR_VARIANT: &str = "tl_hyphen";
/// Columns of `resolved.tsv` before the input variants.
const ITEM_COLUMNS: usize = 7;

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/walker_gold")
}

/// Rows of a tab-separated file with a header, as column → cell maps.
fn read_tsv(path: &PathBuf) -> (Vec<String>, Vec<HashMap<String, String>>) {
    let text = std::fs::read_to_string(path).expect("read walker_gold TSV");
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .expect("TSV header")
        .split('\t')
        .map(str::to_string)
        .collect();
    let rows = lines
        .map(|line| {
            header
                .iter()
                .cloned()
                .zip(line.split('\t').map(str::to_string))
                .collect()
        })
        .collect();
    (header, rows)
}

/// One row of `resolved.tsv`.
struct GoldItem {
    id: String,
    split: String,
    category: String,
    /// Gold words' hanji, `-` stripped.
    words: Vec<String>,
    /// Gold words' TL; their syllable counts are the gold boundaries.
    tl_words: Vec<String>,
    /// Per gold word: every hanji the default dictionary has for its key.
    homophones: Vec<BTreeSet<String>>,
    /// Whole-buffer answers judged acceptable besides the gold text.
    extra_accepted: Vec<String>,
    /// Input variant → keystrokes.
    inputs: Vec<(String, String)>,
}

fn read_gold() -> Vec<GoldItem> {
    let (header, rows) = read_tsv(&output_dir().join("resolved.tsv"));
    let variants = &header[ITEM_COLUMNS..];
    rows.into_iter()
        .map(|cells| {
            let words = |column: &str| -> Vec<String> {
                cells[column]
                    .split(WORD_SEPARATOR)
                    .map(str::to_string)
                    .collect()
            };
            GoldItem {
                id: cells["id"].clone(),
                split: cells["split"].clone(),
                category: cells["category"].clone(),
                words: words("hanji"),
                tl_words: words("tl"),
                homophones: words("homophones")
                    .iter()
                    .map(|word| word.split('|').map(str::to_string).collect())
                    .collect(),
                extra_accepted: cells["extra_accepted"]
                    .split('|')
                    .filter(|answer| !answer.is_empty())
                    .map(str::to_string)
                    .collect(),
                inputs: variants
                    .iter()
                    .map(|variant| (variant.clone(), cells[variant].clone()))
                    .collect(),
            }
        })
        .collect()
}

/// Syllables per word of a displayed roman (`gín-á-lâng tsia̍h-pn̄g` → [3, 2]).
fn syllable_counts(words: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<usize> {
    words
        .into_iter()
        .map(|word| {
            word.as_ref()
                .split(|c: char| c == '-' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .count()
        })
        .collect()
}

/// Internal word boundaries as syllable offsets.
fn boundaries(counts: &[usize]) -> BTreeSet<usize> {
    let mut offset = 0;
    let mut out = BTreeSet::new();
    for count in &counts[..counts.len().saturating_sub(1)] {
        offset += count;
        out.insert(offset);
    }
    out
}

fn boundary_f1(found: &BTreeSet<usize>, gold: &BTreeSet<usize>) -> f64 {
    if found.is_empty() && gold.is_empty() {
        return 1.0;
    }
    let hits = found.intersection(gold).count() as f64;
    if hits == 0.0 {
        return 0.0;
    }
    let precision = hits / found.len() as f64;
    let recall = hits / gold.len() as f64;
    2.0 * precision * recall / (precision + recall)
}

/// Can `hanji` be cut, in order, into one homophone of each gold word?
fn splits_into(hanji: &str, homophones: &[BTreeSet<String>]) -> bool {
    let Some((first, rest)) = homophones.split_first() else {
        return hanji.is_empty();
    };
    first.iter().any(|word| {
        hanji
            .strip_prefix(word.as_str())
            .is_some_and(|tail| splits_into(tail, rest))
    })
}

fn plain_hanji(candidate: &CandidateMessage) -> String {
    candidate
        .hanji
        .as_deref()
        .unwrap_or_default()
        .replace('-', "")
}

/// One fetch, scored against one gold item.
struct Outcome {
    is_exact: bool,
    /// (same words whatever the orthography, boundary F1); `None` when the
    /// variant's display cannot show the walker's boundaries.
    boundary: Option<(bool, f64)>,
    /// Position of the first acceptable whole-buffer candidate, capped at
    /// [`TOP_K`] (also when absent).
    rank: usize,
    slot0_hanji: String,
    slot0_word_syllables: Vec<usize>,
    /// Slot 0 as `hanji [roman]`, for the miss list.
    slot0: String,
}

fn score(
    item: &GoldItem,
    raw: &str,
    candidates: &[CandidateMessage],
    measures_boundaries: bool,
) -> Outcome {
    let whole_buffer = raw.len() as u32;
    let gold_text = item.words.concat();
    let is_acceptable =
        |hanji: &str| hanji == gold_text || item.extra_accepted.iter().any(|a| a == hanji);
    let rank = candidates
        .iter()
        .take(TOP_K)
        .position(|c| c.consumed_span_end == whole_buffer && is_acceptable(&plain_hanji(c)))
        .unwrap_or(TOP_K);
    let Some(slot0) = candidates.first() else {
        return Outcome {
            is_exact: false,
            boundary: measures_boundaries.then_some((false, 0.0)),
            rank,
            slot0_hanji: String::new(),
            slot0_word_syllables: Vec::new(),
            slot0: "∅".into(),
        };
    };
    let slot0_hanji = plain_hanji(slot0);
    let is_whole_buffer = slot0.consumed_span_end == whole_buffer;
    let found = syllable_counts(slot0.roman.split_whitespace());
    let gold = syllable_counts(&item.tl_words);
    let boundary = measures_boundaries.then(|| {
        if !is_whole_buffer {
            return (false, 0.0);
        }
        let is_segmented = found == gold && splits_into(&slot0_hanji, &item.homophones);
        (
            is_segmented,
            boundary_f1(&boundaries(&found), &boundaries(&gold)),
        )
    });
    Outcome {
        is_exact: is_whole_buffer && is_acceptable(&slot0_hanji),
        boundary,
        rank,
        slot0: format!("{slot0_hanji} [{}]", slot0.roman),
        slot0_hanji,
        slot0_word_syllables: found,
    }
}

#[derive(Default)]
struct Tally {
    items: usize,
    exact: usize,
    boundary_items: usize,
    segmented: usize,
    boundary_f1: f64,
    top_k: usize,
    rank_sum: usize,
}

impl Tally {
    fn add(&mut self, outcome: &Outcome) {
        self.items += 1;
        self.exact += usize::from(outcome.is_exact);
        if let Some((is_segmented, f1)) = outcome.boundary {
            self.boundary_items += 1;
            self.segmented += usize::from(is_segmented);
            self.boundary_f1 += f1;
        }
        self.top_k += usize::from(outcome.rank < TOP_K);
        self.rank_sum += outcome.rank;
    }

    fn row(&self) -> String {
        let n = self.items.max(1) as f64;
        let boundary = if self.boundary_items == 0 {
            "-\t-".to_string()
        } else {
            let m = self.boundary_items as f64;
            format!(
                "{:.1}\t{:.3}",
                100.0 * self.segmented as f64 / m,
                self.boundary_f1 / m
            )
        };
        format!(
            "{}\t{:.1}\t{boundary}\t{:.1}\t{:.2}",
            self.items,
            100.0 * self.exact as f64 / n,
            100.0 * self.top_k as f64 / n,
            self.rank_sum as f64 / n,
        )
    }
}

fn candidates_for(mode: &str, raw: &str, enabled_sources_bitmask: u32) -> Vec<CandidateMessage> {
    let response = fetch_at_pos_response(
        &config(mode),
        raw,
        Fetch {
            enabled_sources_bitmask,
            literal_roman_candidate_disabled: true,
            ..Default::default()
        },
    );
    response
        .continuous
        .map(|c| c.candidates)
        .unwrap_or_default()
}

fn inputs_ready(file: &str) -> bool {
    let path = output_dir().join(file);
    let ready = crate::common::production_lexicon_ready() && path.exists();
    if !ready {
        eprintln!(
            "walker_gold: production artifacts or {} absent — run `make dict` and \
             `tools.walker_gold resolve` first; skipping.",
            path.display()
        );
    }
    ready
}

#[test]
#[ignore = "E1 gold-set harness — run with --ignored after `tools.walker_gold resolve`"]
fn walker_gold_metrics() {
    if !inputs_ready("resolved.tsv") {
        return;
    }
    let splits: BTreeSet<String> = std::env::var("GOLD_SPLITS")
        .unwrap_or_else(|_| DEFAULT_SPLITS.to_string())
        .split(',')
        .map(str::to_string)
        .collect();
    let show_failures = std::env::var("GOLD_FAILURES").is_ok_and(|v| v == "1");
    let bitmask = default_sources_bitmask();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    let mut slot0_rows = vec!["id\tvariant\tslot0\tword_syllables".to_string()];
    for item in read_gold()
        .iter()
        .filter(|item| splits.contains(&item.split))
    {
        let is_listed = item.split != HELD_OUT_SPLIT;
        for (variant, raw) in &item.inputs {
            let mode = variant
                .split('_')
                .next()
                .expect("variant has a mode prefix");
            let candidates = candidates_for(mode, raw, bitmask);
            let outcome = score(item, raw, &candidates, variant != TYPED_SEPARATOR_VARIANT);
            for key in [
                format!("{}\tall\tall", item.split),
                format!("{}\tall\t{variant}", item.split),
                format!("{}\t{}\tall", item.split, item.category),
                format!("{}\t{}\t{variant}", item.split, item.category),
            ] {
                tallies.entry(key).or_default().add(&outcome);
            }
            if !is_listed {
                continue;
            }
            let lengths: Vec<String> = outcome
                .slot0_word_syllables
                .iter()
                .map(usize::to_string)
                .collect();
            slot0_rows.push(format!(
                "{}\t{variant}\t{}\t{}",
                item.id,
                outcome.slot0_hanji,
                lengths.join(",")
            ));
            if show_failures && !outcome.is_exact {
                println!(
                    "MISS\t{}\t{}\t{variant}\t{raw}\tgold={}\tslot0={}\trank={}",
                    item.id,
                    item.category,
                    item.words.join(" "),
                    outcome.slot0,
                    outcome.rank,
                );
            }
        }
    }
    let header = "split\tcategory\tvariant\tn\texact%\tsegmented%\tboundary_f1\ttop5%\tmean_rank";
    let mut table = vec![header.to_string()];
    table.extend(
        tallies
            .iter()
            .map(|(key, tally)| format!("{key}\t{}", tally.row())),
    );
    println!("{}", table.join("\n"));
    if let Ok(out) = std::env::var("GOLD_SLOT0_OUT") {
        std::fs::write(&out, slot0_rows.join("\n") + "\n").expect("write GOLD_SLOT0_OUT");
    }
    if let Ok(out) = std::env::var("GOLD_OUT") {
        std::fs::write(&out, table.join("\n") + "\n").expect("write GOLD_OUT");
    }
}

/// Plan D3: the homophone the engine shows for each multi-homophone fully
/// toned key, typed alone — the first whole-key candidate that is one of the
/// key's words (`pick`), and slot 0's hanji as displayed (`slot0`; it may be
/// a split path, or a §22-promoted row). Written to `edge_picks.tsv` for
/// `tools.walker_gold d3`, the offline simulator, and slot-0 before / after
/// counts.
#[test]
#[ignore = "E1 edge-pick screen — run with --ignored after `tools.walker_gold resolve`"]
fn walker_edge_picks() {
    if !inputs_ready("edge_keys.tsv") {
        return;
    }
    let (_, keys) = read_tsv(&output_dir().join("edge_keys.tsv"));
    let mut rows = vec!["key\tsources\tpick\tslot0".to_string()];
    for (sources, bitmask) in [("default", default_sources_bitmask()), ("all", u32::MAX)] {
        for key_row in &keys {
            let key = &key_row["key"];
            let homophones: BTreeSet<&str> = key_row["homophones"].split('|').collect();
            let candidates = candidates_for("tl", key, bitmask);
            let pick = candidates
                .iter()
                .find(|c| {
                    c.consumed_span_end == key.len() as u32
                        && homophones.contains(plain_hanji(c).as_str())
                })
                .map(plain_hanji)
                .unwrap_or_default();
            let slot0 = candidates.first().map(plain_hanji).unwrap_or_default();
            rows.push(format!("{key}\t{sources}\t{pick}\t{slot0}"));
        }
    }
    std::fs::write(output_dir().join("edge_picks.tsv"), rows.join("\n") + "\n")
        .expect("write edge_picks.tsv");
    println!("walker_edge_picks: {} keys × 2 source sets", keys.len());
}

#[test]
fn boundaries_and_f1_follow_word_syllable_counts() {
    // trace: `gín-á-lâng tsia̍h-pn̄g bē-sái` → [3, 2, 2] → boundaries {3, 5}
    let found = syllable_counts("gín-á-lâng tsia̍h-pn̄g bē-sái".split_whitespace());
    assert_eq!(found, vec![3, 2, 2]);
    assert_eq!(boundaries(&found), BTreeSet::from([3, 5]));
    // `kàu siū` vs gold `kàu-siū`: one spurious boundary, none expected → F1 0.
    let split = boundaries(&syllable_counts(["kàu", "siū"]));
    let whole = boundaries(&syllable_counts(["kàu-siū"]));
    assert_eq!(boundary_f1(&split, &whole), 0.0);
    assert_eq!(boundary_f1(&whole, &whole), 1.0);
    // Neutral tone and an internal space both separate syllables.
    assert_eq!(syllable_counts(["hōo--guá", "tsit tsūn"]), vec![2, 2]);
}

#[test]
fn splits_into_accepts_any_homophone_per_word_and_multi_char_syllables() {
    let words = |list: &[&[&str]]| -> Vec<BTreeSet<String>> {
        list.iter()
            .map(|w| w.iter().map(|s| s.to_string()).collect())
            .collect()
    };
    let gold = words(&[&["一"], &["个", "的"]]);
    assert!(splits_into("一的", &gold));
    assert!(!splits_into("一个人", &gold));
    // A romanized word spans more characters than syllables.
    assert!(splits_into("じょうとう", &words(&[&["じょうとう"]])));
}
