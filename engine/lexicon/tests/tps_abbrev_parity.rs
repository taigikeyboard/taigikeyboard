//! v3.5.9 D / C-5 — runtime TPS-abbrev derivation parity test (D capstone).
//!
//! C-0's build pipeline emits `tps:<tps_abbrev>` keys per row whose TL
//! has ≥2 syllables (`dictionary/common/abbrev.py::extract_tps_abbrev`).
//! This test pins a runtime mirror of that derivation against the shipped
//! `dictionary/output/dictionary.csv` column. Drift breaks the
//! continuous-input toneless-key guard's abbrev-collision rejection
//! contract silently — the guard at `lexicon::continuous::
//! matches_continuous_tps_toneless_key` rejects FST hits whose
//! `tps_notone_from_tl(record.tl) != body`, so a `tps:<abbrev>` hit gets
//! rejected only when `tps_notone` derivation works correctly; this test
//! is the symmetric build-pipeline ↔ runtime parity gate for the
//! `tps_abbrev` column itself.
//!
//! ## Runtime derivation
//!
//! Mirrors `extract_tps_abbrev(tl, tps_per_syllable)`:
//!   1. Split TL on `[-\s]+`; require ≥2 non-empty tokens.
//!   2. Per TL token: `phonetics::api::to_tone_number(token)` → numeric
//!      TL → `phonetics::tl_numeric_token_to_tps(numeric, false, true)`
//!      (`or_maps_to_er=true` matches the build pipeline's Node bridge
//!      default — same toggle [`phonetics::tps_notone_from_tl`] uses).
//!   3. Take the first non-tone-mark, non-whitespace, non-hyphen Bopomofo
//!      char of each per-token TPS string.
//!   4. Concat fused.
//!   5. If any token's TPS is empty (Node bridge rejected upstream), the
//!      Python helper returns `""`; mirror that posture.
//!
//! ## tps_abbrev_var (C-3a er↔or dual emit)
//!
//! C-3a dual-emits `tps:<tps_abbrev>` AND `tps:<tps_abbrev_var>` per row
//! whose primary abbrev contains ㄜ. Mirrors
//! `dictionary/common/notone.py::apply_or_dialect_variant` (ㄜ→ㄛ glyph
//! swap) — runtime reuse is [`phonetics::tps_notone_or_variant`] (the
//! substitution is glyph-level and applies identically to abbrev or
//! notone form). The csv has 28 non-empty `tps_abbrev_var` rows; pin
//! all of them.

// 中文: D / C-5 — runtime TPS-abbrev derive 與 build pipeline 平行性測試。
// 中文:   逐行 dictionary.csv 取 (tl, tps_abbrev) → 跑 runtime derivation → 對位 byte-match。
// 中文:   再對 tps_abbrev_var 欄(28 行)跑 tps_notone_or_variant 同等變體規則檢查。

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Runtime mirror of `dictionary/common/abbrev.py::extract_tps_abbrev`.
/// Returns the concatenated first-Bopomofo-char-per-syllable abbrev, or
/// `""` when (a) TL has <2 tokens, or (b) any token's per-syllable TPS
/// is empty (Node bridge rejection in production).
// 中文: extract_tps_abbrev 的 runtime 鏡像;<2 音節或任一音節 TPS 空 ⇒ "".
fn derive_tps_abbrev_runtime(tl: &str) -> String {
    let tokens: Vec<&str> = tl.split(['-', ' ', '\t']).filter(|t| !t.is_empty()).collect();
    if tokens.len() < 2 {
        return String::new();
    }
    let mut out = String::with_capacity(tokens.len() * 3);
    for tok in tokens {
        let numeric = phonetics::to_tone_number(tok);
        let tps = phonetics::tl_numeric_token_to_tps(&numeric, false, true);
        let first = tps.chars().find(|&c| {
            !phonetics::is_tps_tone_mark(c) && c != '-' && !c.is_whitespace()
        });
        match first {
            Some(c) => out.push(c),
            None => return String::new(),
        }
    }
    out
}

fn dictionary_csv_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // engine
    path.pop(); // repo root
    path.push("dictionary");
    path.push("output");
    path.push("dictionary.csv");
    path
}

fn read_rows(path: &std::path::Path) -> std::io::Result<Vec<(String, String, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let header = lines.next().expect("CSV header line present")?;
    let columns: Vec<&str> = header.split(',').collect();
    let tl_idx = columns
        .iter()
        .position(|c| *c == "tl")
        .expect("`tl` column present in CSV header");
    let tps_abbrev_idx = columns
        .iter()
        .position(|c| *c == "tps_abbrev")
        .expect("`tps_abbrev` column present in CSV header");
    let tps_abbrev_var_idx = columns
        .iter()
        .position(|c| *c == "tps_abbrev_var")
        .expect("`tps_abbrev_var` column present in CSV header");

    let mut rows = Vec::new();
    let max_idx = tl_idx.max(tps_abbrev_idx).max(tps_abbrev_var_idx);
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= max_idx {
            continue;
        }
        rows.push((
            fields[tl_idx].to_string(),
            fields[tps_abbrev_idx].to_string(),
            fields[tps_abbrev_var_idx].to_string(),
        ));
    }
    Ok(rows)
}

fn is_pure_bopomofo(s: &str) -> bool {
    !s.is_empty() && s.chars().all(phonetics::is_tps_char)
}

#[test]
fn runtime_tps_abbrev_matches_build_pipeline_for_every_row() {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping TPS-abbrev parity test: {} not present (lean checkout)",
            path.display()
        );
        return;
    }

    let rows = read_rows(&path).expect("read dictionary.csv rows");
    assert!(
        !rows.is_empty(),
        "expected non-empty `dictionary.csv` row stream"
    );

    let mut compared = 0usize;
    let mut anomalies = 0usize;
    let mut drift: Vec<(String, String, String)> = Vec::new();

    for (tl, abbrev, _var) in &rows {
        if tl.is_empty() || abbrev.is_empty() {
            continue;
        }
        // Upstream build-pipeline anomalies (rare): non-pure-Bopomofo
        // glyphs in tps_abbrev; skip — no continuous user input can
        // produce a `tps:<non-bopomofo>` lookup key.
        if !is_pure_bopomofo(abbrev) {
            anomalies += 1;
            continue;
        }
        compared += 1;
        let derived = derive_tps_abbrev_runtime(tl);
        if derived != *abbrev {
            drift.push((tl.clone(), abbrev.clone(), derived));
            if drift.len() >= 10 {
                break;
            }
        }
    }

    assert!(
        drift.is_empty(),
        "runtime TPS-abbrev derivation drifted from build pipeline on \
         {} of {} compared rows (first {} shown):\n{}",
        drift.len(),
        compared,
        drift.len(),
        drift
            .iter()
            .map(|(tl, expected, got)| format!(
                "  tl={tl:?} expected tps_abbrev={expected:?} got {got:?}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    eprintln!(
        "TPS-abbrev parity OK: {compared} rows compared \
         ({anomalies} non-Bopomofo anomaly rows skipped)."
    );
}

/// v3.5.9 D / C-5 — variant column parity. C-3a's
/// `apply_or_dialect_variant` is glyph-level ㄜ→ㄛ substitution applied
/// post-derivation; runtime mirror is [`phonetics::tps_notone_or_variant`]
/// (same helper notone uses — substitution is independent of which
/// column produced the input).
#[test]
fn runtime_tps_abbrev_var_matches_build_pipeline_for_every_row() {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping TPS-abbrev-var parity test: {} not present (lean checkout)",
            path.display()
        );
        return;
    }

    let rows = read_rows(&path).expect("read dictionary.csv rows");
    // Iterate every row with a non-empty pure-Bopomofo primary, compute
    // the runtime-derived variant, and compare to the CSV variant —
    // INCLUDING the empty-string case (no ㄜ → no variant). Skipping
    // empty-var rows would mask a regression where the build pipeline
    // stops populating `tps_abbrev_var` entirely. Codex post-impl
    // BLOCK 2026-05-26.
    // 中文: 不能略過空 var 行,否則 pipeline 停發變體時測試會無聲通過;
    // 中文:   逐行比對 derived (含空) vs CSV var,並追蹤含 ㄜ 行最少筆數。
    let mut compared = 0usize;
    let mut anomalies = 0usize;
    let mut nonempty_vars = 0usize;
    let mut drift: Vec<(String, String, String)> = Vec::new();

    for (_tl, abbrev, var) in &rows {
        if abbrev.is_empty() {
            continue;
        }
        if !is_pure_bopomofo(abbrev) || (!var.is_empty() && !is_pure_bopomofo(var)) {
            anomalies += 1;
            continue;
        }
        compared += 1;
        let derived = phonetics::tps_notone_or_variant(abbrev);
        if !var.is_empty() {
            nonempty_vars += 1;
        }
        if derived != *var {
            drift.push((abbrev.clone(), var.clone(), derived));
            if drift.len() >= 10 {
                break;
            }
        }
    }

    assert!(
        drift.is_empty(),
        "runtime tps_notone_or_variant(tps_abbrev) drifted from build \
         pipeline tps_abbrev_var on {} of {} compared rows (first {} \
         shown):\n{}",
        drift.len(),
        compared,
        drift.len(),
        drift
            .iter()
            .map(|(abbrev, expected, got)| format!(
                "  tps_abbrev={abbrev:?} expected tps_abbrev_var={expected:?} got {got:?}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Coverage gate (Codex post-impl Finding 1): non-empty
    // `tps_abbrev_var` rows are rare (~28 in the current CSV — only
    // multi-syllable rows whose first-Bopomofo abbrev char is ㄜ, e.g.
    // er/or-initial nuclei). Asserting > 0 catches a build-pipeline
    // regression that silently zeros out the column.
    // 中文: 覆蓋率守門 — pipeline 完全停發變體時上面 drift loop 仍會 0/0 過關,
    // 中文:   故強制 nonempty_vars > 0(C-3a 後 CSV 應有 ~28 筆,只有首音節縮寫為 ㄜ 之列)。
    assert!(
        nonempty_vars > 0,
        "expected the shipped CSV to contain at least one non-empty \
         `tps_abbrev_var` row (C-3a build pipeline emits dual-form for \
         multi-syllable rows whose first-Bopomofo abbrev char is ㄜ). \
         Found 0 — build pipeline regressed or the CSV column is unpopulated."
    );

    eprintln!(
        "TPS-abbrev-var parity OK: {compared} rows compared, \
         {nonempty_vars} non-empty variants ({anomalies} anomaly rows skipped)."
    );
}
