// The General pane: romanization system, output script, display language, and updates.

import AppKit
import SwiftUI

/// The General pane of the settings window.
///
/// Bound with `@AppStorage` rather than through `SettingsStore`, so the form
/// re-renders when a value is changed from outside it — the input-source menu's
/// TL/POJ items and the shortcut hotkeys write straight to `UserDefaults`, and
/// so does anyone running `defaults write`. The keys and the fallbacks come
/// from the store's descriptors, so the form and the engine cannot disagree
/// about either.
///
/// The two learning switches are gone, along with the Frequency Records and Association Records
/// panes that carried them: the records are always on, they are bounded by
/// the engine's store capacities, and the product decision (USER 2026-08-24) is that they
/// are not the user's to administer. The one destructive verb — clear both at
/// once — lives beside the custom dictionary's own clear button on the Custom Dictionary
/// pane, so every "throw away what is stored" action is in one place.
///
/// Full-width Punctuation (Hanji/romanization swap output) is not a switch either: it is always on (USER
/// 2026-08-24), because CJK output takes CJK punctuation. It never had a state
/// worth administering. Shift-to-English was retired the same day for the same
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

    /// The STORED swap — the same key the `` ` `` shortcut flips, so the
    /// picker and the chord are one setting seen from two places. Read raw
    /// on purpose: the picker shows what is stored, and `candidateDisplayMode`
    /// below decides whether that has any effect right now.
    @AppStorage(SettingsStore.Keys.isTranslateSwapped.name)
    private var isTranslateSwapped = SettingsStore.Keys.isTranslateSwapped.defaultValue

    @AppStorage(SettingsStore.Keys.candidateDisplayMode.name)
    private var candidateDisplayMode = SettingsStore.Keys.candidateDisplayMode.defaultValue

    @AppStorage(SettingsStore.Keys.isAutoSpaceEnabled.name)
    private var isAutoSpaceEnabled = SettingsStore.Keys.isAutoSpaceEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isCandidateWindowEnabled.name)
    private var isCandidateWindowEnabled = SettingsStore.Keys.isCandidateWindowEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    private var isLiteralRomanCandidateEnabled = SettingsStore.Keys.isLiteralRomanCandidateEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isHyphenlessRomanEnabled.name)
    private var isHyphenlessRomanEnabled = SettingsStore.Keys.isHyphenlessRomanEnabled.defaultValue
    @AppStorage(SettingsStore.Keys.isNasalMarkerUppercaseEnabled.name)
    private var isNasalMarkerUppercaseEnabled = SettingsStore.Keys.isNasalMarkerUppercaseEnabled.defaultValue

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
    }

    private var settingsForm: some View {
        Form {
            // One group, no sub-groups (USER 2026-09-18: "no grouping"), in the
            // order the typing pipeline runs (USER 2026-09-21): what is typed
            // and how its tones are spelled, then the candidate window and
            // its content, then what a commit writes and its shape, then the
            // app's language.
            Section {
                // A pop-up like the Output Script row below, not a radio group: System
                // Settings states a small mutually-exclusive choice with a
                // pop-up, and two shapes for two adjacent N-of-1 rows read as
                // a difference that means something. Input Script / Output Script name
                // the pair (USER 2026-09-18); mobile keeps Input Mode, whose
                // picker also holds TPS.
                Picker(language.string(.settingsInputScript), selection: $inputMode) {
                    Text(language.string(.settingsTlMode)).tag(InputMode.tl)
                    Text(language.string(.settingsPojMode)).tag(InputMode.poj)
                }

                // Which keys type a tone is a fact about how the syllable is
                // spelled, not a shortcut (USER 2026-09-08), and the slot
                // keys follow from it rather than being chosen on the
                // shortcut pane. Under Input Script because both say what the
                // user types.
                Picker(language.string(.settingsToneInputScheme), selection: $toneInputScheme) {
                    Text(language.string(.settingsToneSchemeStandard)).tag(ToneInputScheme.standard)
                    Text(language.string(.settingsToneSchemeTelex)).tag(ToneInputScheme.telex)
                }

                // S33 (USER 2026-09-08): off means no window at all — the
                // user types romanization and Space / Return write it as
                // typed. Directly above Show Typed Text First, which describes the
                // window's content and so reads as its sub-option; that row
                // stays enabled with the window off (one plain switch, no
                // greyed-out state to explain).
                Toggle(language.string(.settingsCandidateWindow), isOn: $isCandidateWindowEnabled)

                // §34/S22. On means candidate slot 0 is the preedit literal,
                // so Return writes the typed romanization.
                Toggle(language.string(.settingsLiteralRomanCandidate), isOn: $isLiteralRomanCandidateEnabled)

                // Which script a commit writes (USER 2026-09-18): the same
                // stored swap the `` ` `` shortcut toggles, so the two never
                // disagree. Disabled exactly where the shortcut is inert —
                // Candidate Display = Romanization Only shows no Hanji to lead with
                // (`allowsSwapToggle`); under Hanji with Romanization both scripts are on
                // screen and this only picks the punctuation width, as the
                // shortcut does there.
                Picker(language.string(.settingsOutputScript), selection: $isTranslateSwapped) {
                    Text(language.string(.settingsOutputScriptHanji)).tag(true)
                    Text(language.string(.settingsOutputScriptRoman)).tag(false)
                }
                .disabled(!candidateDisplayMode.allowsSwapToggle)

                // No Hyphens (§49), directly under Output Script — it describes that
                // output's shape.
                Toggle(language.string(.settingsHyphenlessRoman), isOn: $isHyphenlessRomanEnabled)

                // ⁿ becomes ᴺ in capitals (§53): the other switch that shapes the output's
                // romanization.
                Toggle(language.string(.settingsNasalMarkerUppercase), isOn: $isNasalMarkerUppercaseEnabled)

                // The space after a commit: the last thing the output stage
                // does.
                Toggle(language.string(.settingsAutoSpace), isOn: $isAutoSpaceEnabled)

                Picker(language.string(.settingsDisplayLanguage), selection: displayLanguageSelection) {
                    ForEach(DisplayLanguage.selectableLanguages, id: \.self) { option in
                        // Endonyms for the authored languages, so a user can find their own language
                        // whatever the UI currently reads in; `.system` is the one translated row.
                        Text(language.selectionLabel(for: option)).tag(option)
                    }
                }
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
                // being offered on its own, and the input-source menu's Check for Updates
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

            // Its own section, at the end, drawn the way the Appearance and Shortcuts
            // panes draw theirs: it acts on every setting above it — not on
            // the update row, which holds no setting of the user's to restore.
            Section {
                WideActionRow(titleKey: .themeEditorResetAll, action: restoreDefaults)
            }
        }
        .formStyle(.grouped)
    }

    /// Puts the pane's input settings back to what a fresh install renders
    /// with; the `@AppStorage` bindings repaint on their own. The display
    /// language is not touched, so the pane stays in the language it was in.
    private func restoreDefaults() {
        SettingsStore().resetGeneralSettings()
    }

    /// The row shown while a newer version is known but not installed.
    ///
    /// Its trailing control is whatever the user's next move is, and the second
    /// line appears only when something went wrong. The two-press shape is
    /// deliberate and lives in `UpdateInstallation`: a finished download turns
    /// this button into Install rather than opening Installer.app on its own, so
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
