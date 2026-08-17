// The databases this input method keeps the user's own data in.

import Foundation

/// Groups the user-data stores so they are constructed against one directory
/// and opened together.
///
/// Separate files rather than tables in one database, matching iOS and
/// Android: they have different capacity policies and very different write
/// rates, and a user who wants to delete one kind of data should not have to
/// lose the others with it.
///
/// The custom dictionary sits here despite not being learned — it shares the
/// directory, the open-at-launch moment and the serial-queue execution model,
/// and naming it anywhere else would give the composition root two places to
/// look for the user's data.
final class UserDataStores: Sendable {
    let frequency: UserFrequencyStore
    let association: UserAssociationStore
    let customDictionary: CustomDictionaryStore

    init(directory: @escaping @Sendable () throws -> URL) {
        frequency = UserFrequencyStore(directory: directory)
        association = UserAssociationStore(directory: directory)
        customDictionary = CustomDictionaryStore(directory: directory)
    }

    /// Assembles a set from stores that already exist. For tests that need one
    /// of the three to behave differently — a store that was never opened, so
    /// its half of a restore fails while the others succeed.
    init(
        frequency: UserFrequencyStore,
        association: UserAssociationStore,
        customDictionary: CustomDictionaryStore,
    ) {
        self.frequency = frequency
        self.association = association
        self.customDictionary = customDictionary
    }

    /// Opens all of them, off the calling thread. Safe to call more than once —
    /// each store opens its file exactly once.
    func open() {
        frequency.open()
        association.open()
        customDictionary.open()
    }
}
