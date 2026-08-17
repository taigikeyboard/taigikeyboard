// The parts every 詞庫 management page has: a filter box, an import/export
// pair, a destructive clear, and somewhere for a failure to appear.

import SwiftUI

/// What a page is doing right now.
///
/// A page is either idle or busy with one named piece of work; there is no
/// state where two of them run at once, because every action disables the
/// others while it runs. That is deliberate: the stores serialise anyway, and
/// a second import queued behind the first would report its counts against a
/// database the user has already changed.
enum UserDataPageActivity: Equatable, Sendable {
    case idle
    case working(String)

    var isWorking: Bool {
        self != .idle
    }

    var label: String? {
        if case let .working(label) = self {
            label
        } else {
            nil
        }
    }
}

/// Something the page has to tell the user about, as its own type so a page
/// can carry exactly one at a time and SwiftUI can drive an alert from it.
struct UserDataPageMessage: Identifiable, Equatable {
    let id = UUID()
    let title: String
    let detail: String

    static func failure(_ title: String, _ error: some Error) -> UserDataPageMessage {
        UserDataPageMessage(title: title, detail: String(describing: error))
    }

    /// The one refusal a CSV page makes before it has a parser error to report,
    /// shared so the same wrong file does not get two different explanations on
    /// two pages.
    static func notUTF8() -> UserDataPageMessage {
        UserDataPageMessage(title: "匯入失敗", detail: "這个檔案毋是 UTF-8 文字。")
    }
}

extension View {
    /// The alert every page reports through, and the veil that keeps a second
    /// action from starting while one is running.
    func userDataPageChrome(
        activity: UserDataPageActivity,
        message: Binding<UserDataPageMessage?>,
    ) -> some View {
        disabled(activity.isWorking)
            .overlay {
                if let label = activity.label {
                    // Indeterminate on purpose: the stores report what they
                    // did when they are done, not how far along they are, and
                    // a percentage this side invented would be a number the
                    // work does not know.
                    VStack(spacing: 8) {
                        ProgressView()
                        Text(label)
                            .font(.callout)
                    }
                    .padding(24)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                }
            }
            .alert(item: message) { message in
                Alert(
                    title: Text(message.title),
                    message: Text(message.detail),
                    dismissButton: .default(Text("好")),
                )
            }
    }
}

extension View {
    /// Reloads a list once its filter has settled.
    ///
    /// Debounced because every change runs a query, and a fast typist would
    /// otherwise queue one per keystroke behind the first.
    func reloadWhenFilterSettles(
        _ filter: String,
        load: @escaping @Sendable () async -> Void,
    ) -> some View {
        task(id: filter) {
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled else { return }
            await load()
        }
    }
}

/// The filter box the list pages put above their rows.
///
/// In the content area rather than the window toolbar: the toolbar belongs to
/// the settings window's `[一般] [詞庫]` tabs, and a search field placed there
/// would be competing with them for the same strip.
struct UserDataFilterField: View {
    let prompt: String
    @Binding var text: String

    var body: some View {
        TextField(prompt, text: $text)
            .textFieldStyle(.roundedBorder)
    }
}

/// The CSV pair every list page offers, and the clear-everything button.
struct UserDataActionsSection: View {
    let exportTitle: String
    let importTitle: String
    let clearTitle: String
    let clearConfirmation: String
    let onExport: () -> Void
    let onImport: () -> Void
    let onClear: () -> Void

    @State private var isConfirmingClear = false

    var body: some View {
        Section {
            Button(exportTitle, action: onExport)
            Button(importTitle, action: onImport)
            Button(clearTitle, role: .destructive) { isConfirmingClear = true }
                .confirmationDialog(
                    clearConfirmation,
                    isPresented: $isConfirmingClear,
                ) {
                    Button(clearTitle, role: .destructive, action: onClear)
                    Button("取消", role: .cancel) {}
                } message: {
                    Text("這改袂轉來。")
                }
        } footer: {
            Text("這份資料干焦囥佇你的電腦,袂上傳。匯出的檔案內底有你拍過的字,請家己保管好。")
        }
    }
}
