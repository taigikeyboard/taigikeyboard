// One entry in the user's own dictionary.

import Foundation

/// A word the user added themselves: the romanization they typed and the 漢字
/// it stands for.
///
/// The romanization is stored exactly as typed, in whichever script the user
/// was in — TL or POJ display form. Nothing here folds it: the engine
/// canonicalizes per mode when it matches the entry against the lattice, and
/// folds it to canonical TL itself when it learns from a commit, so a stored
/// form that has been massaged on the way in is a key the engine can no longer
/// reproduce (`engine/protos/proto/composing.proto:439-446`).
///
/// `hanzi` may be empty — a romanization-only entry is legitimate, and reaches
/// the engine as an absent `hanji` rather than an empty one.
///
/// CROSS-PLATFORM INVARIANT — the stored shape mirrors
/// ios/Sources/TaigiKeyboard/Lexicon/Models/CustomDictionaryEntry.swift:11-31
/// and the Android `custom_dictionary` table. Drift means the same `.taigi`
/// backup restores differently per platform.
/// What makes two custom-dictionary entries the same word to an import.
///
/// A struct rather than the two columns joined into a string: no separator is
/// safe to assume absent from either, and `(a<sep>b, c)` colliding with
/// `(a, b<sep>c)` would silently drop a word the user asked for.
struct CustomDictionaryIdentity: Hashable, Sendable {
    let roman: String
    let hanzi: String
}

struct CustomDictionaryRow: Equatable, Sendable, Identifiable {
    /// Stable across edits so the side table of search keys can be replaced
    /// rather than accumulated. A UUID rather than the `(roman, hanzi)` pair
    /// because editing either column must keep the row's identity.
    let id: String
    var roman: String
    var hanzi: String
    let createdAt: Date
    var updatedAt: Date

    /// What an import compares this row against. Not the `id`: a file has no
    /// ids, so "already there" can only mean the same word spelled the same
    /// way.
    var identity: CustomDictionaryIdentity {
        CustomDictionaryIdentity(roman: roman, hanzi: hanzi)
    }

    init(
        id: String = UUID().uuidString,
        roman: String,
        hanzi: String,
        createdAt: Date = Date(),
        updatedAt: Date = Date(),
    ) {
        self.id = id
        self.roman = roman
        self.hanzi = hanzi
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}
