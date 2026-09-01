# desktop v3.6.7

The first Windows release: Taigi Keyboard is now a Text Services Framework input
method for Windows 11 and Windows 10 1809+, on the same Rust engine and the same
dictionaries as macOS. Both desktop apps gain the 候選詞顯示 picker (漢羅並排 /
漢羅合用 / 羅馬字); macOS gets the POJ tone-mark fix for `au` before a coda, and
both take the 台 tile icon.

### macOS

#### Input

- **POJ tone marks on `au` before a coda.** 落 rendered `lau̍h` instead of
  `la̍uh`. Marking the second vowel in a closed syllable is a POJ exception that
  belongs to `oa` / `oe` alone; `au` was taking it too. `ere` / `iri` were also
  marking the leading vowel (`e̍re`, `i̍ri`) instead of the trailing one, which
  is what the MOE manual and canonical taigi-converter do. A full initial ×
  final × tone sweep is now byte-identical to canonical. (#622)

#### Candidates

- **候選詞顯示 — how a candidate cell shows the word.** A picker in 外觀, three
  values. **漢羅並排** is today's cell: 漢字 and 羅馬字 side by side, the swap
  shortcut deciding which leads. **漢羅合用** lists both as candidates — a 漢字
  cell and, next to it, its 羅馬字 cell, no subtitles — so either script is one
  pick away and no swap is needed; Space on either commits the other.
  **羅馬字** shows the romanization alone and commits it: rows that would now
  read the same (食 and 𤆬 are both `tsia̍h`) are collapsed by the engine, so
  the window never offers two identical cells, and next-word predictions
  collapse the same way. Under 合用 and 羅馬字 the swap shortcut is inert and
  the stored swap waits for 並排; 括號標註 still applies under 合用
  (`漢字 (羅馬字)`) and has nothing to bracket under 羅馬字. Space on a
  one-label cell has no other script to commit and does nothing, as it already
  does on a romanization-only candidate. 方音齒盤 is unaffected. Changing the
  picker re-fetches an open candidate bar in place. **⌃⌘H** cycles the three
  from the keyboard — the new mode flashes, the bar re-fetches — and is
  re-recordable in 快捷鍵. (#662, #664)

#### Appearance

- **台 tile app icon.** The Mac app wore the mobile "Tâi" wordmark — its
  `AppIcon.icns` was the iOS asset catalogue re-packed by hand. It now wears the
  台 rounded square the menu bar already uses. iOS and Android keep the
  wordmark. (#643)

### Windows

Taigi Keyboard for Windows is a TSF text service (`TaigiKeyboard.dll`) plus a
settings window (`TaigiKeyboardSettings.exe`), installed machine-wide by an Inno
Setup installer. It runs the same Rust engine, the same dictionaries and the
same user-data schemas as macOS.

#### Input

- **Tâi-lô and POJ**, with the composing display, tone handling, continuous
  input, full-width punctuation and auto-space behaviour of the macOS input
  method — the same engine decides all of it. 方音齒盤 (TPS) is not offered on
  Windows.
- **Candidate window** drawn with Direct2D / DirectWrite: three layouts, the
  slot-key labels, the unfold timer and the scroller macOS has. It follows the
  system high-contrast setting and the Windows accent colour, and hides itself
  when the host application draws candidates itself.
- **候選詞顯示** in 外觀 — 漢羅並排 / 漢羅合用 / 羅馬字, the same three cells as
  macOS (漢字 and 羅馬字 as adjacent one-script cells under 合用; romanization
  alone, same-reading rows collapsed, under 羅馬字; a list with no second
  script is one line tall). The `` ` `` swap is inert outside 並排;
  **Ctrl+Alt+H** cycles the three from the keyboard (the new mode flashes, an
  open list re-fetches in place) and is re-recordable in 快捷鍵. A change made
  in the settings window applies from the next keystroke, like the candidate
  layout. (#662, #664)
- **Shortcuts.** `Ctrl+Alt+S` opens the settings window, `Ctrl+Alt+C` switches
  Tâi-lô / POJ, `` ` `` swaps 漢字 / 羅馬字 — the macOS roster with ⌘ read as
  Ctrl and ⌃ as Alt. Every shortcut is re-recordable in 快捷鍵.
- **Learning.** 詞頻, 詞關聯 and 自訂詞庫 are stored per user under
  `%APPDATA%\TaigiKeyboard`, in the same SQLite schemas macOS uses. 自訂詞庫
  imports and exports CSV.

#### Settings window

- WinUI 3 in Windows 11 Settings style: 一般, 外觀, 快捷鍵, 自訂詞庫, 詞庫來源.
  The Windows App Runtime ships beside the executable — nothing to install
  separately.
- 詞庫來源 keeps 教典's eleven 腔口 subcollections open under the master switch,
  greyed rather than cleared while 教典 is off. (#657)
- Interface language follows the Windows display language and can be set in the
  window; the input method's own name is localized in the language bar and the
  input indicator.

#### Updates

- The window checks for a new version on launch when a check is overdue, and a
  per-user scheduled task checks daily. An update is downloaded and staged only
  when the installer is signed by the same certificate as the running copy;
  otherwise the download page is offered.

#### Install

- Administrator, `%ProgramFiles%\TaigiKeyboard`, x64. If a running application
  still holds the previous DLL, the installer says so and asks for a sign-out
  and sign-in — no restart. Uninstalling leaves `%APPDATA%\TaigiKeyboard`
  alone.
- Windows 10 1809 (build 17763) is the floor: the settings window's runtime does
  not start below it. Windows 11 is what the input method is developed and
  tested on.
- x64 only. Windows on Arm is refused by the installer — no Arm64 service is
  built, and an x64 one cannot be loaded by an Arm64-native application, so
  the input method would be silently absent in most of them. 32-bit
  applications have no input method either; everywhere Taiwanese is typically
  typed is 64-bit today.
