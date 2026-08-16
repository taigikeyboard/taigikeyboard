// Turns commit handshakes into learned bigrams.

import Foundation

/// Sits between the composing path and `user_association.db`.
///
/// The engine decides WHAT is worth learning — whether the 10-second window is
/// still open, how a compound word splits, whether the text is noise at all
/// (`engine/nextword/src/decide.rs`) — and this type only carries the answer to
/// the store. Splitting or filtering here would be a second copy of a decision
/// that has to match iOS and Android, which is exactly what
/// `PLATFORM_MACOS` exists to express on the engine side instead.
@MainActor
final class NextWordLearner {
    private let store: UserAssociationStore
    private let now: @Sendable () -> Int64
    private static let logger = DebugLogger(category: "NextWordLearner")

    /// - Parameter now: milliseconds since the epoch. Injectable because the
    ///   engine's association window is a comparison against this clock, and a
    ///   test that cannot move it can only ever exercise one side of it.
    init(
        store: UserAssociationStore,
        now: @escaping @Sendable () -> Int64 = { Int64(Date().timeIntervalSince1970 * 1000) },
    ) {
        self.store = store
        self.now = now
    }

    /// The user finalized `text` into the document.
    func wordSelected(text: String, roman: String, settings: EngineSettings, generation: UInt64) {
        apply(RustEngineBridge.nextwordWordSelected(
            text: text,
            roman: roman,
            nowMs: now(),
            settings: settings,
            generation: generation,
        ))
    }

    /// A continuous composition nailed a segment without finalizing.
    func segmentNailed(text: String, roman: String, settings: EngineSettings, generation: UInt64) {
        apply(RustEngineBridge.nextwordUpdateLastSelectedWord(
            text: text,
            roman: roman,
            nowMs: now(),
            settings: settings,
            generation: generation,
        ))
    }

    /// Drops the current context so nothing that follows is learned as having
    /// followed it.
    func forgetContext(settings: EngineSettings, generation: UInt64) {
        apply(RustEngineBridge.nextwordResetFull(
            nowMs: now(),
            settings: settings,
            generation: generation,
        ))
    }

    /// `nil` is a round-trip that never reached the engine: nothing was
    /// decided, so there is nothing to write.
    private func apply(_ outcome: RustEngineBridge.NextWordOutcome?) {
        guard let outcome else { return }
        for effect in outcome.effects {
            // Counts only, never the words. A bigram is two words of the user's
            // running text plus the order they typed them in, which is the
            // aggregate input `.claude/rules/security-rules.md` forbids logging
            // even in a debug build — the release no-op is not what makes that
            // rule satisfied.
            switch effect {
            case let .recordAssociation(pair):
                Self.logger.debug("recordAssociation")
                store.record([pair])
            case let .recordCompoundAssociations(pairs):
                Self.logger.debug("recordCompoundAssociations count=\(pairs.count)")
                store.record(pairs)
            }
        }
    }
}
