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
/// Text comes from the injected `DisplayLanguageStore`: reading it inside `body` is what makes the
/// form re-render when the display language changes, with no window rebuild.
struct GeneralSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    @AppStorage(SettingsStore.Keys.isAutoSpaceEnabled.name)
    private var isAutoSpaceEnabled = SettingsStore.Keys.isAutoSpaceEnabled.defaultValue

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
            // The form stays a bounded column; the pane around it takes the whole detail area, so
            // the footer below centres on the window rather than on the column.
            .frame(maxWidth: SettingsPaneLayout.maximumFormWidth)
            .frame(maxWidth: .infinity)
            .safeAreaInset(edge: .bottom, alignment: .center, spacing: 0) {
                sponsorFooter
            }
    }

    private var settingsForm: some View {
        Form {
            Section {
                Picker(language.string(.macosRomanizationSystem), selection: $inputMode) {
                    Text(language.string(.settingsTlMode)).tag(InputMode.tl)
                    Text(language.string(.settingsPojMode)).tag(InputMode.poj)
                }
                .pickerStyle(.radioGroup)

                Picker(language.string(.settingsDisplayLanguage), selection: displayLanguageSelection) {
                    ForEach(DisplayLanguage.selectableLanguages, id: \.self) { option in
                        // Endonyms for the authored languages, so a user can find their own language
                        // whatever the UI currently reads in; `.system` is the one translated row.
                        Text(language.selectionLabel(for: option)).tag(option)
                    }
                }

                Toggle(language.string(.settingsAutoSpace), isOn: $isAutoSpaceEnabled)
            }

            // The update rows. No toggle and no explanatory text (USER
            // 2026-08-23): the daily check is always on, and the button is the
            // on-demand version of the same thing.
            //
            // This section is the surface that keeps working when the
            // notification did not: no network needed, nothing to miss, and
            // still right after a banner was dismissed weeks ago.
            Section {
                LabeledContent(language.resolver.macosUpdateCurrentVersionLabel(version: AppVersion.installed)) {
                    Button(language.string(.macosUpdateCheckNow)) {
                        UpdateChecker.shared.checkManually()
                    }
                }

                if let pending = UpdateChecker.shared.pendingUpdate {
                    LabeledContent(
                        language.resolver.macosUpdatePendingVersionLabel(version: pending.version),
                    ) {
                        ExternalLinkButton(
                            titleKey: .macosUpdateDownloadAction,
                            url: pending.downloadPageURL,
                        )
                    }
                }

                if areNoticesBlocked {
                    LabeledContent(language.string(.macosUpdateNotificationsOffNote)) {
                        Button(language.string(.macosUpdateOpenNotificationSettings)) {
                            NotificationManager.shared.openSystemNotificationSettings()
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
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
            Text(language.string(.macosCopyrightLine))
            // Punctuation between two labels, with nothing to say on its own.
            Text(verbatim: "\u{00B7}")
                .accessibilityHidden(true)
            ExternalLinkButton(titleKey: .macosSponsorLink, url: Self.sponsorURL, style: .footer)
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
