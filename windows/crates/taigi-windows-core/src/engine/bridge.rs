//! The envelope round-trip and the per-request config snapshot.
//! Port of `RustEngineBridge.swift:97-187`.

// 中文: 信封往返(id 序列、錯誤檢查、失敗紀錄)與每次請求攜帶的 AppConfig。

use std::sync::atomic::{AtomicU32, Ordering};

use prost::Message;
use protos::engine::{request, response, AppConfig, ErrorCode, Platform, Request, Response};

use crate::settings::EngineSettings;

static LAST_REQUEST_ID: AtomicU32 = AtomicU32::new(0);

fn next_request_id() -> u32 {
    LAST_REQUEST_ID
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1)
}

/// Encodes one request, dispatches it, and returns the response payload when
/// the engine reported success. `None` means the round-trip FAILED rather than
/// "the engine had nothing to say" — callers must keep the two apart, because
/// a failed round-trip leaves the engine's state untouched and any snapshot
/// synthesized here would contradict it (`RustEngineBridge.swift:83-141`).
///
/// Shared by every slice: envelope, id sequence, error checks and failure log
/// are identical, only the payload case differs.
// 中文: 單一往返;None = 往返失敗、引擎狀態未動,絕不可當成「未組字」。
pub(super) fn roundtrip(
    payload: request::Payload,
    op: &str,
    generation: u64,
    config: Option<AppConfig>,
) -> Option<response::Payload> {
    let request = Request {
        id: next_request_id(),
        generation,
        payload: Some(payload),
        config_snapshot: config,
        ..Default::default()
    };
    let request_id = request.id;
    log::debug!("[engine->] op={op} id={request_id} generation={generation}");

    let response_bytes = dispatch::process_request(&request.encode_to_vec());
    let response = match Response::decode(response_bytes.as_slice()) {
        Ok(response) => response,
        Err(error) => {
            record_failure(op, &format!("response decode failed: {error}"));
            return None;
        }
    };
    // The call is synchronous, so a mismatched id means the response belongs
    // to some other request — reading its payload would apply another
    // operation's state to this one.
    if response.id != request_id {
        record_failure(
            op,
            &format!(
                "response id {} does not match request {request_id}",
                response.id
            ),
        );
        return None;
    }
    let error = ErrorCode::try_from(response.error).unwrap_or(ErrorCode::FailInternal);
    if error != ErrorCode::Ok {
        record_failure(op, &format!("engine returned {error:?}"));
        return None;
    }
    match response.payload {
        Some(payload) => Some(payload),
        None => {
            record_failure(op, "response carried no payload");
            None
        }
    }
}

/// One place for every bridge failure, so a degraded engine is visible in the
/// log instead of surfacing only as candidates that never appear.
pub(super) fn record_failure(op: &str, message: &str) {
    log::error!("[{op}] {message}");
}

/// The engine holds no settings of its own; every request carries the
/// snapshot it should be rendered under. `platform_id` is set on every
/// request, not only the ones that read it: the next-word engine rejects the
/// unset value outright (`engine/nextword/src/decide.rs:50-51`).
///
/// Both double-tap folds are unconditional here, unlike iOS and Android where
/// they are user settings: their on-screen keyboards have dedicated `o͘` and
/// `ⁿ` keys, a hardware keyboard has not, so switching the fold off would
/// leave both graphemes untypable in POJ (`RustEngineBridge.swift:161-175`).
///
/// `candidate_display_mode` rides on the BASE config: the engine collapses
/// same-roman rows under roman-only in both the candidate fetch and the
/// next-word filter, and the two derived configs below inherit it.
// 中文: 每個請求都帶的 AppConfig;oo/nn 雙擊摺疊在硬體鍵盤上永遠開。
pub(super) fn app_config(settings: &EngineSettings) -> AppConfig {
    AppConfig {
        input_mode: settings.input_mode.wire().to_owned(),
        oo_doubletap_enabled: true,
        nn_doubletap_enabled: true,
        platform_id: Platform::Windows as i32,
        candidate_display_mode: settings.candidate_display_mode.wire() as i32,
        ..Default::default()
    }
}

/// `app_config` plus the two word-boundary-spacing flags the engine consults
/// while rendering a continuous composition's nailed prefix
/// (`docs/engine/continuous-input-ranking.md` §10.2). Applied only at the
/// entry points that render that prefix, matching iOS and macOS.
pub(super) fn continuous_app_config(settings: &EngineSettings) -> AppConfig {
    AppConfig {
        is_translate_swapped: settings.is_translate_swapped,
        output_both_scripts: settings.is_output_both_scripts,
        ..app_config(settings)
    }
}

/// `app_config` plus the two fields the next-word decide table reads: the
/// swap flag (suppresses recording for raw-romanization commits) and the
/// recording switch (`decide.rs:109`, `:130`).
pub(super) fn nextword_config(settings: &EngineSettings) -> AppConfig {
    AppConfig {
        is_translate_swapped: settings.is_translate_swapped,
        is_association_recording_enabled: settings.is_association_recording_enabled,
        ..app_config(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{CandidateDisplayMode, InputMode};
    use protos::engine::CandidateDisplayMode as WireDisplayMode;

    #[test]
    fn app_config_carries_platform_and_unconditional_doubletaps() {
        // trace: RustEngineBridgeAppConfigTests.swift:21 pins `.macos`; here Windows.
        let settings = EngineSettings {
            input_mode: InputMode::Poj,
            ..EngineSettings::default()
        };
        let config = app_config(&settings);
        assert_eq!(config.input_mode, "poj");
        assert_eq!(config.platform_id, Platform::Windows as i32);
        assert!(config.oo_doubletap_enabled && config.nn_doubletap_enabled);
        assert!(
            !config.is_translate_swapped,
            "swap flag stays off on the plain path"
        );
        assert_eq!(
            config.candidate_display_mode,
            WireDisplayMode::SideBySide as i32,
            "the default is spelled out, not left Unspecified"
        );
        assert!(!config.is_roman_only_display());
    }

    #[test]
    fn combined_reaches_the_wire_with_the_derived_swap() {
        // trace: the derivation is pinned in `document.rs`; here only the
        // forwarding — the bridge carries the mode and the swap it was handed,
        // and the engine's only normaliser still reads it as "not roman-only".
        let settings = EngineSettings {
            is_translate_swapped: true,
            candidate_display_mode: CandidateDisplayMode::Combined,
            ..EngineSettings::default()
        };
        let continuous = continuous_app_config(&settings);
        assert_eq!(
            continuous.candidate_display_mode,
            WireDisplayMode::Combined as i32
        );
        assert!(continuous.is_translate_swapped);
        assert!(!continuous.output_both_scripts);
        assert!(!continuous.is_roman_only_display());
        assert!(nextword_config(&settings).is_translate_swapped);
    }

    #[test]
    fn roman_only_reaches_every_config_through_the_base_one() {
        let settings = EngineSettings {
            candidate_display_mode: CandidateDisplayMode::RomanOnly,
            ..EngineSettings::default()
        };
        assert!(app_config(&settings).is_roman_only_display());
        assert!(continuous_app_config(&settings).is_roman_only_display());
        assert!(nextword_config(&settings).is_roman_only_display());
    }

    #[test]
    fn continuous_and_nextword_configs_add_their_own_flags_only() {
        let settings = EngineSettings {
            is_translate_swapped: true,
            is_output_both_scripts: true,
            is_association_recording_enabled: false,
            ..EngineSettings::default()
        };
        let continuous = continuous_app_config(&settings);
        assert!(continuous.is_translate_swapped && continuous.output_both_scripts);
        assert!(!continuous.is_association_recording_enabled);
        let nextword = nextword_config(&settings);
        assert!(nextword.is_translate_swapped && !nextword.is_association_recording_enabled);
        assert!(!nextword.output_both_scripts);
    }

    #[test]
    fn successive_request_ids_differ_and_start_non_zero() {
        let a = next_request_id();
        let b = next_request_id();
        assert_ne!(a, b);
        assert_ne!(a, 0);
    }
}
