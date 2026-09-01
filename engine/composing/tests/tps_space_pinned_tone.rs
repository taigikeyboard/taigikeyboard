//! A3 (§41) — TPS keyboard-space tone pinning integration test.
//!
//! Reported bug (Gmail `19eeca0bc4a8e6e1`, v3.6.3):
//! 「方音齒盤兮第一佮第四調無法度用空白齒揀聲調，其他聲調正常。」 TPS writes
//! tones 2/3/5/6/7/8/9 with a standalone mark, so typing the mark already
//! filters candidates to that tone (§17). Tones 1 (open rime) and 4 (stop
//! coda ㆴㆵㆻㆷ) carry NO mark — the keyboard's space is their only
//! delimiter — and the space was stripped to zero width before key
//! selection (§31 cross-space phrase edges), so every tone of the toneless
//! key came back. `ㄒㄧ`␣ surfaced 是 (si7) / 時 (si5) / 死 (si2) instead of
//! 詩 (si1), and `ㄐㄧㆵ`␣ surfaced the higher-frequency 一 (tsit8) instead
//! of 這 (tsit4).
//!
//! After the fix a span that ENDS on the stripped space keeps only readings
//! whose syllable at that boundary carries an unmarked tone (1 or 4). A
//! barrier strictly INSIDE a span stays a plain syllable boundary, which is
//! what keeps §31's `ㄍㄠ`␣`ㄉㄞ˪` → 交代 alive (its first syllable is
//! kau1, but the phrase's own span runs THROUGH the space, not up to it).
//!
//! Fixture shape follows `continuous_explicit_tone.rs`: hermetic
//! `LexiconHandle` install, `dictionary.fst` emitting BOTH the toneless
//! `tps:<tps_notone>` and toned `tps:<tps_num>` families like production
//! `dictionary/build/create_fst.py:126-141`. Per the fixture-coverage rule
//! in `.claude/rules/taigi-incidents.md`, every strict prefix of a probed
//! key that is itself a production syllable is present as a control row
//! (之/tsi under ㄐㄧㆵ) and asserted on.

// 中文: A3 (§41) — TPS 空白釘定聲調整合測試。TPS 第 2/3/5/6/7/8/9 調有調號可過濾,
// 中文:   第 1 調(開音節)與第 4 調(入聲尾 ㆴㆵㆻㆷ)無調號,唯一分界是鍵盤空白,而該空白在
// 中文:   鍵選擇前已被剝為零寬(§31 跨空白整詞 edge)→ 全聲調都回來。修復後:span 結尾落在
// 中文:   被剝空白上時,只留該邊界音節為無調號調(1/4)的讀法;span 內部的 barrier 仍是單純
// 中文:   音節邊界,故 §31 的 ㄍㄠ␣ㄉㄞ˪ → 交代 不受影響(整詞 span 跨過空白而非停在其上)。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use composing::api::Engine;
use composing::dispatch;
use fst::SetBuilder;
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, ComposingRequest, CustomDictEntry, EnterContinuous, FetchAtPos, Start,
};

const SEPARATOR: u8 = 0xFF;
const RANK_NEUTRAL_BITMASK: u16 = 1u16 << 11;
const TKDB_HEADER_SIZE: usize = 16;

fn engine_install_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

struct Row {
    hanzi: &'static str,
    tl: &'static str,
    syll: u8,
    freq: u32,
}

fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("composing-space-pin-{pid}-{name}"));
    std::fs::write(&path, bytes).expect("write temp fixture");
    path
}

fn build_tkdb_v3(rows: &[Row]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKDB");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let offset_table_size = rows.len() * 4;
    let mut offsets = Vec::<u32>::with_capacity(rows.len());
    let mut payload = Vec::<u8>::new();
    for row in rows {
        offsets.push((TKDB_HEADER_SIZE + offset_table_size + payload.len()) as u32);
        payload.extend_from_slice(&RANK_NEUTRAL_BITMASK.to_le_bytes());
        payload.extend_from_slice(&row.freq.to_le_bytes());
        payload.push(row.hanzi.len() as u8);
        payload.push(row.tl.len() as u8);
        payload.push(row.syll);
        payload.extend_from_slice(&0u16.to_le_bytes()); // kautian_subtag (v3); 0 = none
        payload.extend_from_slice(row.hanzi.as_bytes());
        payload.extend_from_slice(row.tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

/// Both TPS key families per row, mirroring `create_fst.py:126-141`. For a
/// tone-1 or tone-4 row the two are byte-identical (no mark to add) — that
/// collision IS the bug's root cause, so the fixture must reproduce it
/// rather than paper over it with a synthetic distinguishing key.
// 中文: 逐列同發兩個 TPS 家族鍵(對齊 create_fst.py)。第 1/4 調兩者逐 byte 相同 —
// 中文:   此碰撞正是本 bug 根因,fixture 必須忠實重現,不可用人造鍵繞過。
fn build_dictionary_fst_tps(rows: &[Row]) -> PathBuf {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(rows.len() * 2);
    for (idx, row) in rows.iter().enumerate() {
        let rowid = (idx + 1) as u32;
        let mut push_key = |body: &str| {
            let mut e = Vec::with_capacity(body.len() + 4 + 5);
            e.extend_from_slice(b"tps:");
            e.extend_from_slice(body.as_bytes());
            e.push(SEPARATOR);
            e.extend_from_slice(&rowid.to_le_bytes());
            entries.push(e);
        };
        push_key(&phonetics::tps_notone_from_tl(row.tl));
        push_key(&phonetics::tps_num_from_tl(row.tl));
    }
    entries.sort();
    entries.dedup();
    let path = write_temp("dictionary-tps.fst", &[]);
    let file = std::fs::File::create(&path).expect("create dictionary-tps.fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for entry in &entries {
        builder.insert(entry).expect("fst insert");
    }
    builder.finish().expect("fst finish");
    path
}

/// Per-syllable TPS inventory, toned + toneless, for every syllable of
/// every fixture row (a phrase row contributes each of its syllables).
// 中文: 逐音節 TPS inventory(含調 + 去調);詞列貢獻其每個音節。
fn build_syllables_fst_tps(rows: &[Row]) -> PathBuf {
    let mut keys: Vec<String> = Vec::new();
    for row in rows {
        for token in row.tl.split(['-', ' ']) {
            if token.is_empty() {
                continue;
            }
            keys.push(format!("tps:{}", phonetics::tps_notone_from_tl(token)));
            keys.push(format!("tps:{}", phonetics::tps_num_from_tl(token)));
        }
    }
    keys.sort();
    keys.dedup();
    let path = write_temp("syllables-tps.fst", &[]);
    let file = std::fs::File::create(&path).expect("create syllables-tps.fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    path
}

fn empty_association_bin() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"TKWA");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

/// Three families, each minimal for one axis of the fix:
/// - `ㄒㄧ` open rime: 詩 (si1, unmarked) vs 死 (si2) / 是 (si7). The
///   higher-frequency wrong-tone rows are what the reporter saw.
/// - `ㄐㄧㆵ` stop coda: 這 (tsit4, unmarked) vs 一 (tsit8, dotted coda,
///   the higher frequency). 之 (tsi1) is the strict-prefix control.
/// - `ㄍㄠ` + `ㄉㄞ˪`: 交 (kau1) vs 到 (kau3) / 猴 (kau5) for the pinned
///   leading span, and 交代 (kau1-tài) for the §31 cross-space phrase.
// 中文: 三組 fixture,各釘一個軸:ㄒㄧ 開音節(詩 si1 vs 死 si2/是 si7,錯調列頻率更高,
// 中文:   即回報者看到的);ㄐㄧㆵ 入聲尾(這 tsit4 vs 一 tsit8 帶點且高頻,之 tsi1 為嚴格前綴對照);
// 中文:   ㄍㄠ + ㄉㄞ˪(交 kau1 vs 到 kau3/猴 kau5 釘前導 span,交代 kau1-tài 驗 §31 跨空白整詞)。
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            hanzi: "詩",
            tl: "si",
            syll: 1,
            freq: 50,
        },
        Row {
            hanzi: "死",
            tl: "sí",
            syll: 1,
            freq: 900,
        },
        Row {
            hanzi: "是",
            tl: "sī",
            syll: 1,
            freq: 1000,
        },
        Row {
            hanzi: "這",
            tl: "tsit",
            syll: 1,
            freq: 90,
        },
        Row {
            hanzi: "一",
            tl: "tsi̍t",
            syll: 1,
            freq: 800,
        },
        Row {
            hanzi: "之",
            tl: "tsi",
            syll: 1,
            freq: 40,
        },
        Row {
            hanzi: "交",
            tl: "kau",
            syll: 1,
            freq: 60,
        },
        Row {
            hanzi: "到",
            tl: "kàu",
            syll: 1,
            freq: 700,
        },
        Row {
            hanzi: "猴",
            tl: "kâu",
            syll: 1,
            freq: 300,
        },
        Row {
            hanzi: "交代",
            tl: "kau-tài",
            syll: 2,
            freq: 200,
        },
    ]
}

fn install_fixture_tps() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary-tps.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tps(&rows);
    let assoc_path = write_temp("association-tps.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tps(&rows);
    let paths = LexiconPaths::validated(
        fst_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        assoc_path.to_str().unwrap(),
        syllables_path.to_str().unwrap(),
        2,
    )
    .expect("LexiconPaths::validated");
    LexiconHandle::install(paths).expect("EngineHandle::install");
}

fn config() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tps".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode: 0,
    }
}

fn req(method: Method) -> ComposingRequest {
    ComposingRequest {
        method: Some(method),
    }
}

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` in TPS mode
/// and return the candidate hanji list.
fn fetch_hanji(raw: &str) -> Vec<String> {
    fetch_hanji_with_custom(raw, Vec::new())
}

/// [`fetch_hanji`] with `custom_dictionary.db` entries attached — the
/// lexicon whole-buffer merge, which the empty-list helper never exercises.
/// (The walker's per-edge custom override is NOT reached in TPS mode:
/// `custom_toneless_key` rejects a non-Bopomofo body, and custom romans are
/// TL / POJ. Its pin gate is symmetry for the day that changes; see the
/// comment at that call site.)
// 中文: fetch_hanji + 自訂詞條 — 咬的是 lexicon 整段合併(空清單版測不到)。
// 中文:   walker 每 edge 的 custom override 在 TPS 走不到(custom_toneless_key 要求純注音 body,
// 中文:   而 custom 羅馬字是 TL/POJ);那裡的 pin gate 是為未來對稱,見該處註解。
fn fetch_hanji_with_custom(raw: &str, custom: Vec<CustomDictEntry>) -> Vec<String> {
    let cfg = config();
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: raw.into() })),
        &mut engine,
        &cfg,
    )
    .expect("Start");
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &cfg,
    )
    .expect("EnterContinuous");
    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: Vec::new(),
            now_ms: 0,
            custom_entries: custom,
            enabled_sources_bitmask: 0,
            literal_roman_candidate_disabled: false,
        })),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .filter_map(|cand| cand.hanji)
                .collect()
        })
        .unwrap_or_default()
}

/// Candidate `(hanji, consumed_span_end)` pairs for `raw`, for the tests
/// that care about how much of the buffer a commit would eat.
// 中文: 回傳候選的 (漢字, consumed_span_end),供在意「commit 會吃掉多少 buffer」的測試。
fn fetch_spans(raw: &str) -> Vec<(String, u32)> {
    let cfg = config();
    let mut engine = Engine::new();
    dispatch::handle(
        &req(Method::Start(Start { text: raw.into() })),
        &mut engine,
        &cfg,
    )
    .expect("Start");
    dispatch::handle(
        &req(Method::EnterContinuous(EnterContinuous {})),
        &mut engine,
        &cfg,
    )
    .expect("EnterContinuous");
    let resp = dispatch::handle(
        &req(Method::FetchAtPos(FetchAtPos {
            position: 0,
            frequency_entries: Vec::new(),
            now_ms: 0,
            custom_entries: Vec::new(),
            enabled_sources_bitmask: 0,
            literal_roman_candidate_disabled: false,
        })),
        &mut engine,
        &cfg,
    )
    .expect("FetchAtPos");
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .filter_map(|cand| cand.hanji.map(|h| (h, cand.consumed_span_end)))
                .collect()
        })
        .unwrap_or_default()
}

/// The TPS surface of one TL syllable, as the keyboard emits it.
fn tps(tl: &str) -> String {
    phonetics::tps_num_from_tl(tl)
}

#[test]
fn space_pins_tone_one_on_open_rime() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // The headline case: `ㄒㄧ` + space = "si, first tone". 詩 (si1) only —
    // 死 (si2) and 是 (si7) share the toneless key and outrank it by
    // frequency, which is exactly what the reporter saw.
    let raw = format!("{} ", tps("si"));
    let hanji = fetch_hanji(&raw);
    assert!(
        hanji.iter().any(|h| h == "詩"),
        "{raw:?} must surface 詩 (si1); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "死" || h == "是"),
        "{raw:?} must NOT surface 死 (si2) / 是 (si7); got {hanji:?}"
    );
}

#[test]
fn no_space_still_surfaces_every_tone_on_open_rime() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // No-tone affordance unchanged: without the space every tone stays.
    let raw = tps("si");
    let hanji = fetch_hanji(&raw);
    for expected in ["詩", "死", "是"] {
        assert!(
            hanji.iter().any(|h| h == expected),
            "toneless {raw:?} must surface {expected}; got {hanji:?}"
        );
    }
}

#[test]
fn space_pins_tone_four_on_stop_coda() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // Stop coda: `ㄐㄧㆵ` + space = "tsit, fourth tone". 這 (tsit4) only —
    // 一 (tsi̍t, tone 8) writes the same coda glyph plus a dot and carries
    // ~9x the frequency, so pre-fix it owned the strip.
    let raw = format!("{} ", tps("tsit"));
    let hanji = fetch_hanji(&raw);
    assert!(
        hanji.iter().any(|h| h == "這"),
        "{raw:?} must surface 這 (tsit4); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "一"),
        "{raw:?} must NOT surface 一 (tsit8, marked coda); got {hanji:?}"
    );
    // Strict-prefix control (fixture-coverage rule): 之 (tsi1) is a valid
    // production syllable and a strict prefix of the probed key, so it must
    // be named explicitly. §18 longest-match suppression drops the shorter
    // single syllable, and the tone pin does not resurrect it.
    assert!(
        !hanji.iter().any(|h| h == "之"),
        "{raw:?} must NOT surface the shorter single syllable 之 (§18); got {hanji:?}"
    );
}

#[test]
fn no_space_still_surfaces_tone_eight_on_stop_coda() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    let raw = tps("tsit");
    let hanji = fetch_hanji(&raw);
    for expected in ["這", "一"] {
        assert!(
            hanji.iter().any(|h| h == expected),
            "toneless {raw:?} must surface {expected}; got {hanji:?}"
        );
    }
}

#[test]
fn interior_space_keeps_cross_barrier_phrase() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // §31 regression pin: 交代 (kau1-tài) spans THROUGH the space, so the
    // space is an interior syllable boundary for it, not a tone
    // instruction. The phrase must survive even though its leading
    // syllable is unmarked.
    let raw = format!("{} {}", tps("kau"), tps("tài"));
    let hanji = fetch_hanji(&raw);
    assert!(
        hanji.iter().any(|h| h == "交代"),
        "{raw:?} must still surface 交代 (§31 cross-space phrase); got {hanji:?}"
    );
}

#[test]
fn trailing_space_pins_leading_span_only() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // The span that STOPS on the space is pinned: `ㄍㄠ` + space keeps 交
    // (kau1) and drops 到 (kau3) / 猴 (kau5), both higher-frequency.
    let raw = format!("{} ", tps("kau"));
    let hanji = fetch_hanji(&raw);
    assert!(
        hanji.iter().any(|h| h == "交"),
        "{raw:?} must surface 交 (kau1); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "到" || h == "猴"),
        "{raw:?} must NOT surface 到 (kau3) / 猴 (kau5); got {hanji:?}"
    );
}

#[test]
fn marked_tail_is_not_pinned_by_a_trailing_space() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // A tail that already carries a tone mark took the verbatim toned key,
    // so a trailing space must not re-interpret it as tone 1/4 and empty
    // the strip: `ㄍㄠ˪` + space still surfaces 到 (kau3).
    let raw = format!("{} ", tps("kàu"));
    let hanji = fetch_hanji(&raw);
    assert!(
        hanji.iter().any(|h| h == "到"),
        "{raw:?} must still surface 到 (kau3); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "交"),
        "{raw:?} must NOT surface 交 (kau1, wrong tone); got {hanji:?}"
    );
}

#[test]
fn space_pin_also_gates_custom_dictionary_entries() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // Codex post-impl 2026-08-20 asked for this coverage: a custom entry
    // is appended AFTER dictionary filtering, so without its own gate the
    // "only tone 1/4" promise leaks through the custom source — `ㄒㄧ`␣
    // would still show a tone-7 entry the user did not ask for.
    let marked = vec![CustomDictEntry {
        roman: "sī".into(),
        hanji: Some("侍".into()),
    }];
    let raw = format!("{} ", tps("si"));
    let hanji = fetch_hanji_with_custom(&raw, marked.clone());
    assert!(
        !hanji.iter().any(|h| h == "侍"),
        "{raw:?} must NOT surface a tone-7 custom entry; got {hanji:?}"
    );
    assert!(
        hanji.iter().any(|h| h == "詩"),
        "{raw:?} must still surface 詩 (si1) with a custom entry present; got {hanji:?}"
    );

    // Same entry with no space typed → the custom entry is visible again
    // (the toneless affordance is unchanged for custom too).
    let toneless = tps("si");
    let hanji_toneless = fetch_hanji_with_custom(&toneless, marked);
    assert!(
        hanji_toneless.iter().any(|h| h == "侍"),
        "toneless {toneless:?} must surface the custom entry; got {hanji_toneless:?}"
    );
}

#[test]
fn space_pin_keeps_an_unmarked_custom_entry() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // The other half of the gate: a custom entry whose pinned syllable IS
    // unmarked survives. `sai` (tone 1) under `ㄙㄞ`␣ — a key the fixture
    // dictionary has no row for, so the entry can only arrive through the
    // custom path.
    let unmarked = vec![CustomDictEntry {
        roman: "sai".into(),
        hanji: Some("私".into()),
    }];
    let raw = format!("{} ", tps("sai"));
    let hanji = fetch_hanji_with_custom(&raw, unmarked);
    assert!(
        hanji.iter().any(|h| h == "私"),
        "{raw:?} must surface the tone-1 custom entry; got {hanji:?}"
    );
}

#[test]
fn space_pin_gates_a_custom_stop_coda_entry() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // Stop-coda half: tone 8 (`tsi̍t`) is marked, so `ㄐㄧㆵ`␣ must drop the
    // custom entry while the tone-4 dictionary row 這 stays.
    let marked = vec![CustomDictEntry {
        roman: "tsi̍t".into(),
        hanji: Some("蜀".into()),
    }];
    let raw = format!("{} ", tps("tsit"));
    let hanji = fetch_hanji_with_custom(&raw, marked);
    assert!(
        !hanji.iter().any(|h| h == "蜀"),
        "{raw:?} must NOT surface a tone-8 custom entry; got {hanji:?}"
    );
    assert!(
        hanji.iter().any(|h| h == "這"),
        "{raw:?} must still surface 這 (tsit4); got {hanji:?}"
    );
}

#[test]
fn pinned_candidate_span_consumes_the_trailing_separator() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // §41 — the space the user pressed is a marker, so a candidate covering
    // the whole buffer must consume it too. A span stopping one byte short
    // leaves a lone `" "` pending: invisible (the display seam hides it) but
    // enough to keep the engine composing a phantom buffer.
    let raw = format!("{} ", tps("si"));
    let spans = fetch_spans(&raw);
    let (_, end) = spans
        .iter()
        .find(|(hanji, _)| hanji == "詩")
        .unwrap_or_else(|| panic!("詩 must be a candidate for {raw:?}; got {spans:?}"));
    assert_eq!(
        *end as usize,
        raw.len(),
        "{raw:?}: 詩's span must reach raw len so the commit eats the marker"
    );
}
