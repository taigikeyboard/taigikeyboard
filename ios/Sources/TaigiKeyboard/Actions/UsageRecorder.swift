// Where a pick is counted — the engine keeps the count
// (docs/architecture/user-data-engine-roadmap.md P7b).

import Foundation

/// One candidate the user took. This side decides WHAT a pick is — the
/// `(displayText, canonicalTl)` identity it counts under — and the engine
/// keeps the count. `hanji` is set for a continuous pick, so a learned phrase
/// taken whole is touched (§50).
struct Usage {
    let displayText: String
    let canonicalTl: String
    var hanji: String?
}

/// CROSS-PLATFORM INVARIANT — mirrors macOS `UsageRecorder.swift`, Android
/// `UsageRecorder.kt` and the desktop `UsageRecorder`
/// (`desktop/crates/taigi-desktop-core/src/composing/usage.rs`).
protocol UsageRecorder: AnyObject {
    func record(_ usage: Usage)
}

/// The engine's own stores (`UserDataRequest.record_usage`) — when this
/// process opened them; otherwise (a keyboard without Full Access, the test
/// process) a pick has nowhere to go.
final class EngineUsageRecorder: UsageRecorder {
    func record(_ usage: Usage) {
        guard UserDataOpening.isOpen else { return }
        RustEngineBridge.userDataRecordUsage(
            displayText: usage.displayText,
            canonicalTl: usage.canonicalTl,
            hanji: usage.hanji,
        )
    }
}
