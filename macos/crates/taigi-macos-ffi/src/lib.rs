//! The one static library the macOS input method links
//! (docs/architecture/macos-desktop-core-roadmap.md D1, D2).
//!
//! Two swift-bridge seams in one archive: the engine's, unchanged, from
//! `engine/swift-ffi` (`process_request_bytes` and the logger calls), and the
//! desktop shell's, `desktop_request_bytes`, defined here. Both are bytes in,
//! bytes out — protobuf envelopes, no handle crosses, no `unsafe` here
//! (docs/contributing/rust-ffi-safety.md §1.1). The shell's requests reach
//! the `taigi-desktop-core` runtime (`runtime.rs`), the composing session
//! (`session.rs`, with `key_translation.rs`), the key rules (`key_rules.rs`),
//! the Manage Dictionaries search (`dictionary_search.rs`) and the key-path
//! settings snapshot each request may carry (`settings.rs`).

// Links the engine seam into this archive. Nothing here calls into it; without
// the `extern crate` rustc would not link an rlib this crate never names, and
// the archive would lack the engine's exports.
extern crate rust_taigi;

#[cfg(test)]
mod chord_rules;
mod dictionary_search;
mod key_rules;
mod key_translation;
mod runtime;
mod session;
mod settings;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::LazyLock;

use prost::Message;
use protos::engine::ErrorCode;

/// prost types for `proto/desktop_shell.proto` (package `taigi.desktop_shell`).
#[allow(clippy::all, clippy::pedantic)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/taigi.desktop_shell.rs"));
}

use proto::{DesktopRequest, DesktopResponse};
use runtime::Shell;

/// The one shell the bridge serves (the runtime is a process singleton,
/// roadmap D2).
static SHELL: LazyLock<Shell> = LazyLock::new(Shell::default);

// Bridge module. Doc comments live OUTSIDE this block — swift-bridge's parser
// rejects `///` on the items inside.
//
// `desktop_request_bytes`: decodes a `taigi.desktop_shell.DesktopRequest`,
// answers with an encoded `taigi.desktop_shell.DesktopResponse`. Always a
// decodable response — never panics across the seam.
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        fn desktop_request_bytes(bytes: &[u8]) -> Vec<u8>;
    }
}

fn desktop_request_bytes(bytes: &[u8]) -> Vec<u8> {
    answer(|| respond(&SHELL, bytes))
}

/// The seam's panic boundary (rust-ffi-safety.md §1.2): a panic in `respond`
/// answers FAIL_INTERNAL. Separate from `desktop_request_bytes` only so the
/// tests can drive this exact path with a panicking closure.
fn answer(respond: impl FnOnce() -> DesktopResponse) -> Vec<u8> {
    catch_unwind(AssertUnwindSafe(respond))
        .unwrap_or_else(|_| {
            log::error!("desktop request panicked");
            error_response(ErrorCode::FailInternal)
        })
        .encode_to_vec()
}

fn respond(shell: &Shell, bytes: &[u8]) -> DesktopResponse {
    // Checked before decoding, as on the engine seam (rust-ffi-safety.md §1.4).
    if dispatch::is_request_too_large(bytes.len()) {
        return error_response(ErrorCode::FailInvariant);
    }
    let request = match DesktopRequest::decode(bytes) {
        Ok(request) => request,
        Err(error) => {
            log::warn!("desktop request decode failed: {error}");
            return error_response(ErrorCode::FailParse);
        }
    };
    // Checked before the snapshot is applied, so a request that cannot run
    // changes nothing.
    let DesktopRequest {
        request: Some(request),
        settings,
    } = request
    else {
        log::warn!("desktop request has no variant");
        return error_response(ErrorCode::FailInvariant);
    };
    match shell.serve(request, settings.as_ref()) {
        Ok(reply) => DesktopResponse {
            error: ErrorCode::Ok as i32,
            reply: Some(reply),
        },
        Err(refusal) => {
            log::warn!("desktop request refused: {refusal:?}");
            error_response(ErrorCode::FailInvariant)
        }
    }
}

fn error_response(code: ErrorCode) -> DesktopResponse {
    DesktopResponse {
        error: code as i32,
        reply: None,
    }
}

/// Request builders the unit tests of every module share.
#[cfg(test)]
mod test_support {
    use crate::proto::{
        desktop_request, setting_value, ConfigureRequest, KeyEvent, PrepareRequest, SettingEntry,
        SettingValue, VersionRequest,
    };
    use crate::runtime::Shell;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

    /// The `NSEvent.ModifierFlags` bits the translation drops — they say how
    /// a key was reached, not which key (`key_translation.rs`).
    pub(crate) const CAPS_LOCK: u64 = 1 << 16;
    pub(crate) const NUMERIC_PAD: u64 = 1 << 21;
    pub(crate) const FUNCTION: u64 = 1 << 23;

    /// A key event as AppKit reports one: `characters` typed with
    /// `modifier_flags` held, the same with none held.
    pub(crate) fn key_event(
        characters: &str,
        modifier_flags: u64,
        special_key: Option<u32>,
    ) -> KeyEvent {
        KeyEvent {
            key_code: None,
            characters: Some(characters.to_owned()),
            characters_ignoring_modifiers: Some(characters.to_owned()),
            modifier_flags,
            special_key,
        }
    }

    pub(crate) fn version() -> desktop_request::Request {
        desktop_request::Request::Version(VersionRequest {})
    }

    /// The dictionaries the app ships, as the repository holds them.
    pub(crate) fn repository_dictionaries() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/dictionaries")
    }

    /// The process has one engine, whichever `Shell` drives it: every test
    /// that reaches it holds this lock, or `cargo test`'s threads would
    /// interleave their compositions.
    pub(crate) fn engine_lock() -> MutexGuard<'static, ()> {
        static ENGINE: Mutex<()> = Mutex::new(());
        ENGINE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A shell configured with the shipped dictionaries (no data directory:
    /// nothing is learned) and prepared once, with the engine lock held
    /// for the caller.
    pub(crate) fn engine_shell() -> (MutexGuard<'static, ()>, &'static Shell) {
        static SHELL: LazyLock<Shell> = LazyLock::new(|| {
            let shell = Shell::default();
            let dictionaries = repository_dictionaries();
            let configure = configure_request(None, Some(&dictionaries));
            shell
                .serve(desktop_request::Request::Configure(configure), None)
                .expect("configured");
            shell
                .serve(desktop_request::Request::Prepare(PrepareRequest {}), None)
                .expect("prepared");
            shell
        });
        let engine = engine_lock();
        (engine, &SHELL)
    }

    /// A token no other test has used — each test starts its own session.
    pub(crate) fn next_token() -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn configure_request(
        data: Option<&Path>,
        dictionaries: Option<&Path>,
    ) -> ConfigureRequest {
        ConfigureRequest {
            data_directory: data.map(|path| path.display().to_string()),
            dictionaries_directory: dictionaries.map(|path| path.display().to_string()),
            dictionary_stamp: 30613,
            system_locale: "zh-Hant-TW".to_owned(),
        }
    }

    fn entry(name: &str, value: setting_value::Value) -> SettingEntry {
        SettingEntry {
            name: name.to_owned(),
            value: Some(SettingValue { value: Some(value) }),
        }
    }

    pub(crate) fn boolean(name: &str, value: bool) -> SettingEntry {
        entry(name, setting_value::Value::Boolean(value))
    }

    pub(crate) fn text(name: &str, value: &str) -> SettingEntry {
        entry(name, setting_value::Value::Text(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{
        desktop_request, desktop_response, PrepareReply, PrepareRequest, SettingsSnapshot,
        VersionReply,
    };
    use std::path::Path;
    use test_support::{boolean, configure_request, version};

    fn decode(bytes: &[u8]) -> DesktopResponse {
        DesktopResponse::decode(bytes).expect("the seam always answers a decodable response")
    }

    fn send(request: &DesktopRequest) -> DesktopResponse {
        decode(&desktop_request_bytes(&request.encode_to_vec()))
    }

    fn version_request() -> DesktopRequest {
        DesktopRequest {
            request: Some(version()),
            settings: None,
        }
    }

    fn version_reply() -> Option<desktop_response::Reply> {
        // trace: CARGO_PKG_VERSION = macos/Cargo.toml [workspace.package] version.
        Some(desktop_response::Reply::Version(VersionReply {
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }))
    }

    /// A field number no `DesktopRequest` field has.
    const UNKNOWN_FIELD: u32 = 99;

    /// A Version request padded with one unknown length-delimited field
    /// ([`UNKNOWN_FIELD`]) to exactly `total_len` bytes — still a valid
    /// request, which the decoder answers like the bare one.
    fn padded_version_request(total_len: usize) -> Vec<u8> {
        let mut bytes = version_request().encode_to_vec();
        let header_len = bytes.len() + prost::encoding::key_len(UNKNOWN_FIELD);
        // The payload length's own varint counts toward `total_len`.
        let payload_len = (1..=10)
            .find_map(|varint_len| {
                let len = total_len - header_len - varint_len;
                (prost::encoding::encoded_len_varint(len as u64) == varint_len).then_some(len)
            })
            .expect("a length whose varint fits");
        prost::encoding::encode_key(
            UNKNOWN_FIELD,
            prost::encoding::WireType::LengthDelimited,
            &mut bytes,
        );
        prost::encoding::encode_varint(payload_len as u64, &mut bytes);
        bytes.resize(total_len, 0);
        bytes
    }

    #[test]
    fn version_request_answers_the_crate_version() {
        let response = send(&version_request());

        assert_eq!(response.error, ErrorCode::Ok as i32);
        assert_eq!(response.reply, version_reply());
    }

    /// rust-ffi-safety.md §6 T1, on this seam's own catch path.
    #[test]
    fn panic_answers_fail_internal() {
        let response = decode(&answer(|| panic!("injected panic")));

        assert_eq!(response.error, ErrorCode::FailInternal as i32);
        assert_eq!(response.reply, None);
    }

    /// rust-ffi-safety.md §6 T3 for a stateless request: concurrent calls
    /// each answer. (The stateful ones: `concurrent_snapshots_all_answer`,
    /// `session.rs` `concurrent_keys_each_run_under_their_own_snapshot`.)
    #[test]
    fn concurrent_version_requests_all_answer() {
        let request = version_request().encode_to_vec();
        std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| decode(&desktop_request_bytes(&request))))
                .collect();
            for worker in workers {
                let response = worker.join().expect("no panic escapes the seam");
                assert_eq!(response.error, ErrorCode::Ok as i32);
                assert_eq!(response.reply, version_reply());
            }
        });
    }

    /// rust-ffi-safety.md §6 T4: malformed bytes answer FAIL_PARSE.
    #[test]
    fn malformed_bytes_answer_fail_parse() {
        let response = decode(&desktop_request_bytes(&[0xFF, 0xFF, 0xFF, 0xFF]));

        assert_eq!(response.error, ErrorCode::FailParse as i32);
        assert_eq!(response.reply, None);
    }

    #[test]
    fn request_without_variant_answers_fail_invariant() {
        let response = send(&DesktopRequest {
            request: None,
            settings: None,
        });

        assert_eq!(response.error, ErrorCode::FailInvariant as i32);
        assert_eq!(response.reply, None);
    }

    /// A newer Swift side's variant this library does not know: prost keeps
    /// it as an unknown field, so the request decodes with no variant.
    #[test]
    fn unknown_variant_answers_fail_invariant() {
        // trace: field 99, wire type 2 → key 99 << 3 | 2 = 794 = varint [0x9A, 0x06]; length 0.
        let response = decode(&desktop_request_bytes(&[0x9A, 0x06, 0x00]));

        assert_eq!(response.error, ErrorCode::FailInvariant as i32);
        assert_eq!(response.reply, None);
    }

    /// rust-ffi-safety.md §6 T5: a valid request of exactly the cap is
    /// answered; one byte more is refused before decoding.
    #[test]
    fn request_size_cap_is_inclusive() {
        let at_cap = padded_version_request(dispatch::MAX_REQUEST_BYTES);
        assert_eq!(at_cap.len(), dispatch::MAX_REQUEST_BYTES);
        let response = decode(&desktop_request_bytes(&at_cap));
        assert_eq!(response.error, ErrorCode::Ok as i32);
        assert_eq!(response.reply, version_reply());

        let over_cap = padded_version_request(dispatch::MAX_REQUEST_BYTES + 1);
        let response = decode(&desktop_request_bytes(&over_cap));
        assert_eq!(response.error, ErrorCode::FailInvariant as i32);
        assert_eq!(response.reply, None);
    }

    // ---- The runtime requests, through the same decode / encode path ----

    fn send_to(shell: &Shell, request: desktop_request::Request) -> DesktopResponse {
        send_with(shell, request, None)
    }

    fn send_with(
        shell: &Shell,
        request: desktop_request::Request,
        settings: Option<SettingsSnapshot>,
    ) -> DesktopResponse {
        let bytes = DesktopRequest {
            request: Some(request),
            settings,
        }
        .encode_to_vec();
        decode(&answer(|| respond(shell, &bytes)))
    }

    fn configure(data: Option<&Path>, dictionaries: Option<&Path>) -> desktop_request::Request {
        desktop_request::Request::Configure(configure_request(data, dictionaries))
    }

    fn prepare() -> desktop_request::Request {
        desktop_request::Request::Prepare(PrepareRequest {})
    }

    /// A refused request answers FAIL_INVARIANT with no reply, and the next
    /// valid one is served. (Which requests are refused: `runtime.rs`,
    /// `settings.rs`.)
    #[test]
    fn a_refusal_is_fail_invariant_and_the_next_request_is_served() {
        let shell = Shell::default();
        let refused = send_to(&shell, prepare());
        assert_eq!(refused.error, ErrorCode::FailInvariant as i32);
        assert_eq!(refused.reply, None);

        let configured = send_to(&shell, configure(None, None));
        assert_eq!(configured.error, ErrorCode::Ok as i32);
        assert!(
            matches!(&configured.reply, Some(desktop_response::Reply::Configure(reply)) if !reply.settings.is_empty()),
            "{:?}",
            configured.reply
        );
        let snapshot = SettingsSnapshot {
            entries: vec![boolean("autoSpaceEnabled", true)],
        };
        let accepted = send_with(&shell, version(), Some(snapshot));
        assert_eq!(accepted.error, ErrorCode::Ok as i32);
        assert_eq!(accepted.reply, version_reply());
    }

    /// The session requests through the seam: a request missing a field it
    /// cannot run without answers FAIL_INVARIANT with no reply; a complete
    /// one answers a session reply. (Every refusal: `session.rs`
    /// `requests_that_cannot_run_are_refused`.)
    #[test]
    fn session_requests_answer_through_the_seam() {
        use proto::{
            ActivateRequest, CommitForSymbolPickerRequest, InsertSymbolRequest, PanelState,
            RepresentRequest,
        };
        let (_engine, shell) = test_support::engine_shell();
        let token = test_support::next_token();
        let panel = Some(PanelState::default());
        for request in [
            desktop_request::Request::CommitForSymbolPicker(CommitForSymbolPickerRequest {
                token,
                panel: None,
            }),
            desktop_request::Request::InsertSymbol(InsertSymbolRequest {
                token,
                symbol: String::new(),
                panel: panel.clone(),
            }),
            desktop_request::Request::Represent(RepresentRequest {
                token: 0,
                refetch: false,
                panel: panel.clone(),
            }),
        ] {
            let refused = send_to(shell, request);
            assert_eq!(refused.error, ErrorCode::FailInvariant as i32);
            assert_eq!(refused.reply, None);
        }
        let activated = send_to(
            shell,
            desktop_request::Request::Activate(ActivateRequest { token }),
        );
        assert_eq!(activated.error, ErrorCode::Ok as i32);
        let inserted = send_to(
            shell,
            desktop_request::Request::InsertSymbol(InsertSymbolRequest {
                token,
                symbol: "，".to_owned(),
                panel,
            }),
        );
        assert_eq!(inserted.error, ErrorCode::Ok as i32);
        assert!(
            matches!(
                &inserted.reply,
                Some(desktop_response::Reply::Session(reply))
                    if reply.effects == [proto::Effect {
                        effect: Some(proto::effect::Effect::InsertText(proto::InsertText {
                            text: "，".to_owned(),
                        })),
                    }]
            ),
            "{:?}",
            inserted.reply
        );
    }

    /// A snapshot refused at the seam answers FAIL_INVARIANT with no reply.
    #[test]
    fn a_refused_snapshot_is_fail_invariant() {
        let shell = Shell::default();
        let bad = SettingsSnapshot {
            entries: vec![boolean("inputMode", true)],
        };
        let refused = send_with(&shell, version(), Some(bad));
        assert_eq!(refused.error, ErrorCode::FailInvariant as i32);
        assert_eq!(refused.reply, None);
    }

    /// rust-ffi-safety.md §6 T3 on runtime state: snapshots from eight
    /// threads each answer and leave one whole snapshot. (Not a TSan run.)
    #[test]
    fn concurrent_snapshots_all_answer() {
        let shell = Shell::default();
        std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|index| {
                    let shell = &shell;
                    scope.spawn(move || {
                        send_with(
                            shell,
                            version(),
                            Some(SettingsSnapshot {
                                entries: vec![boolean("autoSpaceEnabled", index % 2 == 0)],
                            }),
                        )
                    })
                })
                .collect();
            for worker in workers {
                let response = worker.join().expect("no panic escapes the seam");
                assert_eq!(response.error, ErrorCode::Ok as i32);
            }
        });
    }

    /// The launch bring-up end to end: the shipped dictionaries install
    /// under the stamp, the user data opens under the Mac's rollback
    /// journal, and a second Prepare repeats the first answer without a
    /// second install or open.
    #[test]
    fn prepare_installs_the_lexicon_and_opens_the_user_data_once() {
        let _engine = test_support::engine_lock();
        let dictionaries = test_support::repository_dictionaries();
        let data = tempfile::tempdir().expect("a temporary data directory");
        let shell = Shell::default();
        assert_eq!(
            send_to(&shell, configure(Some(data.path()), Some(&dictionaries))).error,
            ErrorCode::Ok as i32
        );

        let first = send_to(&shell, prepare());
        assert_eq!(first.error, ErrorCode::Ok as i32);
        let Some(desktop_response::Reply::Prepare(PrepareReply {
            lexicon: Some(stats),
        })) = first.reply.clone()
        else {
            panic!("expected lexicon stats, got {:?}", first.reply);
        };
        assert!(stats.dictionary_record_count > 0);
        assert!(stats.prefix_index_entry_count > 0);
        assert_eq!(send_to(&shell, prepare()), first, "once-only");

        // A page request queues behind the background open, so its answer
        // means the stores are open.
        taigi_desktop_core::engine::user_data::list_custom_entries("", 1, 0)
            .expect("the user data opened");
        for store in [
            "user_frequency.db",
            "user_association.db",
            "custom_dictionary.db",
            "learned_phrases.db",
        ] {
            let path = data.path().join(store);
            let header = std::fs::read(&path).unwrap_or_else(|e| panic!("{store}: {e}"));
            // SQLite header bytes 18 / 19 (write / read format version):
            // 1 = rollback journal, 2 = write-ahead log.
            assert_eq!(&header[18..20], &[1, 1], "{store} is not in DELETE mode");
            assert!(
                !data.path().join(format!("{store}-wal")).exists(),
                "{store} has a write-ahead log"
            );
        }

        // The Manage Dictionaries search over these stores, as iOS and
        // Android answer it (`DictionarySearchService`): a custom entry
        // leads, wears the supplements' one badge, and offers the web
        // lookups for its own romanization.
        taigi_desktop_core::engine::user_data::save_custom_entry("", "tsia̍h-pá", "食飽")
            .expect("saved");
        let search = desktop_request::Request::DictionarySearch(proto::DictionarySearchRequest {
            query: "tsiahpa".to_owned(),
        });
        let tl = SettingsSnapshot {
            entries: vec![test_support::text("inputMode", "tl")],
        };
        let Some(desktop_response::Reply::DictionarySearch(found)) =
            send_with(&shell, search, Some(tl)).reply
        else {
            panic!("expected a search reply");
        };
        let first = found.hits.first().expect("hits");
        assert!(first.key.starts_with("custom:"), "{found:?}");
        assert_eq!(first.hanji.as_deref(), Some("食飽"));
        assert_eq!(
            first.badge_keys,
            [taigi_desktop_core::strings::StringKey::DictionarySupplementSectionTitle.as_str()]
        );
        assert!(first.moe_url.is_some() && first.chhoe_url.is_some());
    }
}
