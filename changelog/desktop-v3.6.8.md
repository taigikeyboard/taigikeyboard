# desktop v3.6.8

The typing release. Tone marks can now be typed the Telex way — letters instead
of digits — with a key card a shortcut raises; the candidate window can be turned
off entirely and the romanization committed as typed; a symbol picker opens the
whole punctuation table on one chord; the caret moves inside what is being
composed; ⇧ with a selection key commits that candidate in the other script; and
the candidate window can be drawn in a typeface the user supplies or in any the
OS already has. 快捷鍵 is
re-ordered and titled to read in the order the keys are met. macOS and Windows
have all of it.

### macOS

#### Input

- **聲調拍法 — type tones with letters.** 一般 gains a picker: tones stay on the
  digits, or move to Telex letters. Under Telex, `v y d w x q` write tones 2 3 5
  7 8 9, `z` and `zh` write ts / tsh (ch / chh in POJ), `f` writes the syllable
  hyphen — and the digits are free, so `1`–`9` pick candidates. The selection
  keys follow the scheme rather than being set separately: `q w d f z x v y ;`
  under the digit scheme, `1`–`9` under Telex. The ⇧ / ⌃ / ⌥ selection-key sets
  and their picker row are gone; a scheme now decides both halves. (#17, #18)
- **Telex 說明 — the key card, on ⌃⌘/.** A floating card lists key, meaning and
  an example, spelled for whichever romanization is active. `1`–`9` still pick
  while it is up, and the next key takes it down. Re-recordable in 快捷鍵; the
  legend that used to sit in 一般 is gone. (#21)
- **候選詞窗 can be turned off.** A switch in 一般, above 顯示當咧拍的字. With it
  off nothing is fetched and no window appears: Return, Space, Tab, `[` and `]`
  commit the composition exactly as typed — `tai5` writes `tâi` — and the digits
  keep whatever the tone scheme gives them. Turning it off mid-composition takes
  the window away at once. (#20)
- **⌥← / ⌥→ move the caret inside the composition.** While composing, Option with
  an arrow moves within the pending text instead of leaving it; the candidate
  list, its highlight and its page stay where they were. Idle, the chord is the
  host application's word jump, as before, and ⌥⇧ / ⌥⌘ / ⌥↑ stay the host's.
  (#29, #30)
- **符號選單 on ⌃⌘,.** One chord opens the whole table — punctuation, then
  bracket pairs, then special symbols — and the first key is already a pick, with
  no category step in between. A bracket pair is one entry and inserts both
  halves, leaving the caret after the closing one. Escape closes it. The bare
  `` ` `` key is untouched: it still swaps 漢字 / 羅馬字. (#26, #28)

#### Candidates

- **⇧ + a selection key commits that candidate in the other script.** Space has
  always committed the highlighted cell in the script it does not stand for;
  reaching another cell that way meant walking the highlight there first. ⇧ with
  a selection key does it in one press. (#35)
- **The 字面羅馬字 cell no longer takes a selection key.** With 顯示當咧拍的字 on,
  the first cell is what is being typed, and the keys now start on the cell after
  it — so the first key of the set picks the first dictionary candidate. The page
  is not enlarged: eight keyed cells, as before. (#25)

#### Appearance

- **A typeface of your own for the candidate window.** 字型管理 was a closed
  roster of five; a font file can now be added to it, used, and removed again.
  Added faces are listed under the file name they were given rather than the name
  the file declares — the same file used to list differently on each platform,
  and a declared name can mean nothing to the person who chose the file. The
  bundled five keep their own names. (#16, #37)
- **A typeface removed and added again is accepted.** Removing a face and
  re-importing the same file was refused with "another typeface is already called
  …", for a font that was no longer there: macOS keeps a lookup that an
  unregister never clears, and the check believed it. (#37)
- **The typefaces the Mac already has are in the list.** 字型管理 lists every
  installed family after the bundled five and the added files, so a face on the
  Mac is one click away rather than an import that was refused with "another
  typeface is already called …". A search field and a pager — one page of ten
  rows, as 自訂詞庫 has — keep a few hundred families usable in one table.
  Importing a file whose face the Mac already has selects that row and says so
  instead of failing; a family installed or removed in Font Book is picked up
  without restarting the input method. (#45)

#### Settings

- **快捷鍵 reads in the order the keys are met** — through the candidates, out of
  the composition, then the switches — under three headings: 選字, 拍字, 其他.
  輸出漢字/羅馬字 sits with 拍字, since it decides which script every commit above
  it writes. Wording throughout is plainer, and the ⇧ row prints the whole set it
  stands for. (#33, #36, #38)

### Windows

#### Input

- **聲調拍法 — type tones with letters.** 一般 gains the same picker as macOS:
  tones on the digits, or on Telex letters — `v y d w x q` for tones 2 3 5 7 8 9,
  `z` / `zh` for ts / tsh, `f` for the syllable hyphen — with the digits then free
  to pick candidates. Selection keys follow the scheme (`q w d f z x v y ;`, or
  `1`–`9` under Telex), and the Shift / Ctrl / Alt selection-key sets and their
  picker row are gone. (#17, #19)
- **Telex 說明 — the key card, on Ctrl+Alt+/.** The same card as the Mac's, built
  from the same rows and spelled for the active romanization; `1`–`9` still pick
  while it is up. Re-recordable in 快捷鍵. (#22)
- **候選詞窗 can be turned off.** The switch in 一般 behaves as it does on macOS:
  nothing fetched, no window, and Return / Space / Tab / `[` / `]` commit the
  composition as typed. (#20)
- **Ctrl+← / Ctrl+→ move the caret inside the composition.** While composing,
  Ctrl with an arrow moves within the pending text; the candidate list is
  untouched. Idle, it stays the host application's word jump, and Ctrl+Shift /
  Alt / Win chords stay the host's. (#29, #31)
- **符號選單 on Ctrl+Alt+,.** The whole table on one chord, first key already a
  pick, bracket pairs inserted as pairs — the Mac's picker, same table file.
  `` ` `` still swaps 漢字 / 羅馬字. (#27, #28)

#### Candidates

- **⇧ + a selection key commits that candidate in the other script**, as on
  macOS. (#35)
- **The 字面羅馬字 cell no longer takes a selection key**; the keys start on the
  cell after it. (#25)

#### Appearance

- **A typeface of your own for the candidate window**, added in 字型管理 and
  listed under its file name. (#16, #37)
- **The typefaces Windows already has are in the list**, after the bundled five
  and the added files, with the same search field and ten-row pager as the Mac's
  pane. Importing a file whose face Windows already has selects that row and says
  so; a family installed or removed in Windows' font settings is picked up by the
  next candidate window. (#46)

#### Settings

- **快捷鍵 reads in typing order under 選字 / 拍字 / 其他**, matching the Mac's
  pane row for row. (#33, #36, #38)

#### Install

- **Upgrading clears the old typeface files.** The typefaces were renamed when
  they moved to a shared directory, and an installer only overwrites the names it
  ships — so upgrading from 3.6.7 or earlier left four dead copies, about 40 MB,
  in the install directory until an uninstall. The installer now sweeps them.
  (#34)

#### Fixes

- **Adding a typeface or a custom word in 設定 no longer closes the window.**
  The settings window aborted when a row was added to 字型管理 or 自訂詞庫: the
  list was told which row to select before it had the row. (#41)
- **Typing with a custom typeface no longer crashes the host application.** The
  font file was loaded through a DirectWrite factory that was released as soon
  as the file was read, so the first candidate drawn in that face took the host
  down with it. (#43)
- **Ctrl+Alt+, opens 符號選單.** The chord was never delivered while Alt was
  held; it is now registered as a preserved key, like the other Ctrl+Alt
  shortcuts. (#44)
