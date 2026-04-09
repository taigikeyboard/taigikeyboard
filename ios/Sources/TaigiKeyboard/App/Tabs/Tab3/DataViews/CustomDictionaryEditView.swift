import SwiftUI

/// Add/Edit form for a custom dictionary entry
struct CustomDictionaryEditView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @Environment(\.dismiss) private var dismiss

    @State private var roman: String
    @State private var hanzi: String

    private let existingEntry: CustomDictionaryEntry?
    private let onSave: (CustomDictionaryEntry) -> Void

    private var isEditing: Bool { existingEntry != nil }

    private var canSave: Bool {
        !roman.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
        !hanzi.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// Create mode
    init(onSave: @escaping (CustomDictionaryEntry) -> Void) {
        self.existingEntry = nil
        self.onSave = onSave
        _roman = State(initialValue: "")
        _hanzi = State(initialValue: "")
    }

    /// Edit mode
    init(entry: CustomDictionaryEntry, onSave: @escaping (CustomDictionaryEntry) -> Void) {
        self.existingEntry = entry
        self.onSave = onSave
        _roman = State(initialValue: entry.roman)
        _hanzi = State(initialValue: entry.hanzi)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField(
                        languageManager.text(Tab3Texts.romanPlaceholder),
                        text: $roman
                    )
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                } header: {
                    Text(languageManager.text(Tab3Texts.romanLabel))
                }

                Section {
                    TextField(
                        languageManager.text(Tab3Texts.hanziPlaceholder),
                        text: $hanzi
                    )
                } header: {
                    Text(languageManager.text(Tab3Texts.hanziLabel))
                }
            }
            .navigationTitle(
                languageManager.text(isEditing ? Tab3Texts.editEntry : Tab3Texts.addEntry)
            )
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(languageManager.text(Tab3Texts.cancel)) {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(languageManager.text(Tab3Texts.save)) {
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

        let entry: CustomDictionaryEntry
        if let existing = existingEntry {
            entry = CustomDictionaryEntry(
                id: existing.id,
                roman: trimmedRoman,
                hanzi: trimmedHanzi,
                createdAt: existing.createdAt,
                updatedAt: Date()
            )
        } else {
            entry = CustomDictionaryEntry(
                roman: trimmedRoman,
                hanzi: trimmedHanzi
            )
        }

        onSave(entry)
        dismiss()
    }
}
