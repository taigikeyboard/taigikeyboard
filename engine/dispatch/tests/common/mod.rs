//! Helpers the user-data integration suites share.

use prost::Message;
use protos::engine::{
    request, response, user_data_request, AppConfig, OpenUserData, Request, Response,
    UserDataJournal, UserDataRequest,
};

/// A TL request config; `swapped` is `is_hanji_first`.
pub fn tl_config(swapped: bool) -> AppConfig {
    AppConfig {
        input_mode: "tl".to_owned(),
        is_hanji_first: swapped,
        ..AppConfig::default()
    }
}

/// One request through `process_request`.
pub fn roundtrip(config: AppConfig, generation: u64, payload: request::Payload) -> Response {
    let request = Request {
        id: 1,
        config_snapshot: Some(config),
        generation,
        payload: Some(payload),
    };
    Response::decode(dispatch::process_request(&request.encode_to_vec()).as_slice())
        .expect("response decodes")
}

/// One user-data request under the TL config.
#[allow(dead_code)]
pub fn user_data(method: user_data_request::Method) -> Response {
    roundtrip(
        tl_config(false),
        0,
        request::Payload::UserData(UserDataRequest {
            method: Some(method),
        }),
    )
}

/// Opens the engine's user data in `directory`, rollback-journaled as on
/// iOS, and asserts it answered.
#[allow(dead_code)]
pub fn open_user_data(directory: &std::path::Path) {
    let opened = user_data(user_data_request::Method::Open(OpenUserData {
        directory: directory.display().to_string(),
        journal: UserDataJournal::Delete as i32,
        ..OpenUserData::default()
    }));
    assert!(
        matches!(opened.payload, Some(response::Payload::UserData(_))),
        "open answered {opened:?}"
    );
}

/// Installs the production lexicon (`assets/dictionaries/`) once per test process;
/// `false` (callers soft-skip) when the artifacts are absent — run `make dict`.
// Not every suite in this directory needs the lexicon.
#[allow(dead_code)]
pub fn production_lexicon_ready() -> bool {
    static READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *READY.get_or_init(|| {
        let Some(artifacts) = test_support::ProductionArtifacts::locate() else {
            return false;
        };
        let paths = lexicon::LexiconPaths::validated(
            &artifacts.dictionary_fst,
            &artifacts.dictionary_bin,
            &artifacts.association_bin,
            &artifacts.syllables_fst,
            0,
        )
        .expect("validate production LexiconPaths");
        lexicon::EngineHandle::install(paths).is_ok()
    })
}

/// One composing request, with the shell's next-word envelope when given.
#[allow(dead_code)]
pub fn composing(
    config: AppConfig,
    generation: u64,
    method: protos::engine::composing_request::Method,
    next_word_context: Option<protos::engine::NextWordContext>,
) -> protos::engine::ComposingResponse {
    let response = roundtrip(
        config,
        generation,
        request::Payload::Composing(protos::engine::ComposingRequest {
            method: Some(method),
            next_word_context,
        }),
    );
    match response.payload {
        Some(response::Payload::Composing(composing)) => composing,
        other => panic!("composing answered {other:?}"),
    }
}

/// Polls `read` until it holds, at most 5 s: the engine's writes are
/// queued, and this process reads them back through its own connection.
#[cfg(feature = "user-data")]
#[allow(dead_code)]
pub fn eventually(mut read: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if read() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}

/// The `user_association.db` rows once the engine's queued write lands
/// (at most 5 s).
#[cfg(feature = "user-data")]
#[allow(dead_code)]
pub fn wait_for_association_rows(
    reader: &userdata::UserDataStores,
) -> Vec<userdata::AssociationRow> {
    let mut rows = Vec::new();
    assert!(
        eventually(|| {
            rows = reader.association.all_rows().expect("association rows");
            !rows.is_empty()
        }),
        "no pair was recorded"
    );
    rows
}
