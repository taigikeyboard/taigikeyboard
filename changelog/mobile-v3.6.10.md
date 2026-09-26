## v3.6.10

Mobile release for iOS and Android. Headline: **one-handed mode** from the
toolbar keyboard button, **custom themes with a photo background** (drag, pinch
to zoom, transparency), and the engine round the desktop train shipped in
3.6.9 — the keyboard **learns the phrases you build pick by pick**, the `-` /
`--` you type is honoured as typed, typed tone digits and capitals land where
they belong, and a **無連字符** switch drops dictionary hyphens. 漢字 is now the
default output. Under the hood, the four user-data stores (自訂詞庫, word
frequency, word association, learned phrases) and the `.taigi` backup now live
in the shared engine; the first launch takes over the existing files in place.
The number v3.6.9 was used by the desktop train only; mobile goes from v3.6.8
to v3.6.10.

### Shared (iOS + Android)

#### New

- **One-handed mode.** Long-press the keyboard button at the end of the toolbar
  for a callout: Hide keyboard · Left hand (倒手) · Right hand (正手). A side
  docks the key rows at 80% width against that edge; the gap holds a panel to
  switch side (換爿) or return to full width. The button's icon follows the last
  pick, so a tap hides the keyboard or toggles that side on and off. The mode is
  remembered. (#246)
- **Custom themes: one background, gradient direction, photo.** A custom theme
  has one background surface shared by the keyboard and the candidate bar:
  孤色 (solid), 漸層 (gradient) or 相片 (photo). Gradients take a direction — eight
  arrows, or drag across the preview. A photo is picked from the library
  (no permission prompt), stored downsampled on the device and excluded from
  OS backup; drag it to reposition, pinch to zoom up to 2×, and set its
  透明度 (transparency, default 0). A short 徙位 · 放大 hint shows until the first
  touch. The editor reads 背景 · 揤鈕介面 · 候選詞介面 · 恢復預設. (#90–#95, #207,
  #211, #221, #244)
- **Custom themes: one key colour.** The editor has a single 揤鈕色水 (Key Fill)
  row for every key, white by default. A saved theme whose function keys were a
  different colour now draws them in its letter-key colour. The theme card
  shows only the background, with the selection checkmark. (#209, #210, #132)
- **Learned phrases.** Composing a phrase from two or more picks — 記 → 起 → 來 on
  `kikhilai` — teaches the keyboard that whole phrase; the next `kikhilai`
  offers 記起來 as one candidate. Always on, with no list and no badge. Learned
  phrases live in their own store beside the frequency and association records
  (cap 2000; a 自訂詞庫 entry for the same pair always wins), are emptied with
  them by the reset that clears learning records, and are not part of the
  `.taigi` backup; the 自訂詞庫 is untouched. A
  learned phrase keeps the word boundaries of the commit: 做 → 進出口 on
  `tsotsintshutkhau` learns `tsò tsìn-tshut-kháu`, two words. (#109–#113,
  #125–#128, #145)
- **無連字符 (No Hyphens).** A switch in settings. On, `tâi-uân` renders `tâiuân`
  and the 輕聲 `--` becomes `·`: `hōo--guá` → `hōo·guá`. Only hyphens the
  dictionary or a custom entry supplies are dropped; what you type stays as
  typed. No effect under TPS. Default off. To type `·` yourself, long-press
  the `-` key, or take the first cell of the symbol keyboard's punctuation row.
  (#103, #108)
- **大本字時 ⁿ 轉做 ᴺ.** In capitals the nasal marker is written `ᴺ` (`SIÂᴺ`);
  switch it off for `SIÂⁿ`. On by default. (#134, #138)
- **About page.** Home → 關於齒盤 carries the desktop About text — an
  introduction in 漢字 with TL / POJ — plus website, GitHub, Discord and email
  links. (#213)

#### Bug Fixes

- **Separators are yours.** The `-` or `--` you type between two picks reaches
  the document as typed — `tng--lai`, 轉, 來 → `tńg--lâi`, no longer `tńg-lâi` —
  and the learned phrase remembers it. A typed `-` is a syllable boundary:
  `khi--ah` never reads 隙 `khiah`. The kind counts: `--` asks for a 輕聲 word
  (`hoo--gua` → 予我 `hōo--guá`), `-` for a plain one (`hoo-gua` → 予 + 我
  `hōo-guá`). The best reading keeps your separator (`goa--si` → `gô-á--sī`),
  and a separator typed inside a dictionary word holds: `pang-tang-lai` → 放重利
  writes `pàng-tāng-lāi`. (#129, #131, #133, #139)
- **Typed tone digits win.** `teng5sek` puts 程式 first instead of 等式 / 中式;
  `kokbin5tong2` finds 國民黨; `iah8` no longer trails the whole `ia` family.
  (#67, #71, #97)
- **Capitals survive.** Caps Lock `SIANN5` writes `SIÂᴺ` rather than `Siâⁿ`;
  Caps Lock raises every letter of `oo` / `ng` (`óo` → `ÓO`, with `OO` / `NG`
  in the long-press callouts); a 自訂詞 stored as `Keng-lâm Su-īⁿ` keeps its
  capitals when reached through `klsi`. (#89, #102, #138, #201)
- **Abbreviations look up the whole buffer.** `ss` reaches 鎖匙; aspirated
  initials count as one unit (`phthk` reaches a 披頭巾 custom word); words with
  no initial (`âng-enn-á`) abbreviate too. (#82)
- **Your homophone pick leads.** Picking 更新 for `kingsin` puts it first next
  time, even against far commoner dictionary words, and a repeatedly picked
  whole word beats its syllable split. (#69)
- **一个 / tsi̍t-ê shows again.** Classifier 个 words were wrongly filed as
  variants of 的 and hidden while 異用字 was off. (#73)
- **Typing after a pick follows the live settings.** Under 漢字 output the
  prefix after a pick no longer gains a stray space (`台 gih`), and 無連字符 no
  longer re-hyphenates it until the next commit. (#137)
- **Custom themes stay light in dark mode.** A custom theme draws the same in
  light and dark: long-press callouts, candidate highlights, separators and the
  emoji panel no longer turn dark, and the emoji panel takes the theme's
  background. Built-in themes still follow the system. (#240, #242)

#### Changes

- **漢字 is the default output.** A fresh install — and anyone who never
  pressed 文/A — gets 漢字 as the candidate with 羅馬字 underneath. A stored
  choice is kept. (#86)
- **Frequency and association recording is always on.** The 詞頻紀錄 and
  詞關聯紀錄 pages and their switches are gone from the dictionary tab. Anyone
  who had switched recording off before the update is recorded again; the
  reset that clears learning records and the `.taigi` backup still cover both. (#130)
- **User data moved into the shared engine.** 自訂詞庫, frequency, association
  and learned phrases are read and written by the engine on every platform, and
  `.taigi` export / restore go through one codec. On the first launch after the
  update the engine takes over the existing files in place (a one-time copy is
  kept beside them) and rebuilds the 自訂詞庫 search keys once. Existing backups
  restore as before. (#219–#237)
- **Home tab.** The in-app version history and the 網站紹介 link are removed; the
  About page replaces 關於開發者. (#101, #213, #216)
- **Labels.** 顯示當咧拍的字 → 當咧拍的字囥第一个; 自動空白 → 自動閬一格; theme
  editor 純色 → 孤色, 淡化 → 透明度, 照片 → 相片, gradient rows 開始 / 結束. Tâi-lô and
  POJ strings start each sentence with a capital, and help texts are shorter.
  (#75, #106, #114, #206, #212, #217, #243)

#### Improvements

- **Theme photos decode off the main thread**, with a small thumbnail for the
  theme cards and the full image only for the keyboard. (#211)

### iOS

#### Bug Fixes

- The theme page background matches the other grouped tabs. (#135)

### Android

#### Changes

- **Android 9 and 10 can install again.** The minimum is lowered from Android 11
  to Android 9; user-data SQL now runs on the engine's bundled SQLite, not the
  system one. (#96, #231)

#### Bug Fixes

- **Next-word predictions after a supplementary-plane 漢字.** Committing 𣍐 or 𫔘
  showed no bundled prediction because the lookup key was half a character; it
  is now the whole character (𣍐 → 使). (#195)

### Dictionary

#### Changes

- Abbreviation keys are rebuilt per leading spelling unit. (#82)
- Classifier 个 / ê words are no longer marked as variants of 的. (#73)
- Dictionary artifacts regenerated for the release.
