//! The production artifacts: the four lexicon files under `assets/dictionaries/`
//! and the pipeline's `dictionary/output/dictionary.csv`.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::repo_root;

/// Paths of the four production lexicon artifacts, as the `String`s
/// `lexicon::LexiconPaths::validated` takes.
pub struct ProductionArtifacts {
    pub dictionary_fst: String,
    pub dictionary_bin: String,
    pub association_bin: String,
    pub syllables_fst: String,
}

/// One production artifact under `assets/dictionaries/` (may be absent — run
/// `make dict`).
pub fn production_artifact(name: &str) -> PathBuf {
    repo_root().join("assets").join("dictionaries").join(name)
}

impl ProductionArtifacts {
    /// `None` (callers soft-skip) when `association.bin` is absent — run
    /// `make dict`.
    pub fn locate() -> Option<Self> {
        let artifact = |name: &str| {
            production_artifact(name)
                .to_str()
                .expect("artifact path UTF-8")
                .to_owned()
        };
        let association_bin = artifact("association.bin");
        if !std::path::Path::new(&association_bin).exists() {
            eprintln!("production artifacts absent — run `make dict`; skipping.");
            return None;
        }
        Some(Self {
            dictionary_fst: artifact("dictionary.fst"),
            dictionary_bin: artifact("dictionary.bin"),
            association_bin,
            syllables_fst: artifact("syllables.fst"),
        })
    }
}

/// `dictionary/output/dictionary.csv` — the pipeline intermediate the
/// parity suites compare the runtime derivations against.
pub fn dictionary_csv_path() -> PathBuf {
    repo_root().join("dictionary/output/dictionary.csv")
}

/// `dictionary/common/cjk.py` — the pipeline's CJK range table, which
/// `lexicon::classification::CJK_RANGES` mirrors.
pub fn dictionary_cjk_py_path() -> PathBuf {
    repo_root().join("dictionary/common/cjk.py")
}

/// `dictionary/tests/fixtures/roman_only_spelling_cases.tsv` — the cases the
/// pipeline's `is_roman_only_hanzi` and `phonetics::api::is_roman_only_spelling`
/// both answer.
pub fn dictionary_roman_only_cases_path() -> PathBuf {
    repo_root().join("dictionary/tests/fixtures/roman_only_spelling_cases.tsv")
}

/// `dictionary.csv`, read once per test process and shared by every test in
/// it. `None` when the file is absent (lean checkout; the answer is cached
/// for the process); a read failure panics. The file must not be rebuilt
/// while a test binary runs.
pub fn dictionary_csv() -> Option<&'static DictionaryCsv> {
    static CSV: OnceLock<Option<DictionaryCsv>> = OnceLock::new();
    CSV.get_or_init(|| {
        let path = dictionary_csv_path();
        path.exists().then(|| DictionaryCsv {
            text: std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
        })
    })
    .as_ref()
}

/// [`dictionary_csv`] for a parity suite that soft-skips in a lean checkout:
/// `None` after logging which `suite` was skipped.
pub fn dictionary_csv_or_skip(suite: &str) -> Option<&'static DictionaryCsv> {
    let csv = dictionary_csv();
    if csv.is_none() {
        eprintln!(
            "skipping {suite} parity test: {} not present (lean checkout)",
            dictionary_csv_path().display()
        );
    }
    csv
}

/// The whole `dictionary.csv` text. The shipped file is plain comma
/// separated with no quoted commas, so a naive split is exact.
pub struct DictionaryCsv {
    text: String,
}

impl DictionaryCsv {
    /// The named `columns` of every data row, in file order. Panics when the
    /// header lacks a column; skips empty lines and rows too short to carry
    /// every selected column.
    pub fn select<const N: usize>(
        &self,
        columns: [&str; N],
    ) -> impl Iterator<Item = [&str; N]> + '_ {
        let mut lines = self.text.lines();
        let header: Vec<&str> = lines
            .next()
            .expect("CSV header line present")
            .split(',')
            .collect();
        let indices = columns.map(|name| {
            header
                .iter()
                .position(|c| *c == name)
                .unwrap_or_else(|| panic!("`{name}` column present in CSV header"))
        });
        let max_idx = indices.iter().copied().max().unwrap_or(0);
        // One buffer reused across rows, split only up to the last selected
        // column.
        let mut fields: Vec<&str> = Vec::with_capacity(max_idx + 1);
        lines.filter_map(move |line| {
            if line.is_empty() {
                return None;
            }
            fields.clear();
            fields.extend(line.split(',').take(max_idx + 1));
            (fields.len() > max_idx).then(|| indices.map(|i| fields[i]))
        })
    }
}
