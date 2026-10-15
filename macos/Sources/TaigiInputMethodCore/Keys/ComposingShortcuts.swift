// The Shortcuts pane's composing rows, as desktop-core resolves them.

/// The user's composing key contract as one store holds it, resolved by
/// desktop-core (`ComposingKeyBindings::from_document`, through
/// `KeyRules.composingShortcuts(in:)`), and the pane's fixed rows.
///
/// "Resolved" means the core's three rules have run: every chord passed the
/// gate; no chord is on two fields (the one the user recorded keeps it); an
/// emptied commit row has taken back a free default of its pair.
struct ComposingShortcuts: Sendable {
    /// One composing action's row: two fields, either of which fires it
    /// (`ChordField`).
    struct Row: Sendable, Equatable {
        /// One of the row's two recorder fields.
        struct Field: Sendable, Equatable {
            /// Where its chord is stored (`composingShortcut.nextCandidate`,
            /// `composingShortcut.nextCandidate.second`).
            let settingsKey: String
            /// The resolved chord; nil when the field is empty.
            let chord: ComposingKeyChord?
            /// What a fresh install has on it; nil on the second field.
            let shippedChord: ComposingKeyChord?
            /// What the store holds, parsed; nil when absent, cleared or
            /// unparsable.
            let storedChord: ComposingKeyChord?

            /// Whether this field holds `chord` — on screen, or stored and
            /// hidden behind another field by the resolver, which a recording
            /// must clear too (`ShortcutConflicts::composing_fields_holding`).
            func holds(_ chord: ComposingKeyChord) -> Bool {
                self.chord == chord || storedChord == chord
            }
        }

        /// The action's name (`nextCandidate`).
        let name: String
        let label: StringKey
        /// Its block on the pane: 0 through the candidates, 1 out of the
        /// composition.
        let group: Int
        /// The first field, then the second.
        let fields: [Field]
        /// What a fresh install has on the first field — every row ships one.
        let defaultChord: ComposingKeyChord

        var first: Field {
            fields[0]
        }
    }

    /// Every composing action, in the roster's order.
    let rows: [Row]
    /// The fixed rows, as the pane prints them: the live slot keys bare and
    /// behind ⇧, the navigation keys, the composing caret's chords, the
    /// cancel key.
    let slotKeys: String
    let shiftedSlotKeys: String
    let navigationKeys: String
    let caretChords: String
    let cancelKey: String

    /// No rows and no labels — what the pane draws when the core could not
    /// answer (`KeyRules`, logged there).
    static let unavailable = ComposingShortcuts(
        rows: [],
        slotKeys: "",
        shiftedSlotKeys: "",
        navigationKeys: "",
        caretChords: "",
        cancelKey: "",
    )

    /// The rows of one block, in roster order.
    func rows(inGroup group: Int) -> [Row] {
        rows.filter { $0.group == group }
    }

    /// The fields holding `chord`, other than `excluded` — the pure half of
    /// recording: the pane empties these before it writes, so the field that
    /// loses a chord empties in front of the user. The cross-registry scan
    /// asks without an exclusion: the global tier is nobody's field here.
    func fields(holding chord: ComposingKeyChord, excluding excluded: Row.Field? = nil) -> [Row.Field] {
        rows.flatMap(\.fields).filter { $0.holds(chord) && $0.settingsKey != excluded?.settingsKey }
    }
}
