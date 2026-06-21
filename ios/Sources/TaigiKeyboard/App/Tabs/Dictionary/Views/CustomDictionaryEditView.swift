// 中文: 自訂詞庫條目的新增 / 編輯表單畫面。
// 中文: 同檔提供 create / edit 兩個 init,onSave callback 由父畫面寫回 ViewModel。

import SwiftUI

/// Add/Edit form for a custom dictionary entry
// 中文: 自訂詞庫單筆編輯畫面。羅馬字 + 漢字兩欄;canSave 需兩欄都非空白。
struct CustomDictionaryEditView: View {
    @Environment(DisplayLanguageStore.self) private var lang
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
    // 中文: 新增模式 — 無既有條目,兩欄初始為空。
    init(onSave: @escaping (CustomDictionaryEntry) -> Void) {
        existingEntry = nil
        self.onSave = onSave
        _roman = State(initialValue: "")
        _hanzi = State(initialValue: "")
    }

    /// Edit mode
    // 中文: 編輯模式 — 帶入既有條目作為 prefill,儲存時保留原 id 與 createdAt。
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
                        lang.string(.dictionaryRomanPlaceholder),
                        text: $roman,
                    )
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                } header: {
                    Text(lang.string(.dictionaryRomanLabel))
                }

                Section {
                    TextField(
                        lang.string(.dictionaryHanziPlaceholder),
                        text: $hanzi,
                    )
                } header: {
                    Text(lang.string(.dictionaryHanziLabel))
                }
            }
            .navigationTitle(
                isEditing ? lang.string(.dictionaryEditEntry) : lang.string(.dictionaryAddEntry),
            )
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(lang.string(.commonCancel)) {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(lang.string(.dictionarySave)) {
                        saveEntry()
                    }
                    .disabled(!canSave)
                }
            }
        }
    }

    // 中文: 儲存按鈕動作 — trim 兩欄、新增/編輯模式各自組裝 entry,呼叫 onSave 後 dismiss。
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
