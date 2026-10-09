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
