## v3.6.5

Headline release: **macOS joins iOS and Android** — a native Input Method Kit input method over the same Rust engine, released separately as a signed and notarized package. On mobile the release is a 方音 (TPS) and candidate-accuracy round: candidates never contain a syllable you did not type, the spacebar reaches the unmarked first and fourth tones, and keys that carry two glyphs resolve from the dictionary instead of guessing. Next-word learning converges on one platform-neutral contract and keys each reading of a 漢字 separately. Android now requires Android 11. Dictionary entries are unchanged from v3.6.4.

### Shared (iOS + Android)

#### Bug Fixes

- **Candidates never carry a syllable you did not type.** Continuous input hydrated every dictionary row whose key merely *started* with the typed body, so `tsuisi` surfaced 水社寮 `tsuí-siā-liâu` and `kesithau` surfaced 家私頭仔 `ke-si-thâu-á` — a syllable never typed at all. A per-syllable reach guard keeps a row only when its second-to-last syllable ends before the typed body does; exact key hits (阿姨仔 `a-î-á`) are untouched. An unfinished trailing syllable counts as reached, so `tai` drops 台語 and `taig` keeps it. (#566)
- **方音 spacebar reaches the unmarked first and fourth tones.** TPS writes tones 2/3/5/6/7/8/9 with standalone marks, but tones 1 and 4 write none, so their keys were byte-identical to the toneless and eighth-tone keys and the family could not express "fourth tone only". The pin is now resolved at lookup time: `ㄒㄧ` + space yields only 詩 / 司 / 絲 / 思 / 施, and `ㄐㄧㆵ` + space only 這 / 即 / 今. Typing without space still offers every tone. (#556)
- **方音 keys that carry two glyphs resolve from the dictionary.** The per-keystroke auto-correct picked one glyph for a dual-form key and overwrote the buffer, so toneless continuous input only ever saw the guess. Resolution moved into the index walk — the buffer keeps the user's literal text — and 考卷, 毋是, 會記得, 毋好, and 啥物 now type straight through. Single `ㄇ` yields 毋 and single `ㄫ` yields 黃 / 向 without a long press. (#554)
- **方音 nasal readings survive toneless continuous input.** The syllabic-nasal fold had exactly one trigger — a following tone mark — which toneless input never supplies, and a buffer-final nasal can never receive. Two further alternate-reading generators recover the stranded cases. (#553)
- **Next-word learning follows one contract on every platform.** Three learning rules branched on the platform identifier, so the same commit taught each platform something different; the divergence was frozen drift, not design. Word boundary is now whitespace only and never `-`, so `tâi-gí khí-puânn` learns as one 台語 → 齒盤 pair instead of being split three ways on one platform and four on the other. 方音 is one unit, since its space is the tone-1 syllable marker rather than a word boundary. (#530)

#### Changes

- **Each reading of a 漢字 learns separately as a previous word.** The 詞關聯 storage key widens to `(prev 漢字, prev TL, next 漢字, next TL)`, so 重/tîng → 複 and 重/tāng → 複 are two observations rather than one overwriting the other. A predicted word now receives at most one learning bonus instead of one per stored reading. Existing associations are preserved: the upgrade rebuilds the table under the wider key in a single transaction, copying every row. (#531)
- **方音 long-press offers the key's own glyph.** Long-pressing `ㄗ` now shows {ㄗ, ㄐ} rather than {ㄐ}, and `ㆠ` shows {ㆠ, 1}. Punctuation keys stay variant-only. (#557)

#### Refactoring

- Replaced the per-version 詞關聯 migration ladder on both platforms with one convergent rebuild. SQLite cannot `ALTER` a table-level `UNIQUE`, so widening the key means rebuilding regardless, and a rebuild that reads the columns actually present subsumes every intermediate step. This also repairs a pre-existing defect where an ancient Android database could be stamped current while missing the `prev_tl` column. (#531)

### iOS

#### Bug Fixes

- **The spacebar no longer dies after a cursor drag.** Long-pressing the spacebar and dragging to move the cursor left a stale drag offset that swallowed every later spacebar release, in every input mode, permanently. The dispatch layer now uses the keyboard framework's own drag state as the single source of truth and resets the offset when the gesture ends. (#545)

### Android

#### Changes

- **Android 11 (API 30) or newer is now required**, raised from Android 9 (API 28). Android's SQLite ships with the OS: API 28 and 29 bundle SQLite 3.22, which predates `UPSERT` (3.24). All three user-data databases — 詞關聯, 詞頻, 自訂詞 — use `UPSERT` at every write, and every write site catches and logs fire-and-forget with release logging compiled out, so on Android 9 and 10 learning silently never worked: typing was fine, nothing was ever saved, no crash and no log. API 30 bundles SQLite 3.28. Play Console at the time of the change: Android 9 at 3 installs (under 1%), Android 10 in the same band, against roughly 869 total. Devices below Android 11 keep v3.6.4 and stop receiving updates. (#532)
- Dependencies raised to the newest versions compatible with compile SDK 36, and the protobuf runtime bumped to pair with the committed generated code. (#546 / #516)

### macOS

**First public macOS release.** A native Input Method Kit input method sharing the Rust engine, the dictionary, and the learning databases' design with the mobile apps. Distributed as a Developer ID-signed, notarized package from the project website — not the Mac App Store — and it checks for updates on its own.

#### Input

- TL and POJ input with candidate selection, 漢羅 swapping, continuous input, next-word prediction, user frequency, and the custom dictionary.
- Full-width punctuation while output is 漢字, matching the Ministry of Education input method.
- A solo Shift tap toggles alphanumeric passthrough; a latched Caps Lock is left alone.
- Bare digits select candidates once no tone can follow, and `⇥` moves to the next candidate.
- `↓` enters candidate-selection mode, where bare digits always select; Space commits the other script.
- Automatic spacing after a committed word, with attaching punctuation swapping the space to its far side.

#### Candidate window

- Horizontal, vertical, and expandable layouts; light and dark following the system.
- 漢字 above its romanization in the row layouts, with window width growing for long candidates instead of truncating.
- Window size, text size, and candidate font are configurable; cells show the key that picks them.

#### Settings and menu

- A System Settings-style window with 一般, 外觀, 快捷鍵, and 詞庫 panes, reachable from one `⌃⌘S` doorway.
- Every composing action is bindable to a key of the user's choosing, with a per-pane restore-defaults control.
- Import and export the same `.taigi` backup format the mobile apps use.
- Menu-bar icon rendered as a template image, matching Apple's own CJK input methods.

#### Packaging

- Universal binary — Apple Silicon and Intel in one package.
- Notify-only update check against a manifest served from the project domain, so an update never interrupts what you are typing.

### Dictionary

#### Changes

- Regenerated all dictionary artifacts for the release. There are no `(漢字, TL)` entry additions or removals compared with v3.6.4; the rebuilt binaries differ only in their build-timestamp header field.
