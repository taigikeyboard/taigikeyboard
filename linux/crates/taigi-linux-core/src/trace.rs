//! Platform-layer e2e trace events, test builds only (`e2e-trace` feature;
//! docs/architecture/e2e-trace-schema.md). Both shells go through
//! `process_raw_key`, so one hook here traces Fcitx5 and IBus alike.

use std::path::PathBuf;

use dispatch::trace::{event, JsonStr};
use taigi_linux_platform::RawKeyEvent;

use crate::session::{Emit, EngineState, KeyReply};

/// `$XDG_DATA_HOME/taigikeyboard/e2e-trace.jsonl` — the driver gives each
/// run its own `XDG_DATA_HOME`, so the file never mixes runs.
const TRACE_FILE_NAME: &str = "e2e-trace.jsonl";

/// Opens the shared trace file (engine + platform events) once per process.
pub(crate) fn open() {
    let Some(directory) = taigi_linux_platform::paths::data_directory() else {
        return;
    };
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let path: PathBuf = directory.join(TRACE_FILE_NAME);
    if let Some(path) = path.to_str() {
        dispatch::trace::open(path);
    }
}

/// One `key` event, then one event per signal the key sent the daemon.
pub(crate) fn key(raw: RawKeyEvent, reply: &KeyReply, state: &EngineState) {
    if !dispatch::trace::is_open() {
        return;
    }
    event(
        "key",
        format_args!(
            r#""keyval":{},"state":{},"handled":{}"#,
            raw.keyval, raw.state, reply.handled
        ),
    );
    for emit in &reply.emits {
        match emit {
            Emit::Preedit { text, .. } => {
                event("preedit", format_args!(r#""text":{}"#, JsonStr(text)))
            }
            Emit::ClearPreedit => event("preedit", format_args!(r#""text":"""#)),
            Emit::Commit(text) => event("commit", format_args!(r#""text":{}"#, JsonStr(text))),
            Emit::LookupTable(_) if state.symbol_picker.is_none() => candidates(state),
            _ => {}
        }
    }
}

/// The list as shown, display order: each cell's `(hanji, displayed
/// reading)` plus its canonical TL — the identity pair of AGENTS.md #6.
fn candidates(state: &EngineState) {
    let items: Vec<String> = (0..)
        .map_while(|cell| state.candidates.resolve(cell, false))
        .map(|(candidate, _)| {
            format!(
                r#"{{"hanji":{},"tl":{},"canonical_tl":{}}}"#,
                JsonStr(candidate.hanji.as_deref().unwrap_or_default()),
                JsonStr(&candidate.roman),
                JsonStr(&candidate.canonical_tl),
            )
        })
        .collect();
    event(
        "candidates",
        format_args!(r#""items":[{}]"#, items.join(",")),
    );
}

/// The daemon ended the composition itself (focus out, reset, disable).
pub(crate) fn session_end() {
    event("session_end", format_args!(r#""source":"daemon""#));
}

#[cfg(test)]
mod tests {
    use crate::session::{process_raw_key, EngineState};
    use taigi_desktop_core::composing::ContextToken;
    use taigi_linux_platform::RawKeyEvent;

    #[test]
    fn a_key_lands_in_the_trace_as_valid_json_lines() {
        let (directory, runtime) = crate::runtime::temporary_runtime();
        let path = directory.path().join("trace.jsonl");
        assert!(dispatch::trace::open(path.to_str().expect("utf-8 path")));
        let mut state = EngineState::default();
        // trace: keysym 0x74 = `t`, no modifiers.
        process_raw_key(
            &runtime,
            ContextToken(1),
            &mut state,
            RawKeyEvent {
                keyval: 0x74,
                keycode: 0,
                state: 0,
            },
        );

        let text = std::fs::read_to_string(&path).expect("trace file");
        let events: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("every line is JSON"))
            .collect();
        assert_eq!(events[0]["event"], "trace_open");
        assert!(
            events
                .iter()
                .any(|e| e["event"] == "key" && e["keyval"] == 0x74),
            "{text}"
        );
    }
}
