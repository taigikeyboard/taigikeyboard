# desktop v3.9.0

Telex changes, a new candidate layout, and better candidates. Under Telex, `z`
now writes tone 6. `c` → ts / ch and `f` → `-` move to two opt-in settings
that also work with Numeric Tones. If you typed the hyphen with `f`, turn on
**Type - with f** under General. A new Vertical Expandable candidate window
unfolds into paged columns (macOS and Windows). Linux draws its own candidate
window, with Hanji and romanization on two lines. Manage Dictionaries can
search words. The Telex guide stays on screen until you hide it. Phonetic
Symbols gets a reworked key layout. The walker no longer splits a word into two
frequent single syllables. About 770 words from the MOE Dictionary and LKK
are added, most with no Hanji. Windows ships one installer for x64 and Arm64.
Upgrade from v3.8.0.

### macOS

#### Input

- **Telex: `z` writes tone 6.** `sit` + `z` gives `sǐt`. Before, `z` typed the
  affricate ts / ch. (#730)
- **ts / ch with `c` and the hyphen with `f` are now settings, off by
  default.** **Type ts / ch with c** and **Type - with f** under General work
  with Telex and Numeric Tones alike. If you typed `-` with `f` under Telex,
  turn on **Type - with f** to keep it. An `f` with nothing being typed then
  writes `-`, so `ffah` gives `--ah`. While it is on, `f` no longer picks a
  candidate under Numeric Tones; the candidate keys shift left. An `h` after
  `tsh` / `chh` is now an ordinary letter. Both settings are greyed out under
  Phonetic Symbols. (#731, #823, #835)
- **The Telex guide stays on screen** until its shortcut hides it, like the
  Phonetic Symbols Mini Keyboard, instead of closing on the next key. It is
  smaller and sits at the bottom centre of the screen. (#687, #688)
- **`\` types 、, `^` types …… and `_` types ——** in Hanji mode (MOE
  ideographic comma, ellipsis and dash). Both are also in the symbol picker.
  `'` still types 、. (#527, #854)
- **Punctuation writes the highlighted candidate.** Typing `ho`, highlighting
  好 and pressing `?` writes 好？, not ho？. (#553)
- **Output the Other Script writes a one-script candidate**, such as what you
  typed or a romanization-only word, instead of doing nothing. (#748)
- **Auto-Space after a Han-Lo word that ends in romanization.** Picking 啥mih
  adds the space, as a romanized pick does. (#856)
- **The Hanji / Romanization swap is always announced** under Side by Side,
  with or without a candidate list. (#570)
- **Fewer lost keystrokes when switching input methods quickly.** Switching
  rapidly between ABC and TaigiKeyboard no longer leaves letters passing
  through with no candidates. (#538)

#### Phonetic Symbols

- **Reworked key layout** at a typist's request. ㆨ moves to `m`, ㆦ / ㆧ to
  `.` / `>`, ㆲ / ㆱ to `-` / `_`, ㆭ to `?` and ㆬ to `P`. The stop codas move
  one key: ㆴ `t`, ㆵ `g`, ㆻ `b`. The hyphen moves to `=`. Keys shared with
  Mandarin Zhuyin stay where they were. The Mini Keyboard shows the new layout.
  (#806)
- **With Show Candidate Window off, you get the symbols you typed**: Enter and
  Space write the symbols, not Hanji, as Romanization does. (#804)
- **The tone-8 dot after a stop coda sits over the coda** (ㆵ̇), not beside it.
  (#805)
- **↓ lists the word before the cursor.** After a pick, ↓ used to list the
  next word instead. Opening the list and pressing Esc keeps your pick. (#807)
- **Custom Dictionary words and learned phrases show up as candidates**, as
  they do with Romanization. (#808)

#### Candidates

- **Words are no longer split into two common syllables.** `ue5` gives 鞋, not
  有的; `khikhi` gives 起去. (#564, #565)
- **A word you picked comes first once it covers what you typed.** After
  picking 按怎 for `antsua`, it comes ahead of 安紙. The same goes for 更新
  (`kingsi`) and 有影 (`uia`). (#753)
- **Capitalised and lowercase typing share what you picked.** A word picked as
  `Kong` comes up for `kong` too. (#755)
- **Hyphen in joined words follows your dictionaries.** A word you build from
  two picks is joined with a hyphen only when one of the dictionaries you have
  on lists it with that reading. (#788)
- **About 770 words, most with no Hanji.** The MOE Dictionary and other official
  sources keep their words with no Hanji (`a-lú-mih`), and LKK adds its
  romanized recommendations. A sentence with one of them reads `我má-tsiah`,
  not all in romanization. LKK also gains mixed spellings such as `lóo力` and
  `sau聲`. (#525, #702)

#### Candidate Window

- **Vertical Expandable**, a new candidate window layout. It opens as a column
  of nine. Press `→` to unfold up to three columns a page, with `←` / `→`
  between columns and Page Up / Page Down between pages. The previous
  Expandable is now called Horizontal Expandable. (#573, #780)

#### Settings

- **Search in Manage Dictionaries.** Type a word to see up to five matches with
  their dictionaries; click a match to look it up in the MOE Dictionary or
  ChhoeTaigi. (#794, #797, #798)
- **Learning Records remembers** the kind (Word Frequency / Phrases) and the
  order you chose. (#789)
- **Edit before Add to Custom Dictionary.** On Learning Records, the button opens
  the Add Word sheet filled in, so you can fix hyphens or capitals first.
  (#790)
- **Custom Dictionary keeps a Latin Hanji form** such as `KTV` or `5W1H`. A
  Hanji column that only repeats the romanization is stored as a
  romanization-only word. (#752, #820)

### Windows

#### Install

- **One installer for x64 and Arm64 Windows 11.** On an Arm64 PC the input
  method runs natively. Arm64 Windows 10 is not supported. (#760, #764, #768)
- **Uninstalling removes the input method's files** even when an app still had
  it loaded. They go at the next restart. (#776)

#### Input

- **Telex: `z` writes tone 6.** `sit` + `z` gives `sǐt`. Before, `z` typed the
  affricate ts / ch. (#730)
- **ts / ch with `c` and the hyphen with `f` are now settings, off by
  default.** **Type ts / ch with c** and **Type - with f** under General work
  with Telex and Numeric Tones alike. If you typed `-` with `f` under Telex,
  turn on **Type - with f** to keep it. An `f` with nothing being typed then
  writes `-`, so `ffah` gives `--ah`. While it is on, `f` no longer picks a
  candidate under Numeric Tones; the candidate keys shift left. An `h` after
  `tsh` / `chh` is now an ordinary letter. Both settings are greyed out under
  Phonetic Symbols. (#731, #823, #835)
- **The Telex guide stays on screen** until its shortcut hides it, like the
  Phonetic Symbols Mini Keyboard, instead of closing on the next key. (#687,
  #695)
- **`\` types 、, `^` types …… and `_` types ——** in Hanji mode (MOE
  ideographic comma, ellipsis and dash). Both are also in the symbol picker.
  `'` still types 、. (#527, #854)
- **Punctuation writes the highlighted candidate.** Typing `ho`, highlighting
  好 and pressing `?` writes 好？, not ho？. (#553)
- **Output the Other Script writes a one-script candidate**, such as what you
  typed or a romanization-only word, instead of doing nothing. (#748)
- **Auto-Space after a Han-Lo word that ends in romanization.** Picking 啥mih
  adds the space, as a romanized pick does. (#856)
- **The Hanji / Romanization swap shows a notice** under Side by Side. Before,
  only the tray icon changed. (#570)

#### Phonetic Symbols

- **Reworked key layout** at a typist's request. ㆨ moves to `m`, ㆦ / ㆧ to
  `.` / `>`, ㆲ / ㆱ to `-` / `_`, ㆭ to `?` and ㆬ to `P`. The stop codas move
  one key: ㆴ `t`, ㆵ `g`, ㆻ `b`. The hyphen moves to `=`. Keys shared with
  Mandarin Zhuyin stay where they were. The Mini Keyboard shows the new layout.
  (#806)
- **With Show Candidate Window off, you get the symbols you typed**: Enter and
  Space write the symbols, not Hanji, as Romanization does. (#804)
- **The tone-8 dot after a stop coda sits over the coda** (ㆵ̇), not beside it.
  (#805)
- **↓ lists the word before the cursor.** After a pick, ↓ used to list the
  next word instead. Opening the list and pressing Esc keeps your pick. (#807)
- **Custom Dictionary words and learned phrases show up as candidates**, as
  they do with Romanization. (#808)

#### Candidates

- **Words are no longer split into two common syllables.** `ue5` gives 鞋, not
  有的; `khikhi` gives 起去. (#564, #565)
- **A word you picked comes first once it covers what you typed.** After
  picking 按怎 for `antsua`, it comes ahead of 安紙. The same goes for 更新
  (`kingsi`) and 有影 (`uia`). (#753)
- **Capitalised and lowercase typing share what you picked.** A word picked as
  `Kong` comes up for `kong` too. (#755)
- **Hyphen in joined words follows your dictionaries.** A word you build from
  two picks is joined with a hyphen only when one of the dictionaries you have
  on lists it with that reading. (#788)
- **About 770 words, most with no Hanji.** The MOE Dictionary and other official
  sources keep their words with no Hanji (`a-lú-mih`), and LKK adds its
  romanized recommendations. A sentence with one of them reads `我má-tsiah`,
  not all in romanization. LKK also gains mixed spellings such as `lóo力` and
  `sau聲`. (#525, #702)

#### Candidate Window

- **Vertical Expandable**, a new candidate window layout. It opens as a column
  of nine. Press `→` to unfold up to three columns a page, with `←` / `→`
  between columns and Page Up / Page Down between pages. The previous
  Expandable is now called Horizontal Expandable. (#572, #778)
- **Unfolding and folding are animated**, as on macOS. (#783)
- **The candidate window stays open in Google Docs** in Chrome when POJ `o͘` is
  in the middle of what you type. (#569)
- **Candidate keys are labelled correctly** after unfolding Horizontal
  Expandable or scrolling Vertical. (#779)

#### Settings

- **Search in Manage Dictionaries.** Type a word to see up to five matches with
  their dictionaries; click a match to look it up in the MOE Dictionary or
  ChhoeTaigi. (#793, #797, #798)
- **Learning Records remembers** the kind (Word Frequency / Phrases) and the
  order you chose. (#789)
- **Edit before Add to Custom Dictionary.** On Learning Records, the button opens
  the Add Word dialog filled in, so you can fix hyphens or capitals first.
  (#790)
- **Custom Dictionary keeps a Latin Hanji form** such as `KTV` or `5W1H`. A
  Hanji column that only repeats the romanization is stored as a
  romanization-only word. (#752, #820)

### Linux

#### Candidate Window

- **TaigiKeyboard draws its own candidate window** on Fcitx5 and IBus, instead
  of the framework's list. A horizontal list puts Hanji and romanization on
  two lines. Appearance gains sizes and a typeface for each script, corner
  radius, padding, highlight fill and colour themes (Default follows the
  desktop's light / dark setting; Sakura, Gold, Sea Breeze, Jade, Wisteria,
  Catppuccin and Custom). The window is placed at the cursor on X11; on
  Wayland it shows but may not sit at the cursor. (#839)

#### Input

- **Telex: `z` writes tone 6.** `sit` + `z` gives `sǐt`. Before, `z` typed the
  affricate ts / ch. (#730)
- **ts / ch with `c` and the hyphen with `f` are now settings, off by
  default.** **Type ts / ch with c** and **Type - with f** under General work
  with Telex and Numeric Tones alike. If you typed `-` with `f` under Telex,
  turn on **Type - with f** to keep it. An `f` with nothing being typed then
  writes `-`, so `ffah` gives `--ah`. While it is on, `f` no longer picks a
  candidate under Numeric Tones; the candidate keys shift left. An `h` after
  `tsh` / `chh` is now an ordinary letter. Both settings are greyed out under
  Phonetic Symbols. (#731, #823, #835)
- **The Telex guide stays up** until its shortcut hides it, instead of closing
  on the next key. While candidates or the symbol picker are shown it gives
  way, and returns when you write or cancel. (#687, #694)
- **`\` types 、, `^` types …… and `_` types ——** in Hanji mode (MOE
  ideographic comma, ellipsis and dash). Both are also in the symbol picker.
  `'` still types 、. (#527, #854)
- **Punctuation writes the highlighted candidate.** Typing `ho`, highlighting
  好 and pressing `?` writes 好？, not ho？. (#553)
- **Output the Other Script writes a one-script candidate**, such as what you
  typed or a romanization-only word, instead of doing nothing. (#748)
- **Auto-Space after a Han-Lo word that ends in romanization.** Picking 啥mih
  adds the space, as a romanized pick does. (#856)
- **The Hanji / Romanization swap is announced** under Side by Side: the mode
  label shows the output script and Fcitx5 shows a notice. IBus has no notice.
  (#570)

#### Phonetic Symbols

- **Reworked key layout** at a typist's request. ㆨ moves to `m`, ㆦ / ㆧ to
  `.` / `>`, ㆲ / ㆱ to `-` / `_`, ㆭ to `?` and ㆬ to `P`. The stop codas move
  one key: ㆴ `t`, ㆵ `g`, ㆻ `b`. The hyphen moves to `=`. Keys shared with
  Mandarin Zhuyin stay where they were. (#806)
- **With Show Candidate Window off, you get the symbols you typed**: Enter and
  Space write the symbols, not Hanji, as Romanization does. (#804)
- **The tone-8 dot after a stop coda sits over the coda** (ㆵ̇), not beside it.
  (#805)
- **↓ lists the word before the cursor.** After a pick, ↓ used to list the
  next word instead. Opening the list and pressing Esc keeps your pick. (#807)
- **Custom Dictionary words and learned phrases show up as candidates**, as
  they do with Romanization. (#808)

#### Candidates

- **Words are no longer split into two common syllables.** `ue5` gives 鞋, not
  有的; `khikhi` gives 起去. (#564, #565)
- **A word you picked comes first once it covers what you typed.** After
  picking 按怎 for `antsua`, it comes ahead of 安紙. The same goes for 更新
  (`kingsi`) and 有影 (`uia`). (#753)
- **Capitalised and lowercase typing share what you picked.** A word picked as
  `Kong` comes up for `kong` too. (#755)
- **Hyphen in joined words follows your dictionaries.** A word you build from
  two picks is joined with a hyphen only when one of the dictionaries you have
  on lists it with that reading. (#788)
- **About 770 words, most with no Hanji.** The MOE Dictionary and other official
  sources keep their words with no Hanji (`a-lú-mih`), and LKK adds its
  romanized recommendations. A sentence with one of them reads `我má-tsiah`,
  not all in romanization. LKK also gains mixed spellings such as `lóo力` and
  `sau聲`. (#525, #702)

#### Settings

- **Search in Manage Dictionaries.** Type a word to see up to five matches with
  their dictionaries; open a match to look it up in the MOE Dictionary or
  ChhoeTaigi. (#793, #797, #798)
- **Learning Records remembers** the kind (Word Frequency / Phrases) and the
  order you chose. (#789)
- **Edit before Add to Custom Dictionary.** On Learning Records, the button opens
  the Add Word dialog filled in, so you can fix hyphens or capitals first.
  (#790)
- **Custom Dictionary keeps a Latin Hanji form** such as `KTV` or `5W1H`. A
  Hanji column that only repeats the romanization is stored as a
  romanization-only word. (#752, #820)
