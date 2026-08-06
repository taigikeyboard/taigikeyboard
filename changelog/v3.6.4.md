## v3.6.4

Headline release: **full app and keyboard display-language support** — choose Automatic, 漢字, English, 日本語, Tâi-lô, or Pe̍h-ōe-jī, with live switching across the host app, keyboard settings, overlays, content, and accessibility labels. The release also improves VoiceOver / TalkBack state announcements and presents the keyboard to the OS as language-neutral instead of Chinese / Min Nan. Input-engine behaviour and dictionary entries are unchanged from v3.6.3.

### Shared (iOS + Android)

#### New Features

- **Six display-language choices.** A new display-language picker offers Automatic (system), 漢字, English, 日本語, Tâi-lô, and Pe̍h-ōe-jī. Automatic follows English and Japanese device languages and otherwise uses 漢字; the selection is independent of the keyboard input mode and updates the app and keyboard UI live. (#449–#468 / #472–#477 / #492–#496)
- **Localized in-app content.** Feature guides, FAQ content, dictionary-source descriptions, theme names, settings, overlays, alerts, and accessibility labels now follow the selected display language. (#475–#490 / #496)

#### Bug Fixes

- **Accessibility state is announced consistently.** VoiceOver now announces the selected candidate and layout state; Android toggle rows expose one full-row TalkBack control with the correct checked state. (#498 / #499)
- **Cross-platform localization output stays consistent.** Android resource escaping now handles backslashes correctly, and romanization punctuation spacing is normalized across localized copy. (#497 / #502)

#### Changes

- **Keyboard language identity is neutral.** iOS and Android now identify the keyboard to the OS as `mul` (Multiple languages), so it is no longer grouped or labelled as Chinese / Min Nan. This does not change Taigi input, candidates, or dictionary lookup. (#494)
- **Localized copy reviewed and refined.** Hanji, Tâi-lô, and Pe̍h-ōe-jī wording and romanization were reviewed against the project lexicon, including dictionary-source descriptions and in-app feature content. (#502)

#### Refactoring

- Replaced the platform-specific hand-maintained text facades with a shared JSON-source i18n pipeline and generated, typed Swift / Kotlin accessors. (#449–#463)
- Moved feature and FAQ content into the i18n content source and renamed bundled files from `tab1-*` to stable content names. (#475 / #484 / #495)

#### Removed

- Removed the legacy iOS and Android `CommonTexts`, `SettingsTexts`, `LayoutTexts`, `ThemeTexts`, and `DictionaryTexts` facades after all consumers migrated to the generated resolver.
- Removed POJ auto-derivation tooling; production Pe̍h-ōe-jī copy is now hand-authored and reviewed. (#479)

#### New Files

- Shared `i18n/*.json` language sources and `tools/i18n/` validation / generation tooling.
- iOS String Catalog, display-language store, generated string keys, and localized app / keyboard names.
- Android display-language resolver, generated string keys / accessors, and English / Japanese resource sets.

### iOS

#### New Features

- The host app and keyboard extension now resolve display-language changes through the shared App Group setting, including keyboard overlays and VoiceOver labels. (#456–#463 / #496)
- The app and keyboard display names are localized for English and Japanese. (#491)

#### Bug Fixes

- Candidate buttons and expanded candidate cells now expose their selected state to VoiceOver; layout selection announces the active layout. (#498)

#### Changes

- Keyboard-extension `PrimaryLanguage` changed to the neutral `mul` tag. (#494)

### Android

#### New Features

- The host app, in-keyboard settings, candidate / symbol / layout overlays, smartbar, and media buttons now follow the same live display-language selection. (#449–#463 / #483 / #485)
- The app display name is localized for English and Japanese. (#491)

#### Bug Fixes

- Settings switches now use the full row as one TalkBack toggle target and announce their checked state without duplicate accessibility nodes. (#499)

#### Changes

- The IME subtype now uses `languageTag="mul"` and no longer declares the deprecated `imeSubtypeLocale="nan_TW"`. (#494)

### Dictionary

#### Changes

- Regenerated all dictionary artifacts for the release; there are no `(漢字, TL)` entry additions or removals compared with v3.6.3.
