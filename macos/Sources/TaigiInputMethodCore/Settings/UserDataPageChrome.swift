// The parts the 詞庫 pages share: a filter box, an import/export pair, a
// destructive clear, and somewhere for a failure to appear.

import SwiftUI

/// What a page is doing right now.
///
/// A page is either idle or busy with one named piece of work; there is no
/// state where two of them run at once, because every action disables the
/// others while it runs. That is deliberate: the stores serialise anyway, and
/// a second import queued behind the first would report its counts against a
/// database the user has already changed.
///
/// The label is a key rather than a resolved string: a long import can outlive
/// a display-language change, and the overlay has to follow it.
enum UserDataPageActivity: Equatable, Sendable {
    case idle
    case working(StringKey)

    var isWorking: Bool {
        self != .idle
    }

    var labelKey: StringKey? {
        if case let .working(key) = self {
            key
        } else {
            nil
        }
    }
}

/// Something the page has to tell the user about — what happened, never how to
/// say it, so nothing a page holds can be left in a language the user has since
/// changed away from. The wording is resolved when the alert draws; whether
/// AppKit re-renders an alert already on screen is its own decision, and not
/// one this side makes.
///
/// One case per valid title-and-body pairing rather than a free title beside a
/// free body, which would admit a success title over a failure explanation.
enum UserDataPageMessage: Identifiable, Hashable {
    /// `diagnostic` is the store's own error text, which is English and stays
    /// that way: it names a SQLite or file-system condition, not something the
    /// product has wording for.
    case failure(StringKey, diagnostic: String)
    /// The one refusal a CSV page makes before it has a parser error to report,
    /// its own case so the same wrong file does not get two different
    /// explanations on two pages.
    case notUTF8
    case imported(Int, skipped: Int)

    var id: Self {
        self
    }

    static func failure(_ title: StringKey, _ error: some Error) -> UserDataPageMessage {
        .failure(title, diagnostic: String(describing: error))
    }

    func title(_ language: StringResolver) -> String {
        switch self {
        case let .failure(key, _): language.resolve(key)
        case .notUTF8: language.resolve(.commonImportFailed)
        case .imported: language.resolve(.macosImportComplete)
        }
    }

    func detail(_ language: StringResolver) -> String {
        switch self {
        case let .failure(_, diagnostic): diagnostic
        case .notUTF8: language.resolve(.macosNotUTF8Detail)
        case let .imported(imported, skipped):
            language.dictionaryImportResult(imported: imported, skipped: skipped)
        }
    }
}

extension View {
    /// The alert every page reports through, and the veil that keeps a second
    /// action from starting while one is running.
    func userDataPageChrome(
        activity: UserDataPageActivity,
        message: Binding<UserDataPageMessage?>,
    ) -> some View {
        modifier(UserDataPageChrome(activity: activity, message: message))
    }
}

/// A modifier rather than a plain `View` extension so it can read the display
/// language and resolve the overlay and the alert as it draws them.
private struct UserDataPageChrome: ViewModifier {
    @Environment(DisplayLanguageStore.self) private var language

    let activity: UserDataPageActivity
    @Binding var message: UserDataPageMessage?

    func body(content: Content) -> some View {
        content
            .disabled(activity.isWorking)
            .overlay {
                if let labelKey = activity.labelKey {
                    // Indeterminate on purpose: the stores report what they
                    // did when they are done, not how far along they are, and
                    // a percentage this side invented would be a number the
                    // work does not know.
                    VStack(spacing: 8) {
                        ProgressView()
                        Text(language.string(labelKey))
                            .font(.callout)
                    }
                    .padding(24)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                }
            }
            .alert(item: $message) { message in
                Alert(
                    title: Text(message.title(language.resolver)),
                    message: Text(message.detail(language.resolver)),
                    dismissButton: .default(Text(language.string(.commonOk))),
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

/// The box the 自訂詞庫 pane types into, above its rows.
///
/// In the content area rather than the window toolbar: the toolbar belongs to
/// the settings window's `[一般] [詞庫]` tabs, and a search field placed there
/// would be competing with them for the same strip.
struct UserDataFilterField: View {
    @Environment(DisplayLanguageStore.self) private var language

    @Binding var text: String

    var body: some View {
        TextField(language.string(.macosFilterPlaceholder), text: $text)
            .textFieldStyle(.roundedBorder)
    }
}

/// The 自訂詞庫 page's CSV pair and its clear-everything button. Its one caller
/// since the 詞頻 / 詞關聯 pages were removed, kept a separate view because the
/// page it serves is already long enough without three more rows inline.
struct UserDataActionsSection: View {
    @Environment(DisplayLanguageStore.self) private var language

    let exportTitle: StringKey
    let importTitle: StringKey
    let clearTitle: StringKey
    let clearConfirmation: StringKey
    let onExport: () -> Void
    let onImport: () -> Void
    let onClear: () -> Void

    @State private var isConfirmingClear = false

    var body: some View {
        Section {
            Button(language.string(exportTitle), action: onExport)
            Button(language.string(importTitle), action: onImport)
            Button(language.string(clearTitle), role: .destructive) { isConfirmingClear = true }
                .confirmationDialog(
                    language.string(clearConfirmation),
                    isPresented: $isConfirmingClear,
                ) {
                    Button(language.string(clearTitle), role: .destructive, action: onClear)
                    Button(language.string(.commonCancel), role: .cancel) {}
                } message: {
                    Text(language.string(.macosIrreversible))
                }
        }
    }
}
