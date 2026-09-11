// The parts the 詞庫 page is built from: a filter box, an import/export pair,
// a destructive clear, and somewhere for a failure to appear.

import SwiftUI

/// What a page is doing right now.
///
/// A page is either idle or busy with one named piece of work; there is no
/// state where two of them run at once, because an action that finds this
/// non-idle refuses to start (`CustomDictionaryPageModel.beginWork`). That is
/// deliberate: the stores serialise anyway, but a second import queued behind
/// the first would report its counts against a database the user has already
/// changed.
///
/// The refusal lives in the model on purpose. The view greys its controls
/// too, but only after a delay (`UserDataPageChrome`), so for the first
/// fraction of a second the interface is not the thing keeping two actions
/// apart.
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
    /// A receipt with nothing to add: a title, and no body at all. For the one
    /// kind of success whose result the user cannot see — deleting records
    /// that have no surface of their own.
    case done(StringKey)
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
        case let .done(key): language.resolve(key)
        case .notUTF8: language.resolve(.commonImportFailed)
        case .imported: language.resolve(.desktopImportComplete)
        }
    }

    /// Nil when the title says the whole of it — an alert then draws a title
    /// and nothing under it, rather than an empty line where a body would be.
    func detail(_ language: StringResolver) -> String? {
        switch self {
        case let .failure(_, diagnostic): diagnostic
        case .done: nil
        case .notUTF8: language.resolve(.desktopNotUTF8Detail)
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

    /// What the overlay says, once the work has run long enough to be worth
    /// interrupting for — and nil while it has not.
    ///
    /// Adding or deleting one entry is a local SQLite write that finishes in
    /// milliseconds, so anything shown the instant work starts appears and
    /// vanishes as a flash — the spinner, and the greyed controls under it
    /// (USER 2026-08-24, twice). Both wait for this.
    @State private var slowWorkLabel: StringKey?

    /// How long work has to run before the spinner is worth the interruption.
    private static let overlayDelay = Duration.milliseconds(400)

    func body(content: Content) -> some View {
        content
            // The same delayed condition the overlay uses, and for the same
            // reason: disabling a bordered button visibly greys it, so a veil
            // raised and dropped inside two frames reads as every control on
            // the page flashing (USER 2026-08-24). This is appearance only —
            // what actually stops two actions overlapping is the model
            // refusing the second one.
            .disabled(slowWorkLabel != nil)
            // Cancelled and restarted on every change of activity, so
            // finishing inside the delay window leaves the overlay unshown,
            // and going idle puts the flag back.
            .task(id: activity) {
                guard let labelKey = activity.labelKey else {
                    slowWorkLabel = nil
                    return
                }
                try? await Task.sleep(for: Self.overlayDelay)
                guard !Task.isCancelled else { return }
                slowWorkLabel = labelKey
            }
            .overlay {
                if let labelKey = slowWorkLabel {
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
            // The page's ONE alert. A second `.alert` further down the same
            // chain does not stack — SwiftUI keeps one, and the 刪除學習紀錄
            // receipt was the one it dropped (USER 2026-08-26: no message
            // appeared). Every page message goes through here now.
            .alert(item: $message) { message in
                Alert(
                    title: Text(message.title(language.resolver)),
                    message: message.detail(language.resolver).map(Text.init),
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
/// An `NSSearchField` (`SearchField`) rather than a text field: what it does
/// is search, and the magnifier and the clear button are how macOS says so.
struct UserDataFilterField: View {
    @Environment(DisplayLanguageStore.self) private var language

    @Binding var text: String

    var body: some View {
        SearchField(placeholder: language.string(.dictionarySearchPlaceholder), text: $text)
    }
}

/// The 自訂詞庫 page's CSV pair and its delete-everything row. Its one caller
/// since the 詞頻 / 詞關聯 pages were removed, kept a separate view because the
/// page it serves is already long enough without three more rows inline.
///
/// The delete acts on the click, with nothing to confirm (USER 2026-08-25):
/// the step it used to cost is paid on every deliberate use, and exporting to
/// CSV is the escape hatch that makes the entries recoverable.
struct UserDataActionsSection: View {
    @Environment(DisplayLanguageStore.self) private var language

    let exportTitle: StringKey
    let importTitle: StringKey
    let deleteTitle: StringKey
    let onExport: () -> Void
    let onImport: () -> Void
    let onDelete: () -> Void

    var body: some View {
        Section {
            // One row, both verbs, trailing (USER 2026-08-26). They are a pair
            // — the same file, written one way and read the other — and a pair
            // reads as one choice side by side and as two unrelated commands
            // stacked. Trailing is where System Settings puts a row's action
            // button; a `Button` alone in a form row sits leading, in the
            // column the pane's LABELS occupy, and these rows have no label.
            //
            // 匯入 first (USER): the pair ends on the button nearest the
            // trailing edge, and 匯出 is the one that ends in a file panel the
            // user then does something with.
            //
            // The clear stays a full-width centred `WideActionRow`: it is not
            // a control on a row, it is the row (USER 2026-08-24).
            HStack {
                Spacer()
                Button(language.string(importTitle), action: onImport)
                Button(language.string(exportTitle), action: onExport)
            }
            WideActionRow(titleKey: deleteTitle, role: .destructive, action: onDelete)
        }
    }
}

/// Every number a managed list is drawn to — the tables of 自訂詞庫 and 自訂字型
/// alike, so two lists of the same kind are the same size.
///
/// Approximate by nature: AppKit owns a table's real row metrics. The direction
/// of the error is what matters — a height derived from a row count comes up a
/// little short rather than cutting a row off, because the count is the input.
enum UserDataListMetrics {
    static let tableRowHeight: CGFloat = 24
    static let tableHeaderHeight: CGFloat = 28

    /// A table tall enough for `rows` of them.
    static func tableHeight(rows: Int) -> CGFloat {
        tableHeaderHeight + CGFloat(rows) * tableRowHeight
    }

    /// Large enough that an empty list's symbol reads as a state rather than as
    /// a control the user is meant to press.
    static let emptyStateSymbolSize: CGFloat = 34
}

/// The `n / N` readout and the two arrows a paged list puts at the trailing
/// end of its `UserDataListControls` (自訂詞庫, 字型管理).
///
/// A list is paged rather than scrolled because a fixed-height `Table` inside
/// a `Form` is one scroll view inside another, and the inner one does not
/// scroll (USER, real device, 2026-08-26). A page that FITS the table needs no
/// scroller of its own, and every row is reachable by paging.
struct UserDataListPager: View {
    @Environment(DisplayLanguageStore.self) private var language

    /// Zero-based; shown one-based.
    let page: Int
    let pageCount: Int
    let onBackward: () -> Void
    let onForward: () -> Void

    var body: some View {
        // Digits only, so the pager needs no wording in five languages — and
        // the two arrows carry the shortcut pane's own page verbs as their
        // accessibility labels, which are already translated.
        Text(verbatim: "\(page + 1) / \(pageCount)")
            .foregroundStyle(.secondary)
            .monospacedDigit()

        Button(action: onBackward) {
            UserDataListControlGlyph(symbolName: "chevron.left")
        }
        .disabled(page <= 0)
        .accessibilityLabel(language.string(.desktopActionPageBackward))

        Button(action: onForward) {
            UserDataListControlGlyph(symbolName: "chevron.right")
        }
        .disabled(page + 1 >= pageCount)
        .accessibilityLabel(language.string(.desktopActionPageForward))
    }
}

/// The `+` / `−` pair under an editable list, where macOS puts the add and
/// remove verbs for one — plus whatever a list wants at the trailing end (a
/// paged list puts its `UserDataListPager` there; a list that fits needs
/// nothing).
///
/// `−` is disabled with nothing selected rather than hidden, so the pair keeps
/// its shape.
struct UserDataListControls<Trailing: View>: View {
    @Environment(DisplayLanguageStore.self) private var language

    /// What the `+` announces to an assistive reader — the list's own verb,
    /// since "add" alone does not say what is being added.
    let addLabelKey: StringKey
    let isRemoveEnabled: Bool
    let onAdd: () -> Void
    let onRemove: () -> Void
    @ViewBuilder let trailing: () -> Trailing

    var body: some View {
        HStack(spacing: 4) {
            Button(action: onAdd) {
                UserDataListControlGlyph(symbolName: "plus")
            }
            .accessibilityLabel(language.string(addLabelKey))

            Button(action: onRemove) {
                UserDataListControlGlyph(symbolName: "minus")
            }
            .disabled(!isRemoveEnabled)
            .accessibilityLabel(language.string(.commonDelete))

            Spacer()

            trailing()
        }
        // Small bordered buttons, the size AppKit gives the +/- bar under a
        // table. `.borderless` around a bare glyph left a hit target the size
        // of the symbol itself.
        .buttonStyle(.bordered)
        .controlSize(.small)
    }
}

extension UserDataListControls where Trailing == EmptyView {
    init(
        addLabelKey: StringKey,
        isRemoveEnabled: Bool,
        onAdd: @escaping () -> Void,
        onRemove: @escaping () -> Void,
    ) {
        self.init(
            addLabelKey: addLabelKey,
            isRemoveEnabled: isRemoveEnabled,
            onAdd: onAdd,
            onRemove: onRemove,
            trailing: { EmptyView() },
        )
    }
}

/// One button's glyph in a list's control bar, sized so the button is as big as
/// the control it imitates. The frame is what does that — `contentShape` only
/// squares off the hit region inside whatever bounds the label already has, it
/// cannot grow them.
struct UserDataListControlGlyph: View {
    let symbolName: String

    var body: some View {
        Image(systemName: symbolName)
            .frame(width: 20, height: 14)
            .contentShape(Rectangle())
    }
}

/// What an empty list draws: a symbol, not a sentence (USER 2026-08-24). An
/// empty list needs no explaining, and the wording would be a string in five
/// languages saying what the blank table already says.
///
/// The symbol carries a sentence as its accessibility label all the same, so a
/// reader is still told what the blank table means.
struct UserDataListEmptySymbol: View {
    @Environment(DisplayLanguageStore.self) private var language

    let symbolName: String
    let accessibilityLabelKey: StringKey

    var body: some View {
        Image(systemName: symbolName)
            .font(.system(size: UserDataListMetrics.emptyStateSymbolSize))
            .foregroundStyle(.tertiary)
            .accessibilityLabel(language.string(accessibilityLabelKey))
    }
}
