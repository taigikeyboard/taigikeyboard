## v3.9.0

Mobile release for iOS and Android. Headline: **more control over Custom
Layouts and themes** — edit each key's long-press characters, duplicate a
custom theme or layout, start a theme from a built-in one, and let TL and POJ
each keep their own layout. Candidates get a bigram check that stops splitting
one word into two frequent syllables, picked words beat the composed sentence,
and Phonetic Symbols conversion finds your custom words. Custom Dictionary
pages through every word. The dictionary keeps official words written without
Hanji. Desktop work in the range (Telex keys, Shortcuts pane, vertical
expandable candidates, the web version) belongs to the desktop train and does
not touch phones.

### Shared (iOS + Android)

#### New

- **Long-Press Characters in Custom Layouts.** Select a character key in the
  layout editor and its long-press list appears as a bubble above the board:
  tap a slot, then a tray character to fill it; remove, reorder or reset to the
  built-in list. Up to nine characters; an empty list turns long-press off.
  The tray groups letters, tone rows and symbols. The custom list replaces the
  built-in one on the TL / POJ letter page. With full-width punctuation on,
  each entry types the form the board would (`,` → `，`, `{` → `｛`).
  (#577, #579, #581, #833, #836)
- **Duplicate** on custom theme and custom layout cards saves a copy named
  "{name} Copy" right after the original. Hidden once you have five. (#580)
- **Customize a built-in theme.** A built-in theme card's menu offers Apply /
  Customize; the editor starts from that theme's colors and outline, and every
  reset returns to it. (#575)
- **TL and POJ each remember their layout.** Switching romanization script
  brings back the layout (built-in or custom) you last used with it; applying a
  custom layout switches to its script. New Layout… asks for the Layout Name
  and its use (Universal / TL / POJ) before the editor opens; a Universal
  layout's long-press lists carry both the TL and the POJ tone-9 mark
  (`a` → a̋ ă). (#666, #836)
- **Questions & Discussion** row with the Discord logo at the end of Home's
  Other section. (#814)

#### Improved

- **Custom Layout tray additions**: the nasal marker `ⁿ`, the final `or`, and
  the second symbol page (`{` `«` `€` `→` …, typed in full-width form). With a
  key selected, chips that cannot go on it are dimmed. The `er` / `ir` / `or`
  keys get tone long-press lists. (#571, #691, #803, #832)
- **Custom Dictionary lists every word.** The list loads page by page as you
  scroll, instead of stopping at 100, and search runs in the engine, so a
  numbered reading such as `tsui2` finds its words. A failed read keeps the
  list and shows a retry row. (#697)
- **Fewer one-word splits.** A bigram check keeps the conversion from
  splitting one word into two frequent syllables: `ue5` gives 鞋 instead of
  有的, `khikhi` 起去 instead of 去去. (#563, #564, #565)
- **A word you keep picking beats the composed sentence.** `antsua` now gives
  按怎 first once you have picked it, instead of 安紙. (#753)
- **Phonetic Symbols conversion uses your custom words and learned phrases**:
  a custom 陳三 typed `ㄉㄢˊㄙㆩ` gives 陳三, not 陳相. (#808)
- **Phonetic Symbols ↓ after a pick** lists the word you just picked, so the
  list stays on the same side of the caret. (#807)
- **Khinsiann dot `·` after an auto space** replaces the space:
  `hōo·guá`, not `hōo ·guá`. (#698)

#### Changed

- **Custom layout swap is explicit.** Tapping a key only selects it (and shows
  its long-press list); press the swap button below the board, then tap the
  key to swap with. Drag is gone from the editor. (#578, #682)
- **Reset settings, Delete All custom words and Delete Learning Records run on
  press**, with no confirmation dialog; a notice appears only when one fails.
  Export a `.taigi` backup first if you may want the data back. (#519)
- **Hanji labels write 的 for possessive ê** (使用的 App). Romanized labels
  keep ê. (#522)
- **Readings ignore capital letters for identity.** A custom or learned
  `Tâi-uân` and `tâi-uân` are one word; capitals still show where you typed
  them. (#755)
- **A custom word whose Hanji is just its romanization** (`lán,lán`) is
  stored without Hanji, as the shipped dictionary does, so it no longer shows
  as a second copy beside the dictionary word. Existing rows are repaired on
  first launch. A Latin Hanji that differs from the roman (`KTV`, `5W1H`) is
  kept. (#752, #820)

#### Bug Fixes

- **Phonetic Symbols tone-8 dot after a stop final** (`ㄒㄧㆻ˙`) keeps
  composing and is stored as the combining dot over the final; Space then
  commits instead of adding a separator. (#805, #822)
- **Compound hyphen follows the dictionary switches.** With Taihoa / iTaigi
  off, picking 俺 then 娘 writes `án niâ` like the first candidate, not
  `án-niâ`. (#788)
- **Custom words rank below the words you pick**: a custom word you never
  picked comes after picked words, and a custom word reached only by its
  abbreviation comes after full readings (`thng` lists 湯 and 糖 before a
  custom 退伍). (#515, #517)
- A custom word reached by abbreviation counts its real syllables, so a
  following compound joins correctly. (#516)
- **Full-width punctuation after an auto space** takes the space's place:
  `想beh` + `，` gives `想beh，`, with no space left before or after the mark.
  Half-width marks keep the space after them (`guá? `). (#860)
- **A Han-Lo pick that ends in romanization** (`啥mih`) earns the auto space
  under Hanji output, like a romanized pick; `m̄好` still does not. (#856)

### iOS

#### Improved

- The layout editor is usable in phone landscape. (#582)
- KeyboardKit 11.1.4. (#568)

### Android

#### Improved

- **Punctuation long-press on the letter page** matches iOS: PhahTaigi
  `! ? , .` and Lohankha `,` now open their symbol lists (`,` → ， 、,
  `.` → 。 ‧ ·). (#583)
- The layout editor scrolls in landscape, and its pills, tray and gallery
  match iOS. (#581, #584, #685)

#### Bug Fixes

- **Toolbar labels and icons keep one size** on every phone, independent of
  the system font scale and slot width. (#680, #686)

### Dictionary

- **Official words without Hanji are kept.** Roman-only and Han-Lo words from
  Kautian, Taiwanese TV, Kungge and STTI no longer drop out; LKK adds 386 fully
  romanized words, and its mixed-script supplement grows. A roman-only word
  inside a sentence keeps the sentence Hanji (`我má-tsiah`). (#525, #702)
- 二來 `jī--lâi` is found with the default dictionary sources (`ji7lai5`,
  `jilai`). (#872)
- 31 neutral-tone pieces that start with `--` (`--ah`, 起來 `--khí-lâi`,
  一下 `--tsi̍t-ē`). (#799)
- Merging keeps source order, so an unrelated addition no longer changes a
  word's chosen TL spelling. (#555)
- Kautian (MOE Dictionary) data refreshed to its latest release. (#537, #557,
  #559, #658)
