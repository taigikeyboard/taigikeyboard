// Where a pick is counted.

import Foundation

/// One candidate the engine confirmed it took. This side decides WHAT a pick
/// is — the identity it counts under, whether the user's frequency setting
/// lets it be counted — and the engine keeps the count
/// (`docs/architecture/user-data-engine-roadmap.md` P6).
struct Usage: Equatable {
    /// The key the engine ranks on — never the document rendering.
    let displayText: String
    let canonicalTl: String
    /// Set for a Hanji pick, so a learned phrase taken whole is touched.
    let hanji: String?
    /// The user's frequency-recording setting.
    let isFrequencyRecordingEnabled: Bool
}

/// CROSS-PLATFORM INVARIANT — mirrors the desktop's `UsageRecorder`
/// (`desktop/crates/taigi-desktop-core/src/composing/usage.rs`).
@MainActor
protocol UsageRecorder: AnyObject {
    func record(_ usage: Usage)
}

/// The engine's own stores (`UserDataRequest.record_usage`).
final class EngineUsageRecorder: UsageRecorder {
    func record(_ usage: Usage) {
        RustEngineBridge.userDataRecordUsage(usage)
    }
}
