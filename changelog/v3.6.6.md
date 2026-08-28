## v3.6.6

macOS-focused release: candidate selection moves to letter keys, Caps Lock toggles ABC through the system, updates download in-app, and the shifted number row types full-width symbols in 漢字 mode. The POJ `o͘` custom-dictionary fix that landed after the v3.6.5 macOS package was cut is included. Mobile: iOS upgrades KeyboardKit to 10.9.0. Dictionary entries are unchanged from v3.6.5.

### iOS

#### Changes

- KeyboardKit upgraded 10.4.1 → 10.9.0; deprecated symbols renamed to their current names and the callouts namespace owned by the app. The three-layer auto-capitalization workaround stays: the framework's standard setup path did not fix the case on device. (#615)

### Android

#### Changes

- protobuf-javalite runtime 4.35.1 → 4.36.0, paired with the committed generated code. (#615)

### macOS

#### Input

- **Candidate slot keys.** Candidates are picked with one key set chosen in 快捷鍵: the bare keys `q w d f z x v y ;` (default), `⇧1-9`, `⌃1-9`, or `⌥1-9`. Bare digits are always the tone marker while composing, so a digit never means two things. The `↓` selection mode is retired; the window always shows the live set's labels on its cells. (#619)
- **Shifted number row types full-width symbols in 漢字 mode.** `⇧2-8`, `⇧-`, `⇧=` now write ＠＃＄％＾＆＊＿＋, completing the row alongside ！（）. Romanized output keeps Latin punctuation, as before. (#620)
- **Caps Lock switches to and from ABC through the system.** The input source is no longer reported as ASCII-capable, so macOS handles Caps Lock the same way it does for its own CJK input methods. Log out and back in after installing if the menu bar still shows the old behavior. (#617)
- **Custom-dictionary entries with POJ `o͘` are found while typing.** The combining dot above was dropped as if it were a tone mark when building the search key, so `bang-so͘-khó` never matched `bangsoo`. Existing entries are re-keyed on first launch. (#614)

#### Candidate window

- 外觀 layout names read 徛直 / 展開, and the default layout returns to 展開 (expandable). (#618)
- The slot-key picker in 快捷鍵 is labelled 選字齒, matching 快速齒.

#### Updates

- **Updates download in the app.** When a newer version is available, the notification offers to download the package; once it arrives it is handed to Installer.app in a second step, so the download never steals focus from what you are typing. (#616)

### Dictionary

#### Changes

- No `(漢字, TL)` entry additions or removals compared with v3.6.5.
