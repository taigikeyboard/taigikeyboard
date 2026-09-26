//! User-data slice of the engine bridge: the engine owns the four stores
//! (`docs/architecture/user-data-engine-roadmap.md`), the desktop names the
//! files and reports its picks. Answered only by a shell built with
//! `dispatch/user-data` (roadmap U11); anywhere else the engine refuses and
//! these answer `None`.

use std::path::Path;

use protos::engine::{
    request, response, user_data_request, user_data_response, OpenUserData, RecordUsage,
    UserDataJournal, UserDataOpened, UserDataRequest,
};

use super::bridge::{record_failure, roundtrip};

/// Opens the engine's stores over `directory` — the desktop's one-directory
/// layout, write-ahead logged (U3). Cheap enough for a key path: the engine
/// puts the stores in use at once (a pick reported meanwhile queues behind
/// the open) and finishes opening — the first takeover's re-derivation
/// included — on a thread of its own. The answer is readiness as of now.
pub fn open(directory: &Path) -> Option<UserDataOpened> {
    let answer = user_data(
        user_data_request::Method::Open(OpenUserData {
            directory: directory.display().to_string(),
            journal: UserDataJournal::Wal as i32,
            in_background: true,
            ..OpenUserData::default()
        }),
        "userDataOpen",
    )?;
    match answer {
        user_data_response::Result::Opened(opened) => Some(opened),
        _ => {
            record_failure("userDataOpen", "response carried no open result");
            None
        }
    }
}

/// One pick, as the engine counts it (`RecordUsage`). Best-effort: the
/// engine queues the write, and a failed round-trip is logged, never
/// surfaced to the user.
pub fn record_usage(
    display_text: &str,
    canonical_tl: &str,
    hanji: Option<&str>,
    frequency_recording_enabled: bool,
) {
    user_data(
        user_data_request::Method::RecordUsage(RecordUsage {
            display_text: display_text.to_owned(),
            canonical_tl: canonical_tl.to_owned(),
            hanji: hanji.filter(|hanji| !hanji.is_empty()).map(str::to_owned),
            frequency_recording_disabled: !frequency_recording_enabled,
        }),
        "userDataRecordUsage",
    );
}

fn user_data(method: user_data_request::Method, op: &str) -> Option<user_data_response::Result> {
    let payload = request::Payload::UserData(UserDataRequest {
        method: Some(method),
    });
    match roundtrip(payload, op, 0, None)? {
        response::Payload::UserData(answer) => answer.result,
        _ => {
            record_failure(op, "response carried no user-data payload");
            None
        }
    }
}
