// The 一般 pane: romanization system, display language, updates, and the attribution footer.

import AppKit
import SwiftUI

/// The 一般 pane of the settings window.
///
/// Bound with `@AppStorage` rather than through `SettingsStore`, so the form
/// re-renders when a value is changed from outside it — the input-source menu's
/// TL/POJ items and the shortcut hotkeys write straight to `UserDefaults`, and
/// so does anyone running `defaults write`. The keys and the fallbacks come
/// from the store's descriptors, so the form and the engine cannot disagree
/// about either.
///
/// The two learning switches are gone, along with the 詞頻紀錄 and 詞關聯紀錄
/// panes that carried them: the records are always on, they are bounded by
/// `LearningCapacity`, and the product decision (USER 2026-08-24) is that they
/// are not the user's to administer. The one destructive verb — clear both at
/// once — lives beside the custom dictionary's own clear button on the 自訂詞庫
/// pane, so every "throw away what is stored" action is in one place.
///
/// 全形標點 (漢羅對調 output) is not a switch either: it is always on (USER
/// 2026-08-24), because CJK output takes CJK punctuation. It never had a state
/// worth administering. Shift 切換英數 was retired the same day for the same
/// reason, and the feature itself went on 2026-08-26: this input method has no
/// English mode, because a Mac already switches input sources with ⌘Space.
///
/// Text comes from the injected `DisplayLanguageStore`: reading it inside `body` is what makes the
/// form re-render when the display language changes, with no window rebuild.
struct GeneralSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    @AppStorage(SettingsStore.Keys.toneInputScheme.name)
    private var toneInputScheme = SettingsStore.Keys.toneInputScheme.defaultValue

    @AppStorage(SettingsStore.Keys.isAutoSpaceEnabled.name)
    private var isAutoSpaceEnabled = SettingsStore.Keys.isAutoSpaceEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isCandidateWindowEnabled.name)
    private var isCandidateWindowEnabled = SettingsStore.Keys.isCandidateWindowEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    private var isLiteralRomanCandidateEnabled = SettingsStore.Keys.isLiteralRomanCandidateEnabled.defaultValue

    /// Whether the system is currently refusing our notices. Re-read when this
    /// app comes back to the front rather than observed: nothing fires when the
    /// setting changes, and changing it means a trip to System Settings and back.
    @State private var areNoticesBlocked = false

    var body: some View {
        settingsForm
            // `didBecomeActive` alone: showing this window activates the app,
            // so it fires as the pane appears and again after every trip out to
            // System Settings. A `.task` on top would only duplicate the first.
            .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
                Task { await refreshNoticeReachability() }
            }
            // Full width, so the sponsor footer below centres on the window.
            .frame(maxWidth: .infinity)
            .safeAreaInset(edge: .bottom, alignment: .center, spacing: 0) {
                sponsorFooter
            }
    }

    private var settingsForm: some View {
        Form {
            Section {
                // A pop-up like the row under it, not a radio group: System
                // Settings states a small mutually-exclusive choice with a
                // pop-up, and two shapes for two adjacent N-of-1 rows read as
                // a difference that means something.
                Picker(language.string(.settingsInputMode), selection: $inputMode) {
                    Text(language.string(.settingsTlMode)).tag(InputMode.tl)
                    Text(language.string(.settingsPojMode)).tag(InputMode.poj)
                }

                // Directly under the romanization it belongs to: which keys
                // type a tone is a fact about how the syllable is spelled,
                // not a shortcut (USER 2026-09-08), and the slot keys follow
                // from it rather than being chosen on the shortcut pane.
                Picker(language.string(.settingsToneInputScheme), selection: $toneInputScheme) {
                    Text(language.string(.settingsToneSchemeStandard)).tag(ToneInputScheme.standard)
                    Text(language.string(.settingsToneSchemeTelex)).tag(ToneInputScheme.telex)
                }
                if toneInputScheme == .telex {
                    // The key table, spelled for the romanization in use:
                    // `z` is `ts` under TL and `ch` under POJ. Shown only
                    // while Telex is on, because Standard's keys are the
                    // ones every TL/POJ user already knows.
                    Text(language.string(inputMode == .poj ? .settingsToneSchemeTelexLegendPoj : .settingsToneSchemeTelexLegendTl))
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Picker(language.string(.settingsDisplayLanguage), selection: displayLanguageSelection) {
                    ForEach(DisplayLanguage.selectableLanguages, id: \.self) { option in
                        // Endonyms for the authored languages, so a user can find their own language
                        // whatever the UI currently reads in; `.system` is the one translated row.
                        Text(language.selectionLabel(for: option)).tag(option)
                    }
                }

                Toggle(language.string(.settingsAutoSpace), isOn: $isAutoSpaceEnabled)

                // S33 (USER 2026-09-08): off means no window at all — the
                // user types romanization and Space / Return write it as
                // typed. Directly above 顯示當咧拍的字, which describes the
                // window's content and so reads as its sub-option; that row
                // stays enabled with the window off (one plain switch, no
                // greyed-out state to explain).
                Toggle(language.string(.settingsCandidateWindow), isOn: $isCandidateWindowEnabled)

                // §34/S22, under 自動空白 where the USER placed it
                // (2026-09-03). On means candidate slot 0 is the preedit
                // literal, so Return writes the typed romanization.
                Toggle(language.string(.settingsLiteralRomanCandidate), isOn: $isLiteralRomanCandidateEnabled)
            }

            // The update rows. No toggle and no explanatory text (USER
            // 2026-08-23): the daily check is always on, and the button is the
            // on-demand version of the same thing.
            //
            // This section is the surface that keeps working when the
            // notification did not: no network needed, nothing to miss, and
            // still right after a banner was dismissed weeks ago.
            Section {
                // One row, never two. A known update replaces the version-and-check
                // row rather than sitting under it: while an update is waiting,
                // checking again is the one thing that cannot tell the user
                // anything they are not already being told, and two rows offering
                // two verbs for one goal is what made the pane read as two paths.
                // macOS Software Update has the same single-row shape.
                //
                // The daily check still runs, so a release that is pulled stops
                // being offered on its own, and the input-source menu's 檢查更新
                // row is still there to force one.
                if let pending = UpdateChecker.shared.pendingUpdate {
                    pendingUpdateRow(for: pending)
                } else {
                    LabeledContent(language.resolver.desktopUpdateCurrentVersionLabel(version: AppVersion.installed)) {
                        Button(language.string(.desktopUpdateCheckNow)) {
                            UpdateChecker.shared.checkManually()
                        }
                    }
                }

                if areNoticesBlocked {
                    LabeledContent(language.string(.desktopUpdateNotificationsOffNote)) {
                        Button(language.string(.desktopUpdateOpenNotificationSettings)) {
                            NotificationManager.shared.openSystemNotificationSettings()
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    /// The row shown while a newer version is known but not installed.
    ///
    /// Its trailing control is whatever the user's next move is, and the second
    /// line appears only when something went wrong. The two-press shape is
    /// deliberate and lives in `UpdateInstallation`: a finished download turns
    /// this button into 安裝 rather than opening Installer.app on its own, so
    /// nothing takes the focus away from a document the user may have gone back
    /// to typing in.
    private func pendingUpdateRow(for pending: UpdateManifest) -> some View {
        let installation = UpdateInstallation.shared
        let offer = installation.offer(for: pending)
        return LabeledContent {
            switch offer {
            case .downloadPage, .packageRejected:
                ExternalLinkButton(
                    titleKey: .desktopUpdateDownloadAction,
                    url: pending.downloadPageURL,
                )
            case .startDownload:
                Button(language.string(.desktopUpdateDownloadAndInstallAction)) {
                    installation.startDownload(for: pending)
                }
            case .downloading:
                ProgressView().controlSize(.small)
            case .install, .installerOpenFailed:
                Button(language.string(.desktopUpdateInstallAction)) {
                    installation.install()
                }
            case .downloadFailed:
                Button(language.string(.desktopUpdateRetryAction)) {
                    installation.startDownload(for: pending)
                }
            }
        } label: {
            Text(language.resolver.desktopUpdatePendingVersionLabel(version: pending.version))
            // The note travels with the state rather than being chosen beside
            // it, so a control and an explanation cannot be paired wrongly.
            if let noteKey = offer.noteKey {
                Text(language.string(noteKey))
            }
        }
    }

    /// The same question posting asks, so the pane cannot stay quiet in a state
    /// where every notice silently fails. `.notAsked` is not blocked — the
    /// offer covers that case on its own.
    private func refreshNoticeReachability() async {
        areNoticesBlocked = await NotificationManager.shared.reachability() == .blocked
    }

    /// Centred at the foot of the pane rather than inside the form: it is neither a setting nor a
    /// note about one. Carried as a bottom safe-area inset, so it holds the window's lower edge
    /// whatever the form above does.
    ///
    /// Type and colour are set here, so the three pieces stay one line of fine print wherever this
    /// footer sits. Matches the project site's own footer: small, grey, the link no louder than the
    /// text around it.
    private var sponsorFooter: some View {
        HStack(spacing: Metrics.footerSpacing) {
            Text(language.string(.desktopCopyrightLine))
            // Punctuation between two labels, with nothing to say on its own.
            Text(verbatim: "\u{00B7}")
                .accessibilityHidden(true)
            ExternalLinkButton(titleKey: .desktopSponsorLink, url: Self.sponsorURL, style: .footer)
        }
        .font(.footnote)
        .foregroundStyle(.secondary)
        .padding(.bottom, Metrics.footerBottomInset)
    }

    private static let sponsorURL = URL(string: "https://p.ecpay.com.tw/AA663DE")

    private enum Metrics {
        /// Tighter than the stack default so the footer reads as one phrase rather than three controls.
        static let footerSpacing: CGFloat = 4

        /// How far the footer sits off the window's bottom edge. A design value: no public API
        /// exposes the grouped form's own margin to match against.
        static let footerBottomInset: CGFloat = 20
    }

    /// The picker's selection, read and written through the store rather than through `@AppStorage`
    /// like the settings around it. The store is what the rest of this view resolves its text from,
    /// so binding past it would let an injected store and the picker disagree — and `setLanguage`
    /// updates the live state synchronously, where a defaults write would arrive an actor hop later.
    private var displayLanguageSelection: Binding<DisplayLanguage> {
        Binding(
            get: { language.selected },
            set: { language.setLanguage($0) },
        )
    }
}
