//! Hanji conversion in the TPS preedit, typing forward
//! (`docs/architecture/desktop-tps-hanji-conversion-roadmap.md` H1, H2): with
//! `AppConfig.hanji_conversion` present, the closed readings of a TPS tail
//! show as the walker's words and the reading being typed as glyphs.
//! INVARIANT_TPS_PREEDIT_HANJI_CONVERSION (behavioral-invariants §59).
//!
//! Hermetic TPS fixture, the shape of `tps_space_pinned_tone.rs`. Per the
//! fixture rule in `docs/contributing/known-pitfalls.md`, every production
//! syllable that is a strict prefix of a probed one has a control row that
//! is asserted on: 機 (ki, `ㄍㄧ`) under 今 (kin, `ㄍㄧㄣ`), 字 (jī, `ㆢㄧ˫`'s
//! body `ㆢㄧ`) under 日 (ji̍t, `ㆢㄧㆵ˙`), 之 (tsi, `ㄐㄧ`) under 這 (tsit,
//! `ㄐㄧㆵ`).

use composing::api::{CaretDirection, CommitScript, Conversion};
use composing::{requests, Engine, Intent, NailedSegment, Phase};
use protos::engine::composing_request::Method;
use protos::engine::effect::Kind;
use protos::engine::{
    AppConfig, ComposingResponse, DictionarySourceToggles, HanjiConversion, TpsKey,
};
use test_support::{engine_install_lock, write_temp};

use crate::common::{
    build_dictionary_fst_tps, build_syllables_fst_tps, build_tkdb_v4, commit_text, config,
    config_converting, converted_words, empty_association_bin, install_lexicon, req, Fetch, Row,
};

pub(crate) fn row(hanji: &'static str, tl: &'static str, syll: u8, freq: u32) -> Row {
    Row {
        toneless_key: "",
        hanji,
        tl,
        syll,
        freq,
    }
}

/// - `ㄒㄧ`: 詩 (si1) under the higher-frequency 死 (si2) / 是 (si7) — the
///   tone a mark or the Space names decides the word.
/// - 今 + 仔 + 日 and the three-syllable 今仔日: a later reading re-segments
///   the words before it.
/// - `ㄐㄧㆵ`: 這 (tsit4) under the higher-frequency 一 (tsit8) — Space pins
///   the stop coda's unmarked tone.
/// - 機, 字 and 之: the strict-prefix controls.
pub(crate) fn dictionary_rows() -> Vec<Row> {
    vec![
        row("詩", "si", 1, 50),
        row("死", "sí", 1, 900),
        row("是", "sī", 1, 1000),
        row("今", "kin", 1, 1700),
        row("機", "ki", 1, 2000),
        row("仔", "á", 1, 1500),
        row("日", "ji̍t", 1, 1200),
        row("字", "jī", 1, 1300),
        row("今仔日", "kin-á-ji̍t", 3, 1000),
        row("這", "tsit", 1, 90),
        row("一", "tsi̍t", 1, 800),
        row("之", "tsi", 1, 2000),
    ]
}

/// The dictionary rows plus 喇 (lá), a syllable the inventory knows and the
/// dictionary has no word for.
fn install_fixture() {
    install_fixture_with(&[]);
}

/// [`install_fixture`] with `extra` dictionary rows, `(hanji, tl, freq)`
/// of one syllable each.
pub(crate) fn install_fixture_with(extra: &[(&'static str, &'static str, u32)]) {
    let with_extra = || {
        let extra = extra
            .iter()
            .map(|&(hanji, tl, freq)| row(hanji, tl, 1, freq));
        dictionary_rows()
            .into_iter()
            .chain(extra)
            .collect::<Vec<_>>()
    };
    let rows = with_extra();
    let mut syllable_rows = with_extra();
    syllable_rows.push(row("喇", "lá", 1, 1));
    let dict_path = write_temp("dictionary-hanji-conversion.bin", &build_tkdb_v4(&rows));
    let fst_path = build_dictionary_fst_tps(&rows);
    let association_path = write_temp("association-hanji-conversion.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tps(&syllable_rows);
    install_lexicon(&fst_path, &dict_path, &association_path, &syllables_path);
}

/// A fresh engine composing `raw` under `config`.
pub(crate) fn composing_engine(raw: &str, config: &AppConfig) -> (Engine, ComposingResponse) {
    let mut engine = Engine::new();
    let response = engine.apply(Intent::Start { text: raw.into() }, config);
    (engine, response)
}

pub(crate) fn tps_key(engine: &mut Engine, key: &str, config: &AppConfig) -> ComposingResponse {
    requests::handle(
        &req(Method::TpsKey(TpsKey { key: key.into() })),
        engine,
        config,
    )
    .expect("TpsKey")
}

pub(crate) fn display(response: &ComposingResponse) -> &str {
    &response.preedit.as_ref().expect("preedit").display_text
}

pub(crate) fn caret_utf16(response: &ComposingResponse) -> u32 {
    response.preedit.as_ref().expect("preedit").caret_utf16
}

pub(crate) fn raw_input(response: &ComposingResponse) -> &str {
    &response.preedit.as_ref().expect("preedit").raw_input
}

/// The displays the response asks the host to write, in order.
pub(crate) fn preedit_writes(response: &ComposingResponse) -> Vec<&str> {
    response
        .effect
        .iter()
        .filter_map(|effect| match effect.kind.as_ref() {
            Some(Kind::UpdatePreedit(update)) => Some(update.display.as_str()),
            _ => None,
        })
        .collect()
}

/// One `MoveCaret` step.
pub(crate) fn step(
    engine: &mut Engine,
    direction: CaretDirection,
    config: &AppConfig,
) -> ComposingResponse {
    engine.apply(
        Intent::MoveCaret {
            direction: Some(direction),
        },
        config,
    )
}

/// A `CommitContinuous` of the one-syllable `hanji` (TL `tl`) ending at
/// `consumed_bytes` of the tail.
pub(crate) fn pick(hanji: &str, tl: &str, consumed_bytes: usize) -> Intent {
    Intent::CommitContinuous {
        canonical_text: hanji.into(),
        association_tl: tl.into(),
        hanji: Some(hanji.into()),
        consumed_bytes,
        syllable_count: 1,
        script: Some(CommitScript::Lead),
        roman: tl.into(),
    }
}

pub(crate) fn nailed(engine: &Engine) -> Vec<NailedSegment> {
    match engine.snapshot_state().phase {
        Phase::Continuous { nailed, .. } => nailed,
        Phase::Idle => Vec::new(),
    }
}

pub(crate) fn conversion_of(engine: &Engine) -> Option<Conversion> {
    match engine.snapshot_state().phase {
        Phase::Continuous { conversion, .. } => conversion,
        Phase::Idle => None,
    }
}

// trace: raw "ㄒㄧ" has no tone mark and no separator → no closing boundary →
// no conversion; the preedit is the glyphs.
#[test]
fn an_open_reading_stays_glyphs() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄒㄧ", &config);
    assert_eq!(display(&response), "ㄒㄧ");
    assert_eq!(conversion_of(&engine), None);
}

// trace: "ㄒㄧ" + "ˋ" → raw "ㄒㄧˋ" (8 bytes), the edge (0, 8) ends on a tone
// mark → closed_end 8; the toned key `tps:ㄒㄧˋ` holds 死 (sí) alone. One
// UpdatePreedit, already converted; `raw_input` keeps the glyphs; the caret
// is the end of the one-character display.
#[test]
fn a_tone_mark_closes_the_reading_and_the_key_answers_converted() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧ", &config);
    let response = tps_key(&mut engine, "ˋ", &config);
    assert_eq!(display(&response), "死");
    assert_eq!(preedit_writes(&response), vec!["死"]);
    assert_eq!(raw_input(&response), "ㄒㄧˋ");
    assert_eq!(caret_utf16(&response), 1);
    assert_eq!(converted_words(&engine), vec![((0, 8), "死".to_string())]);
}

// INVARIANT_TPS_SPACE_PINS_UNMARKED_TONE (§41) holds for the conversion.
// trace: "ㄒㄧ" + Space → raw "ㄒㄧ " (7 bytes); the shadow "ㄒㄧ" ends on the
// separator barrier, whose raw end already holds the trailing separator
// (`shadow_to_raw_end`'s full-span end, §41) → closed_end 7. The
// closed part is walked with its Space, so the tail pins the unmarked tone:
// 詩 (si1), not the higher-frequency 是 (si7) / 死 (si2).
#[test]
fn space_closes_an_unmarked_reading_with_its_tone_pinned() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧ", &config);
    let response = tps_key(&mut engine, " ", &config);
    assert_eq!(display(&response), "詩");
    assert_eq!(converted_words(&engine), vec![((0, 7), "詩".to_string())]);
}

// trace: "ㄐㄧㆵ " — the stop coda is the body, the Space closes it and pins
// the unmarked tone 4: 這 (tsit), not the higher-frequency 一 (tsi̍t, whose
// dot was not typed). The control 之 (tsi) is a strict prefix and absent.
#[test]
fn space_closes_a_stop_coda_reading_with_tone_four_pinned() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄐㄧㆵ ", &config);
    assert_eq!(display(&response), "這");
    assert_eq!(converted_words(&engine), vec![((0, 10), "這".to_string())]);
}

// A separator typed after a hyphen closes nothing. trace: raw "ㄒㄧ- " — the
// shadow "ㄒㄧ" has the separator barrier at its end, but the reading's raw
// end (6) is followed by the hyphen, which stays pending; a closed part cut
// there would be "ㄒㄧ" without its separator, unpinned, and show 是. The
// tail stays glyphs, the separator hidden as always.
#[test]
fn a_separator_after_a_hyphen_closes_nothing() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄒㄧ- ", &config);
    assert_eq!(display(&response), "ㄒㄧ-");
    assert_eq!(conversion_of(&engine), None);
}

// The open reading is a slice of a TPS buffer even when it holds no glyph.
// trace: "ㄒㄧˋ" closed (8); the hyphen and the separator typed after it are
// the open reading "- ", shown without the separator; the caret is the end
// of the two-character display.
#[test]
fn an_open_reading_without_a_glyph_still_hides_the_separator() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋ-", &config);
    let response = tps_key(&mut engine, " ", &config);
    assert_eq!(raw_input(&response), "ㄒㄧˋ- ");
    assert_eq!(display(&response), "死-");
    assert_eq!(caret_utf16(&response), 2);
}

// A tail the whole-buffer walk cannot span still converts what is closed.
// trace: "ㄒㄧˋㄒˋ" — `ㄒˋ` is no syllable, so no edge leaves offset 8 and the
// buffer has no full path; the edge (0, 8) still closes on its mark.
#[test]
fn the_closed_part_converts_in_front_of_an_unspannable_reading() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄒㄧˋㄒˋ", &config);
    assert_eq!(display(&response), "死ㄒˋ");
    assert_eq!(converted_words(&engine), vec![((0, 8), "死".to_string())]);
}

// A pick and its un-nail go through the same phase constructor. trace:
// "ㄒㄧˋㄒㄧ " → 死詩; a step left puts the caret after 死 (8), whose list
// starts at 0 (H4); picking 是 for those eight bytes nails it and leaves
// "ㄒㄧ " pending, converted again from its own start → 是 + 詩. Backspace
// takes the separator, then ㄧ and ㄒ; on the empty tail it un-nails and the
// restored "ㄒㄧˋ" is the walker's 死 again, not the pick.
#[test]
fn the_tail_after_a_pick_and_an_unnailed_segment_convert() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    step(&mut engine, CaretDirection::Left, &config);
    let response = engine.apply(pick("是", "sī", 8), &config);
    assert_eq!(display(&response), "是詩");
    assert_eq!(converted_words(&engine), vec![((0, 7), "詩".to_string())]);

    for expected in ["是ㄒㄧ", "是ㄒ", "是"] {
        let response = engine.apply(Intent::DeleteBackward, &config);
        assert_eq!(display(&response), expected);
    }
    let response = engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(raw_input(&response), "ㄒㄧˋ");
    assert_eq!(display(&response), "死");
}

// trace: closed "ㄒㄧˋ" → 死; the glyphs typed after it are an open reading
// and show as typed. The closed text is unchanged, so the words are the ones
// the earlier key walked. Space then closes "ㄒㄧ" as 詩 (tone 1 pinned).
#[test]
fn the_open_reading_follows_the_converted_words() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, _) = composing_engine("ㄒㄧˋ", &config);
    let closed_words = converted_words(&engine);

    tps_key(&mut engine, "ㄒ", &config);
    let response = tps_key(&mut engine, "ㄧ", &config);
    assert_eq!(raw_input(&response), "ㄒㄧˋㄒㄧ");
    assert_eq!(display(&response), "死ㄒㄧ");
    assert_eq!(caret_utf16(&response), 3);
    assert_eq!(converted_words(&engine), closed_words);

    let response = tps_key(&mut engine, " ", &config);
    assert_eq!(display(&response), "死詩");
    assert_eq!(
        converted_words(&engine),
        vec![((0, 8), "死".to_string()), ((8, 15), "詩".to_string())]
    );
}

// trace: "ㄍㄧㄣ ㄚˋ" (9 + 1 + 5 bytes) closes on ㄚˋ: no row spans kin-á, so
// the path is 今 (0, 10) + 仔 (10, 15) — the separator belongs to the word
// whose reading it closes. Closing "ㆢㄧㆵ˙" (11 bytes) makes the three-syllable 今仔日
// one edge over the whole closed part: the words before the new reading
// change. The controls 機 (ki) and 字 (jī) are strict prefixes of the typed
// syllables and never appear.
#[test]
fn closing_a_later_reading_resegments_the_words_before_it() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, response) = composing_engine("ㄍㄧㄣ ㄚˋ", &config);
    assert_eq!(display(&response), "今仔");
    assert_eq!(
        converted_words(&engine),
        vec![((0, 10), "今".to_string()), ((10, 15), "仔".to_string())]
    );

    for key in ["ㆢ", "ㄧ", "ㆵ"] {
        tps_key(&mut engine, key, &config);
    }
    let response = tps_key(&mut engine, "˙", &config);
    // trace: the typed U+02D9 after the stop coda ㆵ is stored as the
    // combining U+0307 (`tps_adjust::adjust_tone_eight_dot`); both are 2
    // UTF-8 bytes, so the spans below are unchanged.
    assert_eq!(raw_input(&response), "ㄍㄧㄣ ㄚˋㆢㄧㆵ\u{0307}");
    assert_eq!(display(&response), "今仔日");
    assert_eq!(
        converted_words(&engine),
        vec![((0, 26), "今仔日".to_string())]
    );
    assert!(!display(&response).contains('機') && !display(&response).contains('字'));
}

// The tone-8 dot closes the reading in both encodings: U+02D9 as the
// keyboards type it and the combining U+0307 the dictionary stores.
// trace: "ㆢㄧㆵ" + dot → the stop coda is the body, the dot the tone mark →
// 日 (ji̍t). Without the dot the reading is open: tone 4 closes on Space only.
#[test]
fn the_tone_eight_dot_closes_in_both_encodings() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    for dot in ["\u{02d9}", "\u{0307}"] {
        let (_, response) = composing_engine(&format!("ㆢㄧㆵ{dot}"), &config);
        assert_eq!(display(&response), "日", "dot {dot:?}");
    }
    let (engine, response) = composing_engine("ㆢㄧㆵ", &config);
    assert_eq!(display(&response), "ㆢㄧㆵ");
    assert_eq!(conversion_of(&engine), None);
}

// trace: 喇 (lá) is in the syllable inventory only. "ㄒㄧˋㄌㄚˋ" closes on the
// second mark; the edge (8, 16) has no dictionary word → its glyphs.
#[test]
fn a_word_without_hanji_shows_its_glyphs() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄒㄧˋㄌㄚˋ", &config);
    assert_eq!(display(&response), "死ㄌㄚˋ");
    assert_eq!(
        converted_words(&engine),
        vec![((0, 8), "死".to_string()), ((8, 16), "ㄌㄚˋ".to_string())]
    );
}

// What cannot be closed stays glyphs (H1). trace: a leading tone mark is no
// syllable, so no lattice edge leaves offset 0; a separator followed by a
// tone mark is not a boundary of the whole tail (the toneless ending before
// a mark is refused), and nothing after it closes either.
#[test]
fn an_unclosable_tail_stays_glyphs() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    for raw in ["ˋㄒㄧˋ", "ㄒㄧ ˋ"] {
        let (engine, response) = composing_engine(raw, &config);
        assert_eq!(display(&response), raw.replace(' ', ""), "{raw:?}");
        assert_eq!(conversion_of(&engine), None, "{raw:?}");
    }
}

// trace: the hyphen is stripped from the shadow; closed_end stays 8 (after
// the mark) and the trailing hyphen is the open reading.
#[test]
fn a_trailing_hyphen_stays_outside_the_closed_part() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (engine, response) = composing_engine("ㄒㄧˋ-", &config);
    assert_eq!(display(&response), "死-");
    assert_eq!(converted_words(&engine), vec![((0, 8), "死".to_string())]);
}

// Backspace removes one glyph of the raw buffer (H3). trace: "ㄒㄧˋㄒㄧ " →
// the separator goes → "ㄒㄧ" is open again → "死ㄒㄧ"; three more take the
// open reading and the tone mark of 死 → "ㄒㄧ", nothing closed.
#[test]
fn backspace_reopens_the_reading_it_takes_the_close_from() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, response) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    assert_eq!(display(&response), "死詩");

    let response = engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(display(&response), "死ㄒㄧ");
    assert_eq!(preedit_writes(&response), vec!["死ㄒㄧ"]);

    engine.apply(Intent::DeleteBackward, &config);
    engine.apply(Intent::DeleteBackward, &config);
    let response = engine.apply(Intent::DeleteBackward, &config);
    assert_eq!(raw_input(&response), "ㄒㄧ");
    assert_eq!(display(&response), "ㄒㄧ");
    assert_eq!(conversion_of(&engine), None);
}

// Without the switch nothing converts — the D7 glyph preedit, as today.
#[test]
fn without_the_switch_the_preedit_is_the_glyphs() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config("tps");
    let (mut engine, response) = composing_engine("ㄒㄧˋㄒㄧ", &config);
    assert_eq!(display(&response), "ㄒㄧˋㄒㄧ");
    let response = tps_key(&mut engine, " ", &config);
    assert_eq!(display(&response), "ㄒㄧˋㄒㄧ");
    assert_eq!(conversion_of(&engine), None);
}

// A request that does not ask for the conversion never shows one the state
// still holds, and the next mutation drops it.
#[test]
fn a_request_without_the_switch_ignores_a_held_conversion() {
    let _lock = engine_install_lock();
    install_fixture();
    let (mut engine, response) = composing_engine("ㄒㄧˋ", &config_converting("tps"));
    assert_eq!(display(&response), "死");

    let off = config("tps");
    assert_eq!(display(&engine.snapshot(&off)), "ㄒㄧˋ");
    let fetched = requests::apply(Fetch::default().intent(), &mut engine, &off);
    assert_eq!(display(&fetched), "ㄒㄧˋ");
    assert!(conversion_of(&engine).is_some(), "a read writes nothing");

    let response = tps_key(&mut engine, "ㄒ", &off);
    assert_eq!(display(&response), "ㄒㄧˋㄒ");
    assert_eq!(conversion_of(&engine), None);
}

// The source filter rides the config. trace: every dictionary off resolves
// to the filter 0 (§57) — a conversion walked with every source is not shown
// to that request, and the next key walks again: no dictionary word, so the
// closed reading shows its glyphs.
#[test]
fn a_changed_source_filter_is_not_served_the_old_words() {
    let _lock = engine_install_lock();
    install_fixture();
    let (mut engine, response) = composing_engine("ㄒㄧˋ", &config_converting("tps"));
    assert_eq!(display(&response), "死");

    let every_dictionary_off = AppConfig {
        hanji_conversion: Some(HanjiConversion {}),
        dictionary_sources: Some(DictionarySourceToggles::default()),
        ..config("tps")
    };
    assert_eq!(display(&engine.snapshot(&every_dictionary_off)), "ㄒㄧˋ");

    let response = tps_key(&mut engine, "ㄒ", &every_dictionary_off);
    assert_eq!(display(&response), "ㄒㄧˋㄒ");
    let conversion = conversion_of(&engine).expect("the closed reading is still walked");
    assert_eq!(conversion.enabled_sources_bitmask, 0);
    assert_eq!(conversion.segments[0].display_text, "ㄒㄧˋ");
}

// TL and POJ are untouched by the switch (roadmap § Guard rails): a
// romanization buffer is never converted. trace: `derived_display("si2")` →
// "sí" under both tables.
#[test]
fn a_romanization_buffer_is_never_converted() {
    let _lock = engine_install_lock();
    install_fixture();
    for input_mode in ["tl", "poj"] {
        let (engine, response) = composing_engine("si2", &config_converting(input_mode));
        let (_, without) = composing_engine("si2", &config(input_mode));
        assert_eq!(display(&response), "sí", "{input_mode}");
        assert_eq!(response, without, "{input_mode}");
        assert_eq!(conversion_of(&engine), None, "{input_mode}");
    }
}

// The conversion changes what the preedit shows, not what a commit writes:
// `CommitRaw` still writes the glyphs as typed (commit as shown is H5).
#[test]
fn commit_raw_still_writes_the_glyphs() {
    let _lock = engine_install_lock();
    install_fixture();
    let config = config_converting("tps");
    let (mut engine, response) = composing_engine("ㄒㄧˋㄒㄧ ", &config);
    assert_eq!(display(&response), "死詩");
    let response = engine.apply(Intent::CommitRaw, &config);
    assert_eq!(commit_text(&response).as_deref(), Some("ㄒㄧˋㄒㄧ"));
}
