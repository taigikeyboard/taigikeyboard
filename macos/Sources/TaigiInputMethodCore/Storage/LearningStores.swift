// The two databases this input method learns into.

import Foundation

/// Pairs the frequency and association stores so they are constructed against
/// one directory and opened together.
///
/// They are separate files rather than two tables in one, matching iOS and
/// Android: the two have different capacity policies and very different write
/// rates, and a user who wants to delete one kind of learned data should not
/// have to lose the other with it.
final class LearningStores: Sendable {
    let frequency: UserFrequencyStore
    let association: UserAssociationStore

    init(directory: @escaping @Sendable () throws -> URL) {
        frequency = UserFrequencyStore(directory: directory)
        association = UserAssociationStore(directory: directory)
    }

    /// Opens both, off the calling thread. Safe to call more than once — each
    /// store opens its file exactly once.
    func open() {
        frequency.open()
        association.open()
    }
}
