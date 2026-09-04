# desktop v3.6.7

The first Windows release: Taigi Keyboard is now a Text Services Framework input
method for Windows 11 and Windows 10 1809+, on the same Rust engine and the same
dictionaries as macOS. Both desktop apps gain the 候選詞顯示 picker (漢羅並排 /
漢羅濫 / 羅馬字), and 顯示當咧拍的字, which puts the romanization being typed at
the head of the candidate list and is on by default. 自動空白 now ships off,
matching iOS and Android. Both learn the `o͘ⁿ` spelling of the POJ nasal final
and take the 台 tile icon; macOS gets the POJ tone-mark fix for `au` before a
coda.

### macOS

#### Input

- **POJ tone marks on `au` before a coda.** 落 rendered `lau̍h` instead of
  `la̍uh`. Marking the second vowel in a closed syllable is a POJ exception that
  belongs to `oa` / `oe` alone; `au` was taking it too. `ere` / `iri` were also
  marking the leading vowel (`e̍re`, `i̍ri`) instead of the trailing one, which
  is what the MOE manual and canonical taigi-converter do. A full initial ×
  final × tone sweep is now byte-identical to canonical. (#622)

- **自動空白 ships off, and follows what a commit actually wrote.** The toggle
  in 一般 starts off on a fresh install, matching iOS and Android; a Mac that
  turned it on keeps it on. When it is on, the space is decided by the string
  that reached the document rather than by the output mode. A candidate with no
  漢字 — the 字面羅馬字 below, an out-of-vocabulary name — writes romanization
  under every mode and now earns its space, where `taigi` and Return used to
  lose it under 漢字優先 and 漢羅濫, while the same list's dictionary
  romanization cell kept one. Committing 漢字 still takes no space, and
  括號標註's `台語 (tâi-gí)` counts as romanization. The swap that moves a
  trailing space after `?` / `!` / `,` now moves only a space this input method
  wrote, never one typed by hand. (#670)

- **`o͘ⁿ` finds the nasal final.** POJ writes the nasal /ɔ̃/ as `oⁿ`, but some
  writers spell it `o͘ⁿ` — the `o͘` key followed by `nn` on the POJ layout. That
  spelling now reaches the dictionary: 好, 否, 呼, 齁, 乎, 歟 and the words built
  on them. The rule that used to attempt this folded across the whole buffer and
  so fired over a syllable seam, losing real keys — 滷卵 (`lo͘nng`) and 可惡
  (`khooⁿ`) both went missing. The alias is indexed when the dictionary is
  built, where syllable boundaries still exist. (#685)

- **The `oo` and `nn` double taps stop cancelling each other.** In POJ, typing
  `hoonn` wrote `ho͘nn`: the `oo` folded and the `nn` did not, because the dot
  the first fold inserted was exactly what the nasal rule looked for in front of
  `nn`. The nasal folds first now — the order the canonical converter has always
  used. `oo` also folds in a single left-to-right pass, so a shift between the
  two taps (`hoO`) folds like the rest and `OOoo` no longer stacks three dots on
  one letter. (#686, #687)

#### Candidates

- **候選詞顯示 — how a candidate cell shows the word.** A picker in 外觀, three
  values. **漢羅並排** is today's cell: 漢字 and 羅馬字 side by side, the swap
  shortcut deciding which leads. **漢羅濫** lists both as candidates — a 漢字
  cell and, next to it, its 羅馬字 cell, no subtitles — so either script is one
  pick away and no swap is needed; Space on either commits the other.
  **羅馬字** shows the romanization alone and commits it: rows that would now
  read the same (食 and 𤆬 are both `tsia̍h`) are collapsed by the engine, so
  the window never offers two identical cells, and next-word predictions
  collapse the same way. Under 漢羅濫 and 羅馬字 the swap shortcut is inert and
  the stored swap waits for 並排; 括號標註 still applies under 漢羅濫
  (`漢字 (羅馬字)`) and has nothing to bracket under 羅馬字. Space on a
  one-label cell has no other script to commit and does nothing, as it already
  does on a romanization-only candidate. 方音齒盤 is unaffected. Changing the
  picker re-fetches an open candidate bar in place. **⌃⌘H** cycles the three
  from the keyboard — the new mode flashes, the bar re-fetches — and is
  re-recordable in 快捷鍵. (#662, #664)

- **顯示當咧拍的字 — the romanization being typed, as the first candidate.** A
  toggle in 一般, below 自動空白, on by default. While composing in Tâi-lô or
  POJ the candidate window leads with the preedit itself — `taigi` while typing
  `taigi`, `nn̄g` while typing `nng7` — so 漢羅 commits the romanization in one
  pick, and Return on a fresh bar writes what was typed. The dictionary's best
  candidate moves one place along; Space on the literal cell does nothing.
  Turn the toggle off and the bar leads with the dictionary candidate again.
  (#669, #673)

- **No cell appears twice under 漢羅濫 or 羅馬字.** A single-script cell hides
  what tells two entries apart, so a cell reading exactly like an earlier one
  is de-duplicated by the text it shows, first one wins: the 字面羅馬字 cell
  and a dictionary cell spelling the same romanization collapse into one, and
  重/tîng and 重/tāng draw one 重 cell — both romanization cells stay, so the
  losing reading is still one pick away. 漢羅並排 is untouched, its subtitle
  telling the pair apart. (#674)

#### Appearance

- **台 tile app icon.** The Mac app wore the mobile "Tâi" wordmark — its
  `AppIcon.icns` was the iOS asset catalogue re-packed by hand. It now wears the
  台 rounded square the menu bar already uses. iOS and Android keep the
  wordmark. (#643)

- **外觀 is one list of pop-up menus.** The light / dark / auto row was a set of
  drawn thumbnails in a section of its own; it is a pop-up menu like every
  other row in the pane now, with no divider fencing it off. (#671)

- **Two names.** The `` ` `` shortcut is called 輸出漢字/羅馬字 in 快捷鍵, where
  it read 漢字/羅馬字代先; the 漢字 interface language is listed by its own
  name, 台漢. Both changed on every platform. (#671)

- **One message per outcome.** A shortcut that cannot be recorded says
  已經予佔用 whatever holds it, instead of one message per kind of holder;
  saving, deleting, importing and exporting share 咧處理…; an update that does
  not go in says 安裝失敗. The recording prompt now uses 揤, the verb every
  other string uses. Both desktop platforms. (#678)

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
- **候選詞顯示** in 外觀 — 漢羅並排 / 漢羅濫 / 羅馬字, the same three cells as
  macOS (漢字 and 羅馬字 as adjacent one-script cells under 漢羅濫; romanization
  alone, same-reading rows collapsed, under 羅馬字; a list with no second
  script is one line tall). The `` ` `` swap is inert outside 並排;
  **Ctrl+Alt+H** cycles the three from the keyboard (the new mode flashes, an
  open list re-fetches in place) and is re-recordable in 快捷鍵. A change made
  in the settings window applies from the next keystroke, like the candidate
  layout. (#662, #664)
- **Tap Shift to switch 中/英**, the convention every Windows CJK input method
  follows: one Shift on its own, under half a second, with no other key in
  between. In 英 every keystroke goes straight to the document and the language
  bar reads 英; another tap returns to 台. On by default, and switchable off in
  一般. Ctrl+Alt shortcuts keep working in both modes. (#680)
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
  per-user scheduled task checks daily. An update downloads and installs from
  the window itself: the published manifest carries the installer's SHA-256, and
  a download that does not match it is refused. A copy that carries an
  Authenticode signature of its own also requires the new installer to be signed
  by the same certificate — the digest is checked either way, never instead.
  (#684)

#### Install

- **Not code-signed.** There is no Authenticode certificate yet, so SmartScreen
  warns on first run: choose 其他資訊 → 仍要執行. Every release publishes the
  installer's SHA-256, which is also what the in-app updater checks. (#682)
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
