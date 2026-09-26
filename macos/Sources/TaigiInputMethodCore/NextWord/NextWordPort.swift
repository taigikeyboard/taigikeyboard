// Where the manager reports commits for next-word learning.

import Foundation

/// Where the next-word handshakes go: the engine in production
/// (`EngineNextWord`); a recorder in the manager tests, which check what is
/// reported and when.
///
/// The engine decides WHAT is worth learning — whether the 10-second window is
/// still open, how a compound word splits, whether the text is noise at all
/// (`engine/nextword/src/decide.rs`) — and records it itself
/// (`docs/architecture/user-data-engine-roadmap.md` P9b); the manager only
/// reports the commits. Splitting or filtering on this side would be a second
/// copy of a decision that has to match iOS and Android, and that decision is
/// deliberately one rule rather than three:
/// `INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`
/// (`docs/architecture/behavioral-invariants.md` §40) makes the learning rules
/// depend on the writing system, never on the platform. CROSS-PLATFORM
/// INVARIANT — mirrors the desktop `NextWordPort`
/// (`desktop/crates/taigi-desktop-core/src/composing/next_word.rs`).
@MainActor
protocol NextWordPort: AnyObject {
    /// The user finalized `text` into the document.
    func wordSelected(text: String, roman: String, settings: EngineSettings, generation: UInt64)
    /// A continuous composition nailed a segment without finalizing.
    func segmentNailed(text: String, roman: String, settings: EngineSettings, generation: UInt64)
    /// The context is gone: nothing that follows follows it.
    func forgetContext(settings: EngineSettings, generation: UInt64)
}

/// The engine's next-word slice, stamped with the wall clock the engine's
/// association window compares against.
final class EngineNextWord: NextWordPort {
    func wordSelected(text: String, roman: String, settings: EngineSettings, generation: UInt64) {
        RustEngineBridge.nextwordWordSelected(
            text: text,
            roman: roman,
            nowMs: Self.nowMs(),
            settings: settings,
            generation: generation,
        )
    }

    func segmentNailed(text: String, roman: String, settings: EngineSettings, generation: UInt64) {
        RustEngineBridge.nextwordUpdateLastSelectedWord(
            text: text,
            roman: roman,
            nowMs: Self.nowMs(),
            settings: settings,
            generation: generation,
        )
    }

    func forgetContext(settings: EngineSettings, generation: UInt64) {
        RustEngineBridge.nextwordResetFull(nowMs: Self.nowMs(), settings: settings, generation: generation)
    }

    private static func nowMs() -> Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
