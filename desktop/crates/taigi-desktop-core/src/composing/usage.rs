//! Where a pick is counted. The desktop decides WHAT a pick is — the
//! identity it counts under, whether the user's frequency setting lets it be
//! counted — and the engine keeps the count (user-data-engine-roadmap P5).

use crate::engine;

/// One candidate the engine confirmed it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    /// The key the engine ranks on — never the document rendering.
    pub display_text: String,
    pub canonical_tl: String,
    /// Set for a Hanji pick, so a learned phrase taken whole is touched.
    pub hanji: Option<String>,
    /// The user's frequency-recording setting.
    pub frequency_recording_enabled: bool,
}

/// `Send + Sync` for the same reason as the manager's other seams: one
/// manager per process behind a mutex several threads may reach.
pub trait UsageRecorder: Send + Sync {
    fn record(&self, usage: &Usage);
}

/// The engine's own stores (`UserDataRequest.record_usage`).
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineUsage;

impl UsageRecorder for EngineUsage {
    fn record(&self, usage: &Usage) {
        engine::user_data::record_usage(
            &usage.display_text,
            &usage.canonical_tl,
            usage.hanji.as_deref(),
            usage.frequency_recording_enabled,
        );
    }
}

/// A process with no data directory (a Windows AppContainer host, a Linux
/// session without HOME): nothing opened, nothing learned, nothing sent.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoUsage;

impl UsageRecorder for NoUsage {
    fn record(&self, _usage: &Usage) {}
}
