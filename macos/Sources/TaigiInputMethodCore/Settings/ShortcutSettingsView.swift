// The 快捷鍵 pane: the global hotkeys, and what each key does while composing.

import KeyboardShortcuts
import SwiftUI

/// The 快捷鍵 pane of the settings window.
///
/// Two kinds of binding, and they are recorded differently on purpose. A global
/// chord can be anything the user presses, so it gets a recorder. A composing
/// key cannot: Return, Space and the brackets carry no modifier, and a recorder
/// that accepted them would let a user bind away the letters they type with.
/// Those get pickers over choices that are known to leave a usable keyboard
/// behind (`ComposingKeyBindings`).
///
/// `@AppStorage`-bound like the panes beside it, with the keys and the
/// fallbacks taken from the store's descriptors, so the form and the key
/// classifier cannot disagree about either.
struct ShortcutSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.returnKeyBehavior.name)
    private var returnKeyBehavior = SettingsStore.Keys.returnKeyBehavior.defaultValue

    @AppStorage(SettingsStore.Keys.spaceKeyBehavior.name)
    private var spaceKeyBehavior = SettingsStore.Keys.spaceKeyBehavior.defaultValue

    @AppStorage(SettingsStore.Keys.bracketPagingBehavior.name)
    private var bracketPagingBehavior = SettingsStore.Keys.bracketPagingBehavior.defaultValue

    @AppStorage(SettingsStore.Keys.tabCycleBehavior.name)
    private var tabCycleBehavior = SettingsStore.Keys.tabCycleBehavior.defaultValue

    @AppStorage(SettingsStore.Keys.candidateSlotModifier.name)
    private var candidateSlotModifier = SettingsStore.Keys.candidateSlotModifier.defaultValue

    var body: some View {
        Form {
            Section {
                // One row per action, off the same list the hotkey registration
                // uses, so a new action cannot appear in one and not the other.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    KeyboardShortcuts.Recorder(action.label(language), name: action.name) { _ in
                        ShortcutConflicts.resolve(after: action)
                    }
                }
            } header: {
                Text(language.string(.macosShortcutsGlobalSection))
            }

            Section {
                captioned(.macosBindingReturnFooter) {
                    Picker(language.string(.macosBindingReturnKey), selection: $returnKeyBehavior) {
                        Text(language.string(.macosBindingCommitLiteral)).tag(ReturnKeyBehavior.commitLiteral)
                        Text(language.string(.macosBindingConfirmHighlighted)).tag(ReturnKeyBehavior.confirmHighlighted)
                    }
                }

                captioned(.macosBindingSpaceFooter) {
                    Picker(language.string(.macosBindingSpaceKey), selection: $spaceKeyBehavior) {
                        Text(language.string(.macosBindingConfirmHighlighted)).tag(SpaceKeyBehavior.confirmHighlighted)
                        Text(language.string(.macosBindingNextCandidate)).tag(SpaceKeyBehavior.nextCandidate)
                    }
                }

                Picker(language.string(.macosBindingBracketPaging), selection: $bracketPagingBehavior) {
                    Text(language.string(.macosBindingTurnPage)).tag(BracketPagingBehavior.enabled)
                    Text(language.string(.macosBindingTypeTheCharacter)).tag(BracketPagingBehavior.disabled)
                }

                Picker(language.string(.macosBindingTabCycle), selection: $tabCycleBehavior) {
                    Text(language.string(.macosBindingWalkCandidates)).tag(TabCycleBehavior.enabled)
                    Text(language.string(.macosBindingLeaveToApp)).tag(TabCycleBehavior.disabled)
                }

                // Glyphs rather than translated words: a modifier is read off
                // the keyboard, and ⌃ and ⌥ are the same symbols in every
                // language the settings window speaks.
                Picker(language.string(.macosBindingSlotModifier), selection: $candidateSlotModifier) {
                    Text(verbatim: "⌃1 – ⌃9").tag(CandidateSlotModifier.control)
                    Text(verbatim: "⌥1 – ⌥9").tag(CandidateSlotModifier.option)
                }
            } header: {
                Text(language.string(.macosShortcutsComposingSection))
            } footer: {
                Text(language.string(.macosShortcutsFixedKeysNote))
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: SettingsPaneLayout.maximumFormWidth)
    }

    /// A control with its explanation under it, as ONE form row.
    ///
    /// A caption placed beside the control instead becomes a row of its own in a
    /// grouped form — separated from what it explains by a divider, and read by
    /// VoiceOver as an unrelated element. Only a `Section` takes a `footer:`,
    /// and one section per picker would leave five loose boxes; nesting keeps
    /// the five together while tying each caption to its control.
    private func captioned(_ caption: StringKey, @ViewBuilder control: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            control()
            Text(language.string(caption))
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
