import SwiftUI

/// Edits one key's long-press list in the layout editor's draft: delete, drag to reorder, add by
/// typing, or go back to the built-in list. Nothing is stored until the editor's Save.
struct LayoutCalloutsSheet: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss
    @Binding var draft: CustomLayout
    /// The half-width label of the key being edited.
    let label: String
    @State private var newEntry = ""

    private var entries: [String] {
        draft.callouts(for: label)
    }

    private var trimmedEntry: String {
        newEntry.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var canAdd: Bool {
        !trimmedEntry.isEmpty && entries.count < CustomLayout.maxCallouts && !entries.contains(trimmedEntry)
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(entries, id: \.self) { entry in
                        Text(verbatim: entry)
                            .font(AppStyle.bodyFont)
                    }
                    .onDelete { offsets in edit { $0.remove(atOffsets: offsets) } }
                    .onMove { source, destination in edit { $0.move(fromOffsets: source, toOffset: destination) } }

                    HStack {
                        TextField(lang.string(.layoutCalloutsAddPlaceholder), text: $newEntry)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .onSubmit(add)
                        Button(action: add) {
                            Image(latinSystemName: "plus.circle.fill")
                        }
                        .disabled(!canAdd)
                        .accessibilityLabel(lang.string(.layoutCalloutsAddPlaceholder))
                    }
                } header: {
                    Text(verbatim: label)
                        .font(.title2)
                        .foregroundStyle(Color.primary)
                        .textCase(nil)
                }

                Section {
                    Button(lang.string(.themeEditorResetAll), role: .destructive) {
                        draft.resetCallouts(for: label)
                    }
                    .disabled(draft.callouts[label] == nil)
                }
            }
            // Delete and reorder controls always show; the list is short and editing is its only use.
            .environment(\.editMode, .constant(.active))
            .navigationTitle(lang.string(.layoutCalloutsTitle))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button(lang.string(.commonClose)) { dismiss() }
                }
            }
        }
    }

    private func add() {
        guard canAdd else { return }
        let entry = trimmedEntry
        edit { $0.append(entry) }
        newEntry = ""
    }

    /// Applies `change` to the shown list and stores the result in the draft.
    private func edit(_ change: (inout [String]) -> Void) {
        var list = entries
        change(&list)
        draft.setCallouts(list, for: label)
    }
}
