//! The process's desktop runtime behind the seam
//! (docs/architecture/macos-desktop-core-roadmap.md D3): `Configure` builds
//! it once, `Prepare` brings the engine up, the session requests drive the
//! composition (`session.rs`), and the key-path settings snapshot a request
//! carries is put in force before it runs. One [`Shell`] serves the bridge;
//! tests build their own.
//!
//! Launch, not first key (inventory C6): Swift sends `Prepare` from
//! `applicationDidFinishLaunching`, so the lexicon is installed and the user
//! data opened at launch as before. `settings_store` is `None`: nothing here
//! writes settings, and the core's launch reconciliation of the Windows /
//! Linux shortcut registries never runs on the Mac, whose global shortcuts
//! stay in Swift (roadmap D5).

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use taigi_desktop_core::platform::DesktopPlatform;
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};

use crate::key_rules;
use crate::proto::{
    desktop_request, desktop_response, ConfigureReply, ConfigureRequest, LexiconStats,
    PrepareReply, SettingsSnapshot, VersionReply,
};
use crate::session::Session;
use crate::settings::{self, SnapshotSettings};

/// Which desktop this is: every desktop-core rule that differs per desktop
/// is handed it (roadmap D6) — the `DELETE` user-data journal, the Mac
/// chord grammar.
pub(crate) const DESKTOP_PLATFORM: DesktopPlatform = DesktopPlatform::MacOS;

/// Why a request was refused; the seam answers FAIL_INVARIANT.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    AlreadyConfigured,
    NotConfigured,
    Settings(settings::SettingsRefusal),
    /// A field the request cannot run without was left unset (proto3 would
    /// otherwise read it as zero / empty and run anyway).
    Missing(&'static str),
}

#[derive(Default)]
pub(crate) struct Shell {
    /// Exists before `Configure`, so a request may carry a snapshot first.
    settings: Arc<SnapshotSettings>,
    runtime: OnceLock<DesktopRuntime>,
    /// Held for one whole request — its snapshot, its dispatch, its reply —
    /// so a request runs under the settings it carried even when another
    /// thread sends one (roadmap D4: the calls come from the main thread;
    /// a stray one is serialised, not undefined). Taken before the
    /// coordinator, never after.
    session: Mutex<Session>,
}

impl Shell {
    /// One request: the snapshot it carries put in force, then the request
    /// run under it. A request that is refused, whatever refused it, or
    /// ignored — a token that does not own the engine changes nothing —
    /// leaves the previous snapshot in force. (One that panics leaves its
    /// own: Swift answers FAIL_INTERNAL with a `Cancel` that carries none,
    /// so the `Cancel` runs under it.) The key rules (`key_rules.rs`) read the
    /// snapshot they carry and never put it in force.
    pub(crate) fn serve(
        &self,
        request: desktop_request::Request,
        snapshot: Option<&SettingsSnapshot>,
    ) -> Result<desktop_response::Reply, Refusal> {
        if let Some(answer) = key_rules::answer(&request, snapshot) {
            return answer;
        }
        let mut session = self.lock_session();
        let previous = snapshot
            .map(|snapshot| settings::document_from(&snapshot.entries))
            .transpose()
            .map_err(Refusal::Settings)?
            .map(|document| self.settings.replace(Arc::new(document)));
        let reply = self.dispatch(request, &mut session);
        let changed_nothing = match &reply {
            Ok(desktop_response::Reply::Session(session)) => session.ignored,
            Ok(_) => false,
            Err(_) => true,
        };
        if let Some(previous) = previous.filter(|_| changed_nothing) {
            self.settings.replace(previous);
        }
        reply
    }

    fn dispatch(
        &self,
        request: desktop_request::Request,
        session: &mut Session,
    ) -> Result<desktop_response::Reply, Refusal> {
        use desktop_request::Request;
        use desktop_response::Reply;
        match request {
            Request::Version(_) => Ok(Reply::Version(VersionReply {
                version: env!("CARGO_PKG_VERSION").to_owned(),
            })),
            Request::Configure(configure) => self.configure(configure).map(Reply::Configure),
            Request::Prepare(_) => self.prepare().map(Reply::Prepare),
            Request::Activate(activate) => session
                .activate(self.runtime()?, &activate)
                .map(Reply::Session),
            Request::Key(key) => session.key(self.runtime()?, &key).map(Reply::Session),
            Request::CommitComposition(commit) => session
                .commit_composition(self.runtime()?, &commit)
                .map(Reply::Session),
            Request::Cancel(cancel) => session.cancel(self.runtime()?, &cancel).map(Reply::Session),
            Request::Release(release) => session
                .release(self.runtime()?, &release)
                .map(Reply::Session),
            Request::CommitForSymbolPicker(commit) => session
                .commit_for_symbol_picker(self.runtime()?, &commit)
                .map(Reply::Session),
            Request::InsertSymbol(insert) => session
                .insert_symbol(self.runtime()?, &insert)
                .map(Reply::Session),
            Request::TpsKeyboardPress(press) => session
                .tps_keyboard_press(self.runtime()?, &press)
                .map(Reply::Session),
            Request::Represent(represent) => session
                .represent(self.runtime()?, &represent)
                .map(Reply::Session),
            // Answered by `key_rules::answer` before the session is locked.
            Request::Press(_)
            | Request::Chord(_)
            | Request::ComposingShortcuts(_)
            | Request::SymbolPickerKey(_)
            | Request::SwitchInputMode(_)
            | Request::TpsKeyboardRows(_) => Err(Refusal::Missing("a session request")),
        }
    }

    /// The session requests' runtime. `Prepare` is sent at launch before
    /// any of them (inventory C6); one that came first would compose on an
    /// engine with no lexicon, as a launch whose install failed does.
    fn runtime(&self) -> Result<&DesktopRuntime, Refusal> {
        self.runtime.get().ok_or(Refusal::NotConfigured)
    }

    /// A poisoned lock is recovered: a panic inside one request must not
    /// fail every later one (D4 consumes each key that fails). The list it
    /// guards is dropped on the next handover or hidden window anyway.
    fn lock_session(&self) -> MutexGuard<'_, Session> {
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Builds the runtime once. Building one has no side effect, so a
    /// second (or a racing) `Configure` is refused whole when it cannot be
    /// published.
    pub(crate) fn configure(&self, request: ConfigureRequest) -> Result<ConfigureReply, Refusal> {
        let runtime = DesktopRuntime::new(self.runtime_parts(request));
        self.runtime
            .set(runtime)
            .map_err(|_| Refusal::AlreadyConfigured)?;
        Ok(ConfigureReply {
            settings: settings::descriptors(),
        })
    }

    /// What `Configure` builds the runtime from. Each directory is
    /// optional on its own, as at launch before: no data directory skips
    /// the user-data open, no dictionaries directory skips the lexicon.
    fn runtime_parts(&self, request: ConfigureRequest) -> RuntimeParts {
        let dictionaries = request.dictionaries_directory.map(PathBuf::from);
        let system_locale = request.system_locale;
        RuntimeParts {
            settings: Arc::clone(&self.settings) as _,
            settings_store: None,
            data_directory: request.data_directory.map(PathBuf::from),
            dictionaries: Box::new(move || dictionaries.clone()),
            dictionary_version: request.dictionary_stamp,
            system_locale: Box::new(move || system_locale.clone()),
            platform: DESKTOP_PLATFORM,
        }
    }

    /// The lexicon installed and the user data opened — once; a repeat
    /// answers the first result without touching the engine.
    pub(crate) fn prepare(&self) -> Result<PrepareReply, Refusal> {
        let runtime = self.runtime()?;
        let lexicon = runtime
            .prepare_for_first_key()
            .lexicon
            .map(|stats| LexiconStats {
                dictionary_record_count: stats.dictionary_record_count,
                prefix_index_entry_count: stats.prefix_index_entry_count,
            });
        Ok(PrepareReply { lexicon })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{ReleaseRequest, SettingEntry};
    use crate::test_support::{configure_request, text, version};
    use std::path::Path;
    use taigi_desktop_core::settings::{InputMode, SettingChoice, SettingsProvider};

    /// trace: the request's fields reach RuntimeParts as given; the platform
    /// is MacOS (journal DELETE — desktop-core `engine/user_data.rs`
    /// `each_desktop_opens_its_stores_under_its_own_journal`).
    #[test]
    fn configure_builds_the_runtime_from_the_request() {
        let shell = Shell::default();
        let parts = shell.runtime_parts(configure_request(
            Some(Path::new("/data")),
            Some(Path::new("/Resources")),
        ));
        assert_eq!(parts.platform, DesktopPlatform::MacOS);
        assert_eq!(parts.dictionary_version, 30613);
        assert_eq!(parts.data_directory, Some(PathBuf::from("/data")));
        assert_eq!((parts.dictionaries)(), Some(PathBuf::from("/Resources")));
        assert_eq!((parts.system_locale)(), "zh-Hant-TW");
        assert!(parts.settings_store.is_none(), "nothing writes settings");

        let parts = shell.runtime_parts(configure_request(None, None));
        assert_eq!(parts.data_directory, None);
        assert_eq!((parts.dictionaries)(), None);
    }

    #[test]
    fn configure_is_once_only() {
        let shell = Shell::default();
        let reply = shell.configure(configure_request(None, None)).unwrap();
        assert_eq!(reply.settings, settings::descriptors());
        assert_eq!(
            shell.configure(configure_request(None, None)),
            Err(Refusal::AlreadyConfigured)
        );
    }

    #[test]
    fn prepare_needs_configure() {
        assert_eq!(Shell::default().prepare(), Err(Refusal::NotConfigured));
    }

    /// No directories: Prepare installs nothing and opens nothing, every
    /// time — and never reaches the engine.
    #[test]
    fn prepare_without_directories_brings_nothing_up() {
        let shell = Shell::default();
        shell.configure(configure_request(None, None)).unwrap();
        assert_eq!(shell.prepare(), Ok(PrepareReply { lexicon: None }));
        assert_eq!(shell.prepare(), Ok(PrepareReply { lexicon: None }));
    }

    fn snapshot(entries: Vec<SettingEntry>) -> SettingsSnapshot {
        SettingsSnapshot { entries }
    }

    fn input_mode(shell: &Shell) -> InputMode {
        shell.settings.current().engine_settings().input_mode
    }

    /// The runtime reads the snapshot a request carried — before or after
    /// Configure; a request without one keeps the last; a refused snapshot
    /// refuses the request and keeps the last.
    #[test]
    fn the_runtime_reads_the_snapshot_a_request_carries() {
        let shell = Shell::default();
        let poj = snapshot(vec![text("inputMode", "poj")]);
        shell.serve(version(), Some(&poj)).unwrap();
        shell
            .serve(
                desktop_request::Request::Configure(configure_request(None, None)),
                None,
            )
            .unwrap();
        let runtime = shell.runtime.get().unwrap();
        assert_eq!(
            runtime.settings.current().engine_settings().input_mode,
            InputMode::Poj
        );
        shell.serve(version(), None).unwrap();
        assert_eq!(
            input_mode(&shell),
            InputMode::Poj,
            "no snapshot keeps the last"
        );

        let mut refused = poj.clone();
        refused.entries.push(text("notASetting", ""));
        assert!(matches!(
            shell.serve(version(), Some(&refused)),
            Err(Refusal::Settings(_))
        ));
        assert_eq!(
            input_mode(&shell),
            InputMode::Poj,
            "a refused snapshot keeps the last"
        );

        shell.serve(version(), Some(&snapshot(vec![]))).unwrap();
        assert_eq!(
            input_mode(&shell),
            InputMode::DEFAULT,
            "a removed key reads as its default"
        );
    }

    /// The key rules (P14) are answered before Configure, under the snapshot
    /// they carry, and leave the one the key path reads as it was — whether
    /// they answer or refuse.
    #[test]
    fn the_key_rules_never_put_their_snapshot_in_force() {
        use crate::proto::{ComposingShortcutsRequest, SymbolPickerKeyRequest};
        use crate::test_support::key_event;
        let shell = Shell::default();
        shell
            .serve(version(), Some(&snapshot(vec![text("inputMode", "poj")])))
            .unwrap();
        let tl = snapshot(vec![
            text("inputMode", "tl"),
            text("toneInputScheme", "telex"),
        ]);
        let Ok(desktop_response::Reply::ComposingShortcuts(reply)) = shell.serve(
            desktop_request::Request::ComposingShortcuts(ComposingShortcutsRequest {}),
            Some(&tl),
        ) else {
            panic!("the pane's rows, before Configure");
        };
        assert_eq!(
            reply.slot_keys, "123456789",
            "read under the snapshot it carried"
        );
        let refused = snapshot(vec![text("inputMode", "tl"), text("notASetting", "")]);
        assert!(shell
            .serve(
                desktop_request::Request::SymbolPickerKey(SymbolPickerKeyRequest {
                    event: Some(key_event("q", 0, None)),
                }),
                Some(&refused),
            )
            .is_err());
        assert_eq!(input_mode(&shell), InputMode::Poj, "the key path's stays");
    }

    /// A refused snapshot stops the request before it runs: Configure with
    /// one leaves the runtime unbuilt.
    #[test]
    fn a_refused_snapshot_refuses_the_request() {
        let shell = Shell::default();
        let bad = snapshot(vec![text("autoSpaceEnabled", "yes")]);
        let configure = desktop_request::Request::Configure(configure_request(None, None));
        assert!(matches!(
            shell.serve(configure, Some(&bad)),
            Err(Refusal::Settings(_))
        ));
        assert!(shell.runtime.get().is_none());
    }

    /// A request refused for any reason puts the previous snapshot back —
    /// a second Configure "changes nothing", a session request before
    /// Configure too.
    #[test]
    fn a_refused_request_keeps_the_previous_snapshot() {
        let shell = Shell::default();
        let poj = snapshot(vec![text("inputMode", "poj")]);
        let session = desktop_request::Request::Release(ReleaseRequest { token: 1 });
        assert_eq!(
            shell.serve(session, Some(&poj)),
            Err(Refusal::NotConfigured)
        );
        assert_eq!(input_mode(&shell), InputMode::DEFAULT);

        let configure = || desktop_request::Request::Configure(configure_request(None, None));
        shell.serve(configure(), None).unwrap();
        assert_eq!(
            shell.serve(configure(), Some(&poj)),
            Err(Refusal::AlreadyConfigured)
        );
        assert_eq!(input_mode(&shell), InputMode::DEFAULT);
    }

    /// Poison recovery after a panic (the seam's FAIL_INTERNAL encoding of
    /// it is `lib.rs` `panic_answers_fail_internal`): a panic while a
    /// request holds the session and the coordinator poisons both; the
    /// next requests are served all the same.
    #[test]
    fn a_panic_mid_request_does_not_stop_the_session() {
        use crate::proto::{
            desktop_response, ActivateRequest, CancelRequest, KeyRequest, PanelState,
        };
        use crate::test_support::{engine_shell, key_event, next_token};

        let (_engine, shell) = engine_shell();
        let runtime = shell.runtime().unwrap();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = shell.lock_session();
            let _coordinator = runtime.lock_coordinator();
            panic!("injected panic mid-request");
        }));
        assert!(panicked.is_err());
        assert!(shell.session.is_poisoned());

        let token = next_token();
        let session = |request| match shell.serve(request, None) {
            Ok(desktop_response::Reply::Session(reply)) => reply,
            other => panic!("expected a session reply, got {other:?}"),
        };
        assert!(
            !session(desktop_request::Request::Activate(ActivateRequest {
                token
            }))
            .ignored
        );
        let cancel = session(desktop_request::Request::Cancel(CancelRequest {
            token,
            panel: Some(PanelState::default()),
        }));
        assert!(!cancel.ignored);
        let key = session(desktop_request::Request::Key(KeyRequest {
            token,
            event: Some(key_event("t", 0, None)),
            panel: Some(PanelState::default()),
        }));
        assert!(key.handled && key.is_composing);
    }
}
