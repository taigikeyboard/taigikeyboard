// Where a learned bigram would go if the engine had not already kept it.

import Foundation

/// A learned bigram. Both halves carry their canonical TL because a Taiwanese
/// word is the `(Hanji, canonical TL)` pair (`CLAUDE.md` Core Principle #6) —
/// 重/tîng followed by 複 is not the same observation as 重/tāng followed by 複.
struct AssociationPair: Hashable, Sendable {
    let previous: String
    let previousTl: String
    let next: String
    let nextTl: String
}

/// Receives the bigrams a next-word answer still carries.
///
/// Once the user data is open the engine writes the bigrams it decides on
/// into `user_association.db` itself and leaves them out of the answer
/// (`docs/architecture/user-data-engine-roadmap.md` P3c), so the shipped
/// sink never sees one. It stays so the context rules — which commits pair,
/// which punctuation breaks a pair — can be asserted from a test process
/// that never opens the user data. Temporary (roadmap U9): removed with the
/// effect itself in P9. CROSS-PLATFORM INVARIANT — mirrors the desktop
/// learner's `AssociationSink` (`desktop/crates/taigi-desktop-core/src/composing/learner.rs`).
@MainActor
protocol AssociationSink: AnyObject {
    func record(_ pairs: [AssociationPair])
}

/// The shipped sink: the engine keeps the bigrams, so there is nothing to do.
final class EngineOwnedAssociations: AssociationSink {
    func record(_: [AssociationPair]) {}
}
