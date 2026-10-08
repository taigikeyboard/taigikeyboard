# desktop v3.7.0

The Phonetic Symbols release, and the first desktop release on one corpus.
Taiwanese Phonetic Symbols (方音符號) is now an input script on macOS, Windows
and Linux, with Hanji conversion in the preedit and an on-screen key chart.
Continuous input now segments, picks its first word and orders its candidate
list from how often written Taigi actually uses each word: `kausiu` gives 教授,
`kap` gives 佮. A new Learning Records page shows and edits what the keyboard
has learned. Syllable Separator replaces No Hyphens. Typed tone digits stay
tone marks in the preview. The keyboard engine now manages your saved words,
custom dictionary and learning records on every desktop. macOS and Windows
upgrade from v3.6.9, Linux from v3.6.10.

### macOS

#### Input

- **Phonetic Symbols (方音符號) input.** Choose Phonetic Symbols under Input
  Script, or press ⌃⌘P; ⌃⌘P again returns to the romanization you used last.
  The keys follow the Dachen Zhuyin layout, and Taigi-only symbols sit on free
  keys and the Shift layer. The preedit converts to Hanji as you type. ↓, or
  Space after a finished syllable, opens the candidates for the word before the
  caret, right under that word. 1–9 on the number row or keypad pick. Return
  commits what is shown and ⇧Return commits the symbols. Only full-width
  punctuation is typed in this mode. The swap and Telex guide shortcuts do
  nothing in it. ㆳ is read as ㆪ for every lookup. (#374, #376, #403, #404, #437,
  #443, #445)
- **Show Phonetic Symbols Mini Keyboard (⌃⌘J).** An on-screen chart of the
  Phonetic Symbols keys, also in the input-source menu. Clicking a key types it,
  and the key you just typed lights up briefly. (#378, #381, #436, #451)
- **Keyboard Layout: QWERTY, Dvorak or Colemak.** A new picker under Input
  Script. TaigiKeyboard reads romanization keys in the layout picked here, which
  is QWERTY by default, whatever the system layout is. If you type on Dvorak or
  Colemak, pick it once after upgrading. Other layouts, such as AZERTY, type as
  QWERTY. Phonetic Symbols always uses QWERTY. (#431)
- **Continuous input picks the word written Taigi uses most.** Segmentation and
  the first candidate now come from word counts in a Taigi text corpus.
  Previously they came from a mix of character and word counts.
  `kau3siu7` and `kausiu` give 教授 (were 到受 and 狗岫). `kap` gives 佮 (was 甲).
  Toneless `koh` gives 閣 and `beh` gives 欲. A word you have picked before still
  comes first, and so does the word that usually follows the previous one.
  (#461, #462, #463, #466)
- **The candidate list follows the same counts.** The words after the first
  candidate are ordered the same way. For example, in `taiuanlang` the
  candidates for the shorter spans list 台灣 before 大 and 台, and the `tt`
  abbreviation leads with 得著 and 直直. (#468)
- **The previous word steers homophones.** Among homophones, the one that
  usually follows the word you just wrote comes first. That word is the last
  segment you fixed, or your last commit within 10 seconds. `，、；：` end that
  context. (#270, #271, #272, #341)
- **Tone digits stay tone marks.** Typing on after a digit no longer turns the
  preview back into digits: `tai5gi2` shows `tâigí`, and `taigi2` shows
  `taigí`. Tones 1 and 4 drop the digit. Runs of digits stay literal (`covid19`,
  `win10`). POJ marks land on the canonical vowel (`hoang2` → `hoáng`). This works
  with the numeric scheme and with Telex. (#449, #450)
- **A hyphen closes a single syllable.** `siam-tioh` no longer offers 寫 or 匙仔
  inside `siam`, and `ai-` no longer offers 阿姨. A digit right after the hyphen
  is that syllable's tone, so `tai-2` gives 台. (#454)
- **⌥↑ / ⌥↓ and Home / End jump to the start / end of the composition**
  (fn+← / fn+→ on a laptop). Before, they committed and passed the key on.
  (#408)
- **⇧` types full-width ～ in Hanji mode.** ⌃⇧` still reaches the app. (#405)
- **⌃ + punctuation no longer types the other width.** The chord is gone, and
  ⌃, and the other ⌃ + punctuation shortcuts now reach the app as normal
  shortcuts. This fixes ⌃, typing nothing in Chromium apps and Notes. Width
  follows the output: full-width with Hanji first, half-width with romanization
  first. Switch it with `` ` ``. (#438)

#### Candidates

- **The same word is listed once.** `li2` no longer shows 李 twice. A custom
  word that matches a dictionary word is merged with it, and the custom one comes
  first. (#447)
- **The separator form you pick stays first.** After you pick `gín-á`, it no
  longer falls behind `gín--á`, and the same goes for other words with both
  forms. (#418)
- **A custom word with a spaced reading ranks first.** For example
  `tshì-giām sû` / 試驗辭. (#411)
- **Custom readings saved with tone digits show tone marks.** `tsui2-ong5-tshut`
  shows `tsuí-ông-tshut`. (#453)
- **The swap chord flashes the new output script.** With no list open and
  candidates shown side by side, `` ` `` briefly shows which script is now
  first. (#409)
- **New defaults: Vertical layout, typed text off.** Candidate Window Layout now
  defaults to Vertical, and Show Typed Text First now defaults to off. If you
  never changed these settings, both new defaults apply after upgrading. A value
  you set yourself is kept. (#337, #382)

#### Settings

- **Learning Records.** A new pane after Custom Dictionary. It has two lists,
  Word Frequency and Phrases, sorted by Most Used or Most Recent, with a filter.
  You can edit a count or delete a row. Add to Custom Dictionary copies a word
  frequency row and moves a phrase. It works on any row with Hanji, including
  one-syllable rows. Delete Learning Records has moved here from Custom
  Dictionary, and its confirmation says that custom words are kept. (#370, #415,
  #422, #425, #427)
- **Custom Dictionary asks before Delete All Custom Words** and before deleting
  learning records. After a failed save or a partial CSV import, the list
  reloads. (#386, #395)
- **Syllable Separator replaces No Hyphens.** Choose Hyphen (the default), Space
  or None. If No Hyphens was on, you get None. (#423)
- **Nasal mark in POJ capitals is now a choice of ᴺ or ⁿ.** Your stored setting
  is kept. (#406)
- **New installs start with an empty Custom Dictionary.** Existing dictionaries
  keep their words. (#426)
- **Menus and labels.** The settings row now reads TaigiKeyboard Settings. The
  input-source menu adds Switch Phonetic Symbols (⌃⌘P) and Show Phonetic
  Symbols Mini Keyboard (⌃⌘J). Several Taigi labels are reworded. If ⌃⌘P is
  already one of your recorded shortcuts, your recording wins. (#177, #414,
  #424, #440)

#### Appearance

- **Font weight.** Manage Fonts shows a Weight picker for installed font
  families that have more than one weight. Leaving it empty keeps the family's
  default weight. (#384)
- **About** adds Facebook, Instagram and Threads links, and the Discord invite
  link is updated. (#247)

#### Install

- **Your words move to the keyboard engine.** On first launch, the stores for
  word frequency, associations, custom dictionary and learned phrases are taken
  over by the keyboard engine. A `.pre-engine` copy of each is kept beside it.
  Custom-dictionary readings are tidied once: Unicode is normalized and tone
  digits become tone marks. Hanji, counts and dates are kept. (#229, #448, #453)
- The dictionary files are about 3 MB larger. They now carry the corpus word
  counts and word-pair data. (#461, #462)

### Windows

#### Input

- **Phonetic Symbols (方音符號) input.** Choose Phonetic Symbols under General →
  Input Script, or press Ctrl+Alt+P; Ctrl+Alt+P again returns to the
  romanization you used last. The keys follow the Dachen Zhuyin layout, and
  Taigi-only symbols sit on free keys and the Shift layer. The preedit converts to Hanji
  as you type. ↓, or Space after a finished syllable, opens the candidates for
  the word before the caret. 1–9 on the number row or keypad pick. Enter
  commits what is shown and Shift+Enter commits the symbols. Only full-width
  punctuation is typed in this mode. ㆳ is read as ㆪ. (#372, #373, #376, #403,
  #437, #443, #445)
- **Show Phonetic Symbols Mini Keyboard (Ctrl+Alt+J).** An on-screen chart of
  the Phonetic Symbols keys, also in the language-bar menu. Clicking a key types
  it, and the key you just typed lights up briefly. (#378, #381, #436, #451)
- **Keyboard Layout: QWERTY, Dvorak or Colemak.** A new setting in General.
  Windows switches to the US layout while TaigiKeyboard is active, so this is
  where you tell it your layout. Romanization keys and Ctrl / Alt shortcuts
  follow the setting. Phonetic Symbols stays on QWERTY. (#444)
- **Continuous input picks the word written Taigi uses most.** Segmentation and
  the first candidate now come from word counts in a Taigi text corpus.
  Previously they came from a mix of character and word counts.
  `kau3siu7` and `kausiu` give 教授 (were 到受 and 狗岫). `kap` gives 佮 (was 甲).
  Toneless `koh` gives 閣 and `beh` gives 欲. A word you have picked before still
  comes first, and so does the word that usually follows the previous one.
  (#461, #462, #463, #466)
- **The candidate list follows the same counts.** The words after the first
  candidate are ordered the same way. For example, in `taiuanlang` the
  candidates for the shorter spans list 台灣 before 大 and 台, and the `tt`
  abbreviation leads with 得著 and 直直. (#468)
- **The previous word steers homophones.** Among homophones, the one that
  usually follows the word you just wrote comes first. That word is the last
  segment you fixed, or your last commit within 10 seconds. `，、；：` end that
  context. (#270, #271, #272, #341)
- **Tone digits stay tone marks.** `tai5gi2` shows `tâigí`, and `taigi2` shows
  `taigí`. Tones 1 and 4 drop the digit. Runs of digits stay literal (`covid19`,
  `win10`). POJ marks land on the canonical vowel (`hoang2` → `hoáng`). This works
  with the numeric scheme and with Telex. (#449, #450)
- **A hyphen closes a single syllable.** `siam-tioh` no longer offers 寫 or 匙仔
  inside `siam`, and `ai-` no longer offers 阿姨. A digit right after the hyphen
  is that syllable's tone, so `tai-2` gives 台. (#454)
- **Ctrl+↑ / Ctrl+↓ and Home / End jump to the start / end of the
  composition.** Before, they committed and passed the key on. (#408)
- **Shift+` types full-width ～ in Hanji mode.** (#405)
- **Ctrl + punctuation no longer types the other width.** The chord is gone,
  and Ctrl + punctuation now reaches the app as a normal shortcut. Width follows
  the output: full-width with Hanji first, half-width with romanization first.
  Switch it with `` ` ``. (#438)

#### Candidates

- **Font choice now reaches the candidate window.** 源樣明體, 源樣烏體, every
  installed font (MingLiU, DFKai-SB, Microsoft JhengHei…) and every weight used
  to draw in the system font; only 粉圓 and 芫荽 worked. They all apply now.
  The System choice now uses Segoe UI Variable Text on Windows 11. (#467)
- **PowerPoint shows the candidate window.** Before, typing in PowerPoint showed
  only an underline, and Space wrote the raw romanization. Now the list opens at
  the caret, and Space picks a candidate as it does in Word. (#385)
- **The candidate window sits at the caret in older apps at 125% scaling and
  above.** In apps that do not handle display scaling (classic WinForms text
  boxes, 32- and 64-bit), the window used to sit down and to the right and look
  enlarged. The symbol picker, mode flash, Telex guide and Phonetic Symbols
  chart are fixed too. The language-bar menu also opens where you click instead
  of stretched in the screen corner. (#387, #397)
- **Ctrl+Alt+H respects Show Candidate Window.** With the candidate window
  turned off, the shortcut no longer opens a list. (#292)
- **The same word is listed once.** `li2` no longer shows 李 twice. A custom
  word that matches a dictionary word is merged with it, and the custom one comes
  first. (#447)
- **The separator form you pick stays first** (`gín-á` over `gín--á`), and **a
  custom word with a spaced reading ranks first** (`tshì-giām sû` / 試驗辭).
  (#411, #418)
- **Custom readings saved with tone digits show tone marks.** `tsui2-ong5-tshut`
  shows `tsuí-ông-tshut`. (#453)
- **New defaults: Vertical layout, typed text off.** Candidate Window Layout now
  defaults to Vertical, and Show Typed Text First now defaults to off. If you
  never changed these settings, both new defaults apply after upgrading. A value
  you set yourself is kept. (#337, #382)

#### Settings

- **Learning Records.** A new page in the settings app. It has two lists, Word
  Frequency and Phrases, sorted by Most Used or Most Recent, with a filter. You
  can edit a count or delete a row. Add to Custom Dictionary copies a word
  frequency row and moves a phrase. It works on any row with Hanji. Delete
  Learning Records has moved here from Custom Dictionary, and it keeps custom
  words. (#370, #415, #422, #425, #427)
- **Syllable Separator replaces No Hyphens.** Choose Hyphen (the default), Space
  or None. If No Hyphens was on, you get None. (#423)
- **Nasal mark in POJ capitals is now a choice of ᴺ or ⁿ.** Your stored setting
  is kept. (#406)
- **Weight picker in Manage Fonts.** It appears for installed families that
  have more than one weight. (#383)
- **The settings app no longer closes** when you switch between Shortcuts and
  Manage Dictionaries. (#452)
- **New installs start with an empty Custom Dictionary.** Existing dictionaries
  keep their words. (#426)
- **Menus and labels.** The language-bar menu now has the same rows as macOS
  and Linux, and its settings row reads TaigiKeyboard Settings. "Manage
  Typefaces" is now Manage Fonts. About adds Facebook, Instagram and Threads.
  (#177, #247)

#### Appearance

- **The language-bar button shows the mode.** 台 / Ts (Tâi-lô, Hanji or
  romanization first), 白 / Ch (POJ), 方 (Phonetic Symbols) and 英 (English).
  It updates when you switch apps, so a change made in Settings shows before
  your next key. (#409)

#### Install

- **32-bit apps can type.** The installer adds `TaigiKeyboard32.dll` and
  registers it beside the 64-bit DLL, so 32-bit programs such as 32-bit Office
  can use the keyboard. To go back to an older version, uninstall first. (#377)
- **Your words move to the keyboard engine.** On first launch, the stores in
  `%APPDATA%\TaigiKeyboard` are taken over by the keyboard engine. A
  `.pre-engine` copy of each is kept beside it. Custom-dictionary readings are
  tidied once: Unicode is normalized and tone digits become tone marks. Hanji,
  counts and dates are kept. (#228, #448, #453)
- The dictionary files are about 3 MB larger. (#461, #462)

### Linux

#### Input

- **Phonetic Symbols (方音符號) input, on Fcitx5 and IBus.** Choose Phonetic
  Symbols under General → Input Script, or press Ctrl+Alt+P; Ctrl+Alt+P again
  returns to the romanization you used last. The keys follow the Dachen Zhuyin
  layout, and Taigi-only symbols sit on free keys and the Shift layer. The preedit converts to
  Hanji as you type. ↓ / ↑, the page keys, or Space after a finished syllable
  open the candidates. 1–9 on the number row or keypad pick. Enter commits what
  is shown, Shift+Enter commits the symbols, and Escape closes the list. Only
  full-width punctuation is typed in this mode. (#372, #373, #376, #403, #437,
  #443, #445)
- **Show Phonetic Symbols Mini Keyboard (Ctrl+Alt+J).** An on-screen chart of
  the Phonetic Symbols keys, also in the panel menu. On Linux it opens as its
  own window and is for reference only: clicking a key does not type it. It
  closes when you leave Phonetic Symbols. (#378, #451)
- **IBus keeps your keyboard layout on GNOME.** Choosing TaigiKeyboard no
  longer forces US QWERTY, so Dvorak and Colemak work. A non-Latin layout no
  longer switches to US by itself. Fcitx5 already behaved this way. (#442)
- **Continuous input picks the word written Taigi uses most.** Segmentation and
  the first candidate now come from word counts in a Taigi text corpus.
  Previously they came from a mix of character and word counts.
  `kau3siu7` and `kausiu` give 教授 (were 到受 and 狗岫). `kap` gives 佮 (was 甲).
  Toneless `koh` gives 閣 and `beh` gives 欲. A word you have picked before still
  comes first, and so does the word that usually follows the previous one.
  (#461, #462, #463, #466)
- **The candidate list follows the same counts.** The words after the first
  candidate are ordered the same way. For example, in `taiuanlang` the
  candidates for the shorter spans list 台灣 before 大 and 台, and the `tt`
  abbreviation leads with 得著 and 直直. (#468)
- **The previous word steers homophones.** Among homophones, the one that
  usually follows the word you just wrote comes first. `，、；：` end that
  context. (#270, #271, #272, #341)
- **Tone digits stay tone marks.** `tai5gi2` shows `tâigí`, and `taigi2` shows
  `taigí`. Tones 1 and 4 drop the digit. Runs of digits stay literal (`covid19`,
  `win10`). POJ marks land on the canonical vowel (`hoang2` → `hoáng`). This works
  with the numeric scheme and with Telex. (#449, #450)
- **A hyphen closes a single syllable.** `siam-tioh` no longer offers 寫 or 匙仔
  inside `siam`, and `ai-` no longer offers 阿姨. `tai-2` gives 台. (#454)
- **Ctrl+↑ / Ctrl+↓ and Home / End jump to the start / end of the
  composition.** When nothing is being composed, Home and End still go to the
  app. (#408)
- **Shift+` types full-width ～ in Hanji mode.** (#405)
- **Ctrl + punctuation no longer types the other width.** The chord is gone, and
  Ctrl + punctuation reaches the app again, which matters in editors such as VS
  Code. Width follows the output. Switch it with `` ` ``. (#438)

#### Candidates

- **The typed-text cell no longer takes the first selection key.** With Show
  Typed Text First on, the typed-text cell has no label and `q` picks the next
  candidate, as on macOS and Windows. (#455)
- **Show Typed Text First now defaults to off.** If you never changed it, it
  turns off after upgrading. A value you set yourself is kept. (#337)
- **The same word is listed once** (`li2` no longer shows 李 twice). **The
  separator form you pick stays first**, and **a custom word with a spaced
  reading ranks first**. (#411, #418, #447)
- **Custom readings saved with tone digits show tone marks.** `tsui2-ong5-tshut`
  shows `tsuí-ông-tshut`. (#453)

#### Mode indicator and menu

- **The tray shows the mode.** 台 / Ts (Tâi-lô, Hanji or romanization first), 白 /
  Ch (POJ), 方 (Phonetic Symbols). On Fcitx5 the tray now draws a mode icon. On
  GNOME's IBus panel it also follows changes made in the settings window.
  (#409, #446)
- **The panel menu matches macOS and Windows.** It has Switch TL/POJ, Switch
  Phonetic Symbols, Switch Candidate Display, Show Phonetic Symbols Mini
  Keyboard, TaigiKeyboard Settings (was a bare 設定, easy to confuse with
  Fcitx5's own) and About the Keyboard. Rows that do nothing in the current mode
  are hidden. (#172, #177, #373)

#### Settings

- **Learning Records.** A new pane after Custom Dictionary. It has two lists,
  Word Frequency and Phrases, sorted by Most Used or Most Recent, with a filter.
  You can edit a count, delete a row, or Add to Custom Dictionary for any row
  with Hanji. Delete Learning Records has moved here from Custom Dictionary. It
  now asks first, and it keeps custom words. (#369, #415, #422, #425, #427)
- **Syllable Separator replaces No Hyphens.** Choose Hyphen (the default), Space
  or None. If No Hyphens was on, you get None. (#423)
- **Nasal mark in POJ capitals is now a choice of ᴺ or ⁿ.** Your stored setting
  is kept. (#406)
- **Shortcuts** lists Ctrl+Alt+P, Ctrl+Alt+J and the new caret keys. The
  "Punctuation in the Other Width" row is gone. (#373, #378, #408, #438)
- **New installs start with an empty Custom Dictionary.** Existing dictionaries
  keep their words. (#426)
- **About** adds Facebook, Instagram and Threads, and the Discord invite link
  is updated. The English product name is TaigiKeyboard everywhere, including
  the input-method lists. (#203, #247)

#### Install

- **Each package uses its own distribution's layout.** On Fedora and Arch, the
  fonts go to `/usr/share/fonts/taigikeyboard/` and the licences to
  `/usr/share/licenses/taigikeyboard/`. On Arch, the IBus engine moves to
  `/usr/lib/ibus/`, so IBus users there need `ibus restart` or a new login after
  upgrading. Debian keeps its paths. Every package now ships the licence notices
  for the bundled dictionary and fonts. (#199)
- **The new mode icons are installed under `share/icons/hicolor`.** The IBus
  component changed too: run `ibus restart` or log in again to pick it up.
  (#409, #442)
- **Your words move to the keyboard engine.** On first launch, the stores in
  `~/.local/share/taigikeyboard` are taken over by the keyboard engine. A
  `.pre-engine` copy of each is kept beside it. Custom-dictionary readings are
  tidied once: Unicode is normalized and tone digits become tone marks. (#228,
  #448, #453)
- **Building from source:** `make install-common`, `install-ibus` and
  `install-fcitx5` install one part each. `INSTALL_FONTS=0` skips the bundled
  fonts. The build works with Rust 1.93. (#457, #459)
