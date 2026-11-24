// One entry in the user's own dictionary.

import Foundation

/// A word the user added themselves: the romanization they typed and the Hanji
/// it stands for.
///
/// The romanization is stored exactly as typed, in whichever script the user
/// was in — TL or POJ display form. Nothing here folds it: the engine
/// canonicalizes per mode when it matches the entry against the lattice, and
/// folds it to canonical TL itself when it learns from a commit, so a stored
/// form that has been massaged on the way in is a key the engine can no longer
/// reproduce (`engine/protos/proto/composing.proto:439-446`).
///
/// `hanji` may be empty — a romanization-only entry is legitimate.
///
/// The page's view of one engine entry (`CustomDictionaryEntry`,
/// `engine/protos/proto/user_data.proto`); the engine owns the stored shape.
struct CustomDictionaryRow: Equatable, Sendable, Identifiable {
    /// Stable across edits: the engine stores an edit under the same id. A
    /// UUID rather than the `(roman, hanji)` pair because editing either
    /// column must keep the row's identity.
    let id: String
    /// The romanization as saved — what an edit starts from.
    var roman: String
    var hanji: String
    /// `roman` in the input mode the list was read under (the engine's
    /// rendering) — what the table shows; empty for a row no list answered.
    var displayRoman: String

    init(
        id: String = UUID().uuidString,
        roman: String,
        hanji: String,
        displayRoman: String = "",
    ) {
        self.id = id
        self.roman = roman
        self.hanji = hanji
        self.displayRoman = displayRoman
    }
}
