import SwiftUI

/// Add/Edit form for a custom dictionary entry
struct CustomDictionaryEditView: View {
    @Environment(\.dismiss) private var dismiss

    @State private var roman: String
    @State private var hanzi: String

    private let existingEntry: CustomDictionaryEntry?
    private let onSave: (CustomDictionaryEntry) -> Void

    private var isEditing: Bool {
        existingEntry != nil
    }

    private var canSave: Bool {
        !roman.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
            !hanzi.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// Create mode
    init(onSave: @escaping (CustomDictionaryEntry) -> Void) {
        existingEntry = nil
        self.onSave = onSave
        _roman = State(initialValue: "")
        _hanzi = State(initialValue: "")
    }

    /// Edit mode
    init(entry: CustomDictionaryEntry, onSave: @escaping (CustomDictionaryEntry) -> Void) {
        existingEntry = entry
        self.onSave = onSave
        _roman = State(initialValue: entry.roman)
        _hanzi = State(initialValue: entry.hanzi)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField(
                        DictionaryTexts.romanPlaceholder,
                        text: $roman,
                    )
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                } header: {
                    Text(DictionaryTexts.romanLabel)
                }

                Section {
                    TextField(
                        DictionaryTexts.hanziPlaceholder,
                        text: $hanzi,
                    )
                } header: {
                    Text(DictionaryTexts.hanziLabel)
                }
            }
            .navigationTitle(
                isEditing ? DictionaryTexts.editEntry : DictionaryTexts.addEntry,
            )
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(CommonTexts.cancel) {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(DictionaryTexts.save) {
                        saveEntry()
                    }
                    .disabled(!canSave)
                }
            }
        }
    }

    private func saveEntry() {
        let trimmedRoman = roman.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedHanzi = hanzi.trimmingCharacters(in: .whitespacesAndNewlines)

        let entry = if let existing = existingEntry {
            CustomDictionaryEntry(
                id: existing.id,
                roman: trimmedRoman,
                hanzi: trimmedHanzi,
                createdAt: existing.createdAt,
                updatedAt: Date(),
            )
        } else {
            CustomDictionaryEntry(
                roman: trimmedRoman,
                hanzi: trimmedHanzi,
            )
        }

        onSave(entry)
        dismiss()
    }
}
